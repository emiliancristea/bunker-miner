// BUNKER POOL - Miner Connection Handler
// High-performance TCP connection management for thousands of concurrent miners

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::protocol::{
    Algorithm, MinerSession, StratumError, StratumMessage, StratumMethod, StratumNotification,
    StratumRequest, StratumResponse, parse_stratum_message, MiningJob, ShareSubmission,
    MiningSubscription,
};

/// Configuration for connection handling
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub ping_interval: Duration,
    pub max_idle_time: Duration,
    pub max_message_size: usize,
    pub default_difficulty: f64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(10),
            ping_interval: Duration::from_secs(60),
            max_idle_time: Duration::from_secs(300), // 5 minutes
            max_message_size: 8192, // 8KB max message size
            default_difficulty: 1.0,
        }
    }
}

/// Miner connection handler
pub struct MinerConnection {
    pub session: Arc<RwLock<MinerSession>>,
    stream: TcpStream,
    addr: SocketAddr,
    config: ConnectionConfig,
    job_sender: mpsc::Sender<MiningJob>,
    share_sender: mpsc::Sender<(Uuid, ShareSubmission)>,
    disconnect_sender: mpsc::Sender<Uuid>,
}

impl MinerConnection {
    pub fn new(
        stream: TcpStream,
        addr: SocketAddr,
        algorithm: Algorithm,
        config: ConnectionConfig,
        job_sender: mpsc::Sender<MiningJob>,
        share_sender: mpsc::Sender<(Uuid, ShareSubmission)>,
        disconnect_sender: mpsc::Sender<Uuid>,
    ) -> Self {
        let session = Arc::new(RwLock::new(MinerSession::new(
            addr.ip().to_string(),
            algorithm,
        )));

        Self {
            session,
            stream,
            addr,
            config,
            job_sender,
            share_sender,
            disconnect_sender,
        }
    }

    /// Main connection handling loop
    pub async fn handle(mut self) {
        let session_id = {
            let session = self.session.read().await;
            session.id
        };

        info!("New miner connection: {} ({})", session_id, self.addr);

        let (reader, mut writer) = self.stream.split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        // Job notification receiver
        let (job_tx, mut job_rx) = mpsc::channel::<MiningJob>(100);
        let job_sender_clone = self.job_sender.clone();

        // Spawn job notification handler
        let session_clone = Arc::clone(&self.session);
        let writer_clone = writer.try_clone();
        tokio::spawn(async move {
            if let Ok(mut writer) = writer_clone {
                while let Some(job) = job_rx.recv().await {
                    let session = session_clone.read().await;
                    if session.subscribed && session.authorized {
                        let notification = StratumNotification::mining_notify(&job);
                        if let Ok(message) = notification.to_json() {
                            let line = format!("{}\n", message);
                            if let Err(e) = writer.write_all(line.as_bytes()).await {
                                error!("Failed to send job notification: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });

        let mut last_activity = Instant::now();
        let mut ping_timer = tokio::time::interval(self.config.ping_interval);

        loop {
            tokio::select! {
                // Read incoming messages
                result = timeout(self.config.read_timeout, reader.read_line(&mut line)) => {
                    match result {
                        Ok(Ok(0)) => {
                            // Connection closed
                            debug!("Connection closed by miner: {}", session_id);
                            break;
                        }
                        Ok(Ok(_)) => {
                            last_activity = Instant::now();
                            
                            // Remove newline
                            if line.ends_with('\n') {
                                line.pop();
                            }
                            if line.ends_with('\r') {
                                line.pop();
                            }

                            // Check message size
                            if line.len() > self.config.max_message_size {
                                warn!("Message too large from {}: {} bytes", self.addr, line.len());
                                let error_response = StratumResponse::error(
                                    None,
                                    StratumError::invalid_request()
                                );
                                if let Ok(response_json) = error_response.to_json() {
                                    let _ = self.send_message(&mut writer, &response_json).await;
                                }
                                line.clear();
                                continue;
                            }

                            // Process the message
                            if let Err(e) = self.process_message(&mut writer, &line).await {
                                error!("Error processing message from {}: {}", self.addr, e);
                            }
                            
                            line.clear();
                        }
                        Ok(Err(e)) => {
                            error!("Read error from {}: {}", self.addr, e);
                            break;
                        }
                        Err(_) => {
                            // Timeout
                            if last_activity.elapsed() > self.config.max_idle_time {
                                warn!("Connection timeout for {}: no activity for {:?}", self.addr, last_activity.elapsed());
                                break;
                            }
                        }
                    }
                }

                // Send periodic pings
                _ = ping_timer.tick() => {
                    // We don't send actual pings in Stratum, but we can check if connection is still alive
                    if last_activity.elapsed() > self.config.max_idle_time {
                        warn!("Connection idle timeout for {}", self.addr);
                        break;
                    }
                }

                // Receive job notifications
                Some(job) = job_rx.recv() => {
                    // Job will be sent by the spawned task above
                    let _ = job_tx.send(job).await;
                }
            }
        }

        // Clean up
        info!("Closing connection for miner: {} ({})", session_id, self.addr);
        let _ = self.disconnect_sender.send(session_id).await;
        let _ = writer.shutdown().await;
    }

    /// Process incoming Stratum message
    async fn process_message(
        &mut self,
        writer: &mut tokio::io::WriteHalf<TcpStream>,
        line: &str,
    ) -> Result<(), anyhow::Error> {
        let message = parse_stratum_message(line)?;

        match message {
            StratumMessage::Request(request) => {
                self.handle_request(writer, request).await?;
            }
            StratumMessage::Response(_) => {
                // Miners typically don't send responses, but we handle them gracefully
                debug!("Received response from miner: {}", line);
            }
            StratumMessage::Notification(_) => {
                // Miners typically don't send notifications, but we handle them gracefully
                debug!("Received notification from miner: {}", line);
            }
        }

        // Update session activity
        {
            let mut session = self.session.write().await;
            session.update_activity();
        }

        Ok(())
    }

    /// Handle Stratum request from miner
    async fn handle_request(
        &mut self,
        writer: &mut tokio::io::WriteHalf<TcpStream>,
        request: StratumRequest,
    ) -> Result<(), anyhow::Error> {
        let method: StratumMethod = request.method.as_str().into();
        let response = match method {
            StratumMethod::Subscribe => self.handle_subscribe(request).await,
            StratumMethod::Authorize => self.handle_authorize(request).await,
            StratumMethod::Submit => self.handle_submit(request).await,
            StratumMethod::Configure => self.handle_configure(request).await,
            _ => StratumResponse::error(request.id, StratumError::method_not_found()),
        };

        let response_json = response.to_json()?;
        self.send_message(writer, &response_json).await?;

        Ok(())
    }

    /// Handle mining.subscribe request
    async fn handle_subscribe(&mut self, request: StratumRequest) -> StratumResponse {
        let mut session = self.session.write().await;
        
        if session.subscribed {
            return StratumResponse::error(
                request.id,
                StratumError::new(20, "Already subscribed"),
            );
        }

        // Generate subscription details
        let subscription_id = format!("sub_{}", Uuid::new_v4().simple());
        let extranonce1 = format!("{:08x}", session.id.as_u128() & 0xffffffff);
        let extranonce2_size = 4; // Standard 4-byte extranonce2

        let subscription = MiningSubscription {
            subscription_id: subscription_id.clone(),
            extranonce1: extranonce1.clone(),
            extranonce2_size,
        };

        session.subscription = Some(subscription);
        session.subscribed = true;

        // Extract user agent if provided
        if let Some(params) = &request.params {
            if let Some(user_agent) = params.as_array().and_then(|a| a.get(0)).and_then(|v| v.as_str()) {
                session.user_agent = Some(user_agent.to_string());
            }
        }

        info!("Miner subscribed: {} from {}", session.id, self.addr);

        let result = serde_json::json!([
            [["mining.set_difficulty", subscription_id], ["mining.notify", subscription_id]],
            extranonce1,
            extranonce2_size
        ]);

        StratumResponse::success(request.id, result)
    }

    /// Handle mining.authorize request
    async fn handle_authorize(&mut self, request: StratumRequest) -> StratumResponse {
        let mut session = self.session.write().await;

        if !session.subscribed {
            return StratumResponse::error(request.id, StratumError::not_subscribed());
        }

        if session.authorized {
            return StratumResponse::error(
                request.id,
                StratumError::new(20, "Already authorized"),
            );
        }

        // Extract worker name and password
        let params = match &request.params {
            Some(params) => params.as_array(),
            None => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        let worker_name = match params.and_then(|p| p.get(0)).and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        // For now, we accept any worker name (in production, validate against database)
        session.authorized = true;
        session.worker_name = Some(worker_name.to_string());

        info!("Miner authorized: {} worker: {} from {}", session.id, worker_name, self.addr);

        StratumResponse::success(request.id, serde_json::json!(true))
    }

    /// Handle mining.submit request
    async fn handle_submit(&mut self, request: StratumRequest) -> StratumResponse {
        let session = self.session.read().await;

        if !session.subscribed || !session.authorized {
            return StratumResponse::error(request.id, StratumError::unauthorized_worker());
        }

        // Parse share submission
        let params = match &request.params {
            Some(params) => params.as_array(),
            None => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        let params = match params {
            Some(p) if p.len() >= 5 => p,
            _ => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        let worker_name = match params[0].as_str() {
            Some(name) => name.to_string(),
            None => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        let job_id = match params[1].as_str() {
            Some(id) => id.to_string(),
            None => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        let extranonce2 = match params[2].as_str() {
            Some(nonce) => nonce.to_string(),
            None => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        let ntime = match params[3].as_str() {
            Some(time) => time.to_string(),
            None => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        let nonce = match params[4].as_str() {
            Some(nonce) => nonce.to_string(),
            None => return StratumResponse::error(request.id, StratumError::invalid_params()),
        };

        let version_bits = params.get(5).and_then(|v| v.as_str()).map(String::from);

        let share = ShareSubmission {
            worker_name,
            job_id,
            extranonce2,
            ntime,
            nonce,
            version_bits,
        };

        // Forward share to share processor
        if let Err(e) = self.share_sender.send((session.id, share)).await {
            error!("Failed to forward share: {}", e);
            return StratumResponse::error(
                request.id,
                StratumError::new(20, "Internal server error"),
            );
        }

        debug!("Share submitted by {}: {}", session.id, session.worker_name.as_deref().unwrap_or("unknown"));

        // For now, accept all shares (share processor will validate)
        StratumResponse::success(request.id, serde_json::json!(true))
    }

    /// Handle mining.configure request (optional)
    async fn handle_configure(&mut self, request: StratumRequest) -> StratumResponse {
        // mining.configure is optional and used for version rolling, etc.
        // For now, we just return an empty result
        StratumResponse::success(request.id, serde_json::json!({}))
    }

    /// Send message to miner
    async fn send_message(
        &self,
        writer: &mut tokio::io::WriteHalf<TcpStream>,
        message: &str,
    ) -> Result<(), anyhow::Error> {
        let line = format!("{}\n", message);
        
        match timeout(self.config.write_timeout, writer.write_all(line.as_bytes())).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(anyhow::anyhow!("Write timeout")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    
    #[tokio::test]
    async fn test_connection_config_defaults() {
        let config = ConnectionConfig::default();
        assert_eq!(config.read_timeout, Duration::from_secs(30));
        assert_eq!(config.default_difficulty, 1.0);
    }
}