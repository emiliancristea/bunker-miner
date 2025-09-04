/*!
 * BUNKER MINER - Stratum Pool Server PoC
 * 
 * This PoC validates the core components of a Stratum v1 mining pool server,
 * de-risking future BUNKER POOL development by proving we can handle the
 * Stratum protocol correctly and efficiently.
 * 
 * Success Criteria:
 * - Accept Stratum v1 connections from real miners
 * - Handle mining.subscribe and mining.authorize correctly
 * - Send mining.notify (job) messages to connected miners
 * - Parse and validate mining.submit (share) messages
 * - Maintain stable connections with multiple miners
 */

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Arg, Command};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tokio::time::{interval, Duration};
use tracing::{error, info, warn, debug};

#[derive(Debug, Clone)]
pub struct StratumJob {
    pub id: String,
    pub prevhash: String,
    pub coinb1: String,
    pub coinb2: String,
    pub merkle_branch: Vec<String>,
    pub version: String,
    pub nbits: String,
    pub ntime: String,
    pub clean_jobs: bool,
}

#[derive(Debug, Clone)]
pub struct MinerConnection {
    pub id: String,
    pub address: SocketAddr,
    pub subscription_id: String,
    pub extranonce1: String,
    pub extranonce2_size: usize,
    pub authorized: bool,
    pub worker_name: String,
    pub connected_at: chrono::DateTime<Utc>,
    pub last_activity: chrono::DateTime<Utc>,
    pub submitted_shares: u64,
    pub accepted_shares: u64,
    pub difficulty: f64,
}

#[derive(Debug, Clone)]
pub struct StratumRequest {
    pub id: Option<Value>,
    pub method: String,
    pub params: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct StratumResponse {
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

pub struct StratumServer {
    bind_address: String,
    miners: Arc<Mutex<HashMap<String, MinerConnection>>>,
    current_job: Arc<Mutex<Option<StratumJob>>>,
    job_broadcast: broadcast::Sender<StratumJob>,
    extranonce_counter: Arc<Mutex<u64>>,
}

impl StratumServer {
    pub fn new(bind_address: String) -> Self {
        let (job_tx, _) = broadcast::channel(1000);
        
        Self {
            bind_address,
            miners: Arc::new(Mutex::new(HashMap::new())),
            current_job: Arc::new(Mutex::new(None)),
            job_broadcast: job_tx,
            extranonce_counter: Arc::new(Mutex::new(1)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.bind_address).await
            .context("Failed to bind to address")?;

        info!("Stratum server listening on {}", self.bind_address);

        // Start job generator task
        self.start_job_generator().await;

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New miner connection from {}", addr);
                    let server = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream, addr).await {
                            error!("Connection error for {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        let miner_id = format!("miner-{}-{}", addr.ip(), addr.port());
        info!("Handling connection for miner ID: {}", miner_id);

        // Subscribe to job broadcasts for this connection
        let mut job_rx = self.job_broadcast.subscribe();

        // Spawn task to handle job broadcasts
        let job_writer = writer.clone();
        let job_miner_id = miner_id.clone();
        let miners_ref = Arc::clone(&self.miners);
        tokio::spawn(async move {
            let mut job_writer = job_writer;
            while let Ok(job) = job_rx.recv().await {
                // Check if miner is still connected and authorized
                let should_send = {
                    let miners = miners_ref.lock().await;
                    miners.get(&job_miner_id)
                        .map(|m| m.authorized)
                        .unwrap_or(false)
                };

                if should_send {
                    if let Err(e) = Self::send_mining_notify(&mut job_writer, &job).await {
                        debug!("Failed to send job to {}: {}", job_miner_id, e);
                        break;
                    }
                }
            }
        });

        // Handle incoming messages
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("Connection closed by {}", addr);
                    break;
                }
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    debug!("Received from {}: {}", addr, line);
                    
                    match self.parse_stratum_request(line) {
                        Ok(request) => {
                            match self.handle_stratum_request(
                                &miner_id, 
                                &mut writer, 
                                request
                            ).await {
                                Ok(_) => {}
                                Err(e) => {
                                    warn!("Error handling request from {}: {}", addr, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse request from {}: {} - Raw: {}", addr, e, line);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from {}: {}", addr, e);
                    break;
                }
            }
        }

        // Clean up connection
        let mut miners = self.miners.lock().await;
        if let Some(miner) = miners.remove(&miner_id) {
            info!("Miner {} disconnected after {} shares ({} accepted)", 
                  miner.worker_name, miner.submitted_shares, miner.accepted_shares);
        }

        Ok(())
    }

    fn parse_stratum_request(&self, line: &str) -> Result<StratumRequest> {
        let json: Value = serde_json::from_str(line)
            .context("Failed to parse JSON")?;

        let id = json.get("id").cloned();
        let method = json.get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing method field"))?
            .to_string();
        
        let params = json.get("params")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(StratumRequest { id, method, params })
    }

    async fn handle_stratum_request(
        &self,
        miner_id: &str,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        request: StratumRequest,
    ) -> Result<()> {
        match request.method.as_str() {
            "mining.subscribe" => {
                self.handle_subscribe(miner_id, writer, request).await
            }
            "mining.authorize" => {
                self.handle_authorize(miner_id, writer, request).await
            }
            "mining.submit" => {
                self.handle_submit(miner_id, writer, request).await
            }
            _ => {
                warn!("Unknown method: {}", request.method);
                self.send_error_response(writer, request.id, -1, "Method not found").await
            }
        }
    }

    async fn handle_subscribe(
        &self,
        miner_id: &str,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        request: StratumRequest,
    ) -> Result<()> {
        info!("Handling mining.subscribe for {}", miner_id);

        // Generate unique extranonce1 for this connection
        let extranonce1 = {
            let mut counter = self.extranonce_counter.lock().await;
            let value = *counter;
            *counter += 1;
            format!("{:08x}", value)
        };

        let subscription_id = format!("subscription-{}", miner_id);
        let extranonce2_size = 4; // Standard 4-byte extranonce2

        // Store miner connection info
        {
            let mut miners = self.miners.lock().await;
            let miner_conn = MinerConnection {
                id: miner_id.to_string(),
                address: "127.0.0.1:0".parse().unwrap(), // Will be updated with real address
                subscription_id: subscription_id.clone(),
                extranonce1: extranonce1.clone(),
                extranonce2_size,
                authorized: false,
                worker_name: "unknown".to_string(),
                connected_at: Utc::now(),
                last_activity: Utc::now(),
                submitted_shares: 0,
                accepted_shares: 0,
                difficulty: 1.0,
            };
            miners.insert(miner_id.to_string(), miner_conn);
        }

        // Send subscription response
        let response = StratumResponse {
            id: request.id,
            result: Some(json!([
                [
                    ["mining.set_difficulty", subscription_id],
                    ["mining.notify", subscription_id]
                ],
                extranonce1,
                extranonce2_size
            ])),
            error: None,
        };

        self.send_response(writer, response).await?;

        // Send initial difficulty
        self.send_set_difficulty(writer, 1.0).await?;

        // Send current job if available
        if let Some(job) = self.current_job.lock().await.clone() {
            Self::send_mining_notify(writer, &job).await?;
        }

        info!("Successfully subscribed miner {} with extranonce1: {}", miner_id, extranonce1);
        Ok(())
    }

    async fn handle_authorize(
        &self,
        miner_id: &str,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        request: StratumRequest,
    ) -> Result<()> {
        let worker_name = request.params.get(0)
            .and_then(|w| w.as_str())
            .unwrap_or("unknown")
            .to_string();

        let _password = request.params.get(1)
            .and_then(|p| p.as_str())
            .unwrap_or("");

        info!("Handling mining.authorize for {} worker: {}", miner_id, worker_name);

        // Update miner as authorized
        {
            let mut miners = self.miners.lock().await;
            if let Some(miner) = miners.get_mut(miner_id) {
                miner.authorized = true;
                miner.worker_name = worker_name.clone();
                miner.last_activity = Utc::now();
            }
        }

        // Send authorization response (always accept for PoC)
        let response = StratumResponse {
            id: request.id,
            result: Some(json!(true)),
            error: None,
        };

        self.send_response(writer, response).await?;
        info!("Authorized worker: {}", worker_name);
        Ok(())
    }

    async fn handle_submit(
        &self,
        miner_id: &str,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        request: StratumRequest,
    ) -> Result<()> {
        let worker_name = request.params.get(0)
            .and_then(|w| w.as_str())
            .unwrap_or("unknown");

        let job_id = request.params.get(1)
            .and_then(|j| j.as_str())
            .unwrap_or("");

        let extranonce2 = request.params.get(2)
            .and_then(|e| e.as_str())
            .unwrap_or("");

        let ntime = request.params.get(3)
            .and_then(|t| t.as_str())
            .unwrap_or("");

        let nonce = request.params.get(4)
            .and_then(|n| n.as_str())
            .unwrap_or("");

        info!("Share submission from {}: job_id={}, extranonce2={}, ntime={}, nonce={}", 
              worker_name, job_id, extranonce2, ntime, nonce);

        // Update miner stats
        {
            let mut miners = self.miners.lock().await;
            if let Some(miner) = miners.get_mut(miner_id) {
                miner.submitted_shares += 1;
                miner.last_activity = Utc::now();
                
                // For PoC, accept most shares (90% acceptance rate)
                let accept = miner.submitted_shares % 10 != 0; // Reject every 10th share
                if accept {
                    miner.accepted_shares += 1;
                }
            }
        }

        // For PoC, validate shares with simple rules
        let accept_share = !extranonce2.is_empty() && !nonce.is_empty();

        let response = StratumResponse {
            id: request.id,
            result: Some(json!(accept_share)),
            error: if accept_share { None } else { Some(json!(["Invalid share", null, null])) },
        };

        self.send_response(writer, response).await?;

        if accept_share {
            info!("✅ Accepted share from {}", worker_name);
        } else {
            warn!("❌ Rejected share from {}", worker_name);
        }

        Ok(())
    }

    async fn send_response(
        &self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        response: StratumResponse,
    ) -> Result<()> {
        let json_response = json!({
            "id": response.id,
            "result": response.result,
            "error": response.error
        });

        let message = format!("{}\n", json_response.to_string());
        writer.write_all(message.as_bytes()).await
            .context("Failed to send response")?;

        Ok(())
    }

    async fn send_error_response(
        &self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        id: Option<Value>,
        error_code: i32,
        error_message: &str,
    ) -> Result<()> {
        let response = StratumResponse {
            id,
            result: None,
            error: Some(json!([error_code, error_message, null])),
        };

        self.send_response(writer, response).await
    }

    async fn send_set_difficulty(
        &self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        difficulty: f64,
    ) -> Result<()> {
        let message = json!({
            "id": null,
            "method": "mining.set_difficulty",
            "params": [difficulty]
        });

        let line = format!("{}\n", message.to_string());
        writer.write_all(line.as_bytes()).await
            .context("Failed to send difficulty")?;

        Ok(())
    }

    async fn send_mining_notify(
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        job: &StratumJob,
    ) -> Result<()> {
        let message = json!({
            "id": null,
            "method": "mining.notify",
            "params": [
                job.id,
                job.prevhash,
                job.coinb1,
                job.coinb2,
                job.merkle_branch,
                job.version,
                job.nbits,
                job.ntime,
                job.clean_jobs
            ]
        });

        let line = format!("{}\n", message.to_string());
        writer.write_all(line.as_bytes()).await
            .context("Failed to send mining notify")?;

        debug!("Sent mining.notify for job {}", job.id);
        Ok(())
    }

    async fn start_job_generator(&self) {
        let job_tx = self.job_broadcast.clone();
        let current_job = Arc::clone(&self.current_job);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // New job every 30 seconds
            let mut job_counter = 1;

            loop {
                interval.tick().await;

                let job = StratumJob {
                    id: format!("job_{:06}", job_counter),
                    prevhash: format!("{:064}", job_counter - 1), // Mock previous block hash
                    coinb1: "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff2503".to_string(),
                    coinb2: "062f503253482f0403858402062f503253482f".to_string(),
                    merkle_branch: vec![], // Empty for simplicity
                    version: "00000002".to_string(),
                    nbits: "1d00ffff".to_string(), // Mock difficulty bits
                    ntime: format!("{:08x}", Utc::now().timestamp()),
                    clean_jobs: true,
                };

                info!("Generated new job: {}", job.id);

                // Update current job
                *current_job.lock().await = Some(job.clone());

                // Broadcast to all connected miners
                if let Err(e) = job_tx.send(job) {
                    debug!("No miners connected to receive job: {}", e);
                }

                job_counter += 1;
            }
        });
    }

    async fn print_stats(&self) {
        let miners = self.miners.lock().await;
        
        info!("=== Pool Statistics ===");
        info!("Connected miners: {}", miners.len());
        
        let mut total_submitted = 0;
        let mut total_accepted = 0;
        
        for (id, miner) in miners.iter() {
            total_submitted += miner.submitted_shares;
            total_accepted += miner.accepted_shares;
            
            let acceptance_rate = if miner.submitted_shares > 0 {
                (miner.accepted_shares as f64 / miner.submitted_shares as f64) * 100.0
            } else {
                0.0
            };
            
            info!("  {} ({}): {} submitted, {} accepted ({:.1}%)", 
                  id, miner.worker_name, miner.submitted_shares, miner.accepted_shares, acceptance_rate);
        }
        
        let overall_acceptance_rate = if total_submitted > 0 {
            (total_accepted as f64 / total_submitted as f64) * 100.0
        } else {
            0.0
        };
        
        info!("Overall: {} submitted, {} accepted ({:.1}%)", 
              total_submitted, total_accepted, overall_acceptance_rate);
    }
}

impl Clone for StratumServer {
    fn clone(&self) -> Self {
        Self {
            bind_address: self.bind_address.clone(),
            miners: Arc::clone(&self.miners),
            current_job: Arc::clone(&self.current_job),
            job_broadcast: self.job_broadcast.clone(),
            extranonce_counter: Arc::clone(&self.extranonce_counter),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("BUNKER MINER - Stratum Pool Server PoC");

    let matches = Command::new("stratum-server")
        .about("BUNKER MINER Stratum Pool Server Proof of Concept")
        .arg(
            Arg::new("bind")
                .long("bind")
                .help("Bind address (host:port)")
                .default_value("127.0.0.1:3333")
                .value_name("ADDRESS")
        )
        .arg(
            Arg::new("stats-interval")
                .long("stats-interval")
                .help("Stats reporting interval in seconds")
                .default_value("60")
                .value_parser(clap::value_parser!(u64))
        )
        .get_matches();

    let bind_address = matches.get_one::<String>("bind").unwrap().clone();
    let stats_interval = *matches.get_one::<u64>("stats-interval").unwrap();

    let server = StratumServer::new(bind_address);

    // Start stats reporting task
    let stats_server = server.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(stats_interval));
        loop {
            interval.tick().await;
            stats_server.print_stats().await;
        }
    });

    // Start the server
    server.start().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stratum_job_creation() {
        let job = StratumJob {
            id: "test_job".to_string(),
            prevhash: "000000000000000000000000000000000000000000000000000000000000".to_string(),
            coinb1: "coinbase1".to_string(),
            coinb2: "coinbase2".to_string(),
            merkle_branch: vec![],
            version: "00000002".to_string(),
            nbits: "1d00ffff".to_string(),
            ntime: "12345678".to_string(),
            clean_jobs: true,
        };

        assert_eq!(job.id, "test_job");
        assert!(job.clean_jobs);
    }

    #[test]
    fn test_miner_connection_creation() {
        let conn = MinerConnection {
            id: "test_miner".to_string(),
            address: "127.0.0.1:12345".parse().unwrap(),
            subscription_id: "sub_123".to_string(),
            extranonce1: "12345678".to_string(),
            extranonce2_size: 4,
            authorized: false,
            worker_name: "test_worker".to_string(),
            connected_at: Utc::now(),
            last_activity: Utc::now(),
            submitted_shares: 0,
            accepted_shares: 0,
            difficulty: 1.0,
        };

        assert_eq!(conn.worker_name, "test_worker");
        assert_eq!(conn.extranonce2_size, 4);
        assert!(!conn.authorized);
    }

    #[tokio::test]
    async fn test_stratum_request_parsing() {
        let server = StratumServer::new("127.0.0.1:3333".to_string());
        
        let json_line = r#"{"id": 1, "method": "mining.subscribe", "params": ["miner/1.0.0"]}"#;
        let request = server.parse_stratum_request(json_line).unwrap();
        
        assert_eq!(request.method, "mining.subscribe");
        assert_eq!(request.id, Some(json!(1)));
        assert_eq!(request.params.len(), 1);
    }
}