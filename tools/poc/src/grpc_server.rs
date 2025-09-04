/*!
 * BUNKER MINER - gRPC Server PoC
 * 
 * This PoC validates gRPC as the definitive communication layer between
 * the Rust daemon and C++ client, testing performance, type safety, and
 * streaming capabilities.
 * 
 * Success Criteria:
 * - gRPC server accepts connections from C++ client
 * - All RPC methods work correctly (ping, system info, streaming)
 * - Benchmark latency and throughput performance
 * - Validate Protocol Buffer type safety and serialization
 */

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Arg, Command};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tokio::time::{interval, Instant};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};
use tracing::{error, info, warn, debug};

// Include the generated protobuf code
pub mod daemon {
    tonic::include_proto!("bunker.daemon");
}

use daemon::{
    daemon_service_server::{DaemonService, DaemonServiceServer},
    *,
};

// Import our hardware detection from the PoC
use bunker_poc::hardware_detection::{detect_cpu_info, detect_nvidia_gpus, detect_system_memory};

#[derive(Debug)]
pub struct DaemonServiceImpl {
    start_time: Instant,
    version: String,
    mining_stats_tx: broadcast::Sender<MiningStatsUpdate>,
    client_connections: Arc<Mutex<HashMap<String, ClientConnection>>>,
}

#[derive(Debug, Clone)]
struct ClientConnection {
    id: String,
    connected_at: chrono::DateTime<Utc>,
    last_ping: chrono::DateTime<Utc>,
    requests_count: u64,
}

impl DaemonServiceImpl {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(1000);
        
        Self {
            start_time: Instant::now(),
            version: "0.1.0-poc".to_string(),
            mining_stats_tx: tx,
            client_connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn register_client(&self, client_id: String) {
        let mut connections = self.client_connections.lock().await;
        let connection = ClientConnection {
            id: client_id.clone(),
            connected_at: Utc::now(),
            last_ping: Utc::now(),
            requests_count: 0,
        };
        connections.insert(client_id, connection);
    }

    async fn update_client_activity(&self, client_id: String) {
        let mut connections = self.client_connections.lock().await;
        if let Some(conn) = connections.get_mut(&client_id) {
            conn.last_ping = Utc::now();
            conn.requests_count += 1;
        }
    }

    fn get_client_id_from_request<T>(request: &Request<T>) -> String {
        // In a real implementation, this would extract client ID from metadata
        // For PoC, we'll use the remote address if available
        request
            .remote_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| format!("unknown-{}", Utc::now().timestamp_nanos()))
    }
}

#[tonic::async_trait]
impl DaemonService for DaemonServiceImpl {
    async fn ping(&self, request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        let client_id = Self::get_client_id_from_request(&request);
        self.register_client(client_id.clone()).await;
        self.update_client_activity(client_id).await;

        let req = request.into_inner();
        let server_timestamp = Utc::now().timestamp_millis();
        
        info!("Received ping: '{}' at {}", req.message, req.timestamp);
        
        let response = PingResponse {
            echo_message: format!("Echo: {}", req.message),
            server_timestamp,
            client_timestamp: req.timestamp,
            server_version: self.version.clone(),
        };
        
        debug!("Sending ping response with server time: {}", server_timestamp);
        Ok(Response::new(response))
    }

    async fn get_system_info(
        &self,
        request: Request<SystemInfoRequest>,
    ) -> Result<Response<SystemInfoResponse>, Status> {
        let client_id = Self::get_client_id_from_request(&request);
        self.update_client_activity(client_id).await;

        let req = request.into_inner();
        info!("System info requested - GPU details: {}, CPU details: {}", 
              req.include_gpu_details, req.include_cpu_details);

        let mut response = SystemInfoResponse {
            timestamp: Utc::now().timestamp_millis(),
            gpus: vec![],
            cpu: None,
            total_memory_gb: 0,
            available_memory_gb: 0,
            platform: std::env::consts::OS.to_string(),
            daemon_version: self.version.clone(),
        };

        // Get GPU information if requested
        if req.include_gpu_details {
            match detect_nvidia_gpus() {
                Ok(gpu_infos) => {
                    for gpu_info in gpu_infos {
                        let grpc_gpu = GpuInfo {
                            index: gpu_info.index,
                            name: gpu_info.name,
                            uuid: gpu_info.uuid,
                            memory_total_mb: gpu_info.memory_total_mb,
                            memory_free_mb: gpu_info.memory_free_mb,
                            driver_version: gpu_info.driver_version,
                            temperature_c: gpu_info.temperature_c,
                            power_usage_w: gpu_info.power_usage_w,
                            gpu_utilization: gpu_info.gpu_utilization,
                            memory_utilization: gpu_info.memory_utilization,
                            core_clock_mhz: gpu_info.core_clock_mhz,
                            memory_clock_mhz: gpu_info.memory_clock_mhz,
                        };
                        response.gpus.push(grpc_gpu);
                    }
                }
                Err(e) => {
                    warn!("Failed to detect GPUs: {}", e);
                }
            }
        }

        // Get CPU information if requested
        if req.include_cpu_details {
            match detect_cpu_info() {
                Ok(cpu_info) => {
                    response.cpu = Some(CpuInfo {
                        name: cpu_info.name,
                        vendor: cpu_info.vendor,
                        cores_physical: cpu_info.cores_physical as u32,
                        cores_logical: cpu_info.cores_logical as u32,
                        frequency_mhz: cpu_info.frequency_mhz,
                        usage_percent: cpu_info.usage_percent,
                    });
                }
                Err(e) => {
                    warn!("Failed to detect CPU: {}", e);
                }
            }
        }

        // Get memory information
        match detect_system_memory() {
            Ok((total, available)) => {
                response.total_memory_gb = total;
                response.available_memory_gb = available;
            }
            Err(e) => {
                warn!("Failed to detect memory: {}", e);
            }
        }

        info!("Returning system info: {} GPUs, platform: {}", 
              response.gpus.len(), response.platform);
        Ok(Response::new(response))
    }

    type StreamMiningStatsStream = Pin<Box<dyn Stream<Item = Result<MiningStatsUpdate, Status>> + Send>>;

    async fn stream_mining_stats(
        &self,
        request: Request<StreamingRequest>,
    ) -> Result<Response<Self::StreamMiningStatsStream>, Status> {
        let client_id = Self::get_client_id_from_request(&request);
        self.update_client_activity(client_id.clone()).await;

        let req = request.into_inner();
        let interval_ms = if req.update_interval_ms > 0 {
            req.update_interval_ms
        } else {
            1000 // Default 1 second
        };

        info!("Starting mining stats stream for client {} with {}ms interval", 
              client_id, interval_ms);

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // Spawn a task to generate mock mining statistics
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms as u64));
            let mut share_count = 0u64;
            let start_time = Instant::now();
            
            loop {
                interval.tick().await;
                
                // Generate mock mining stats
                let uptime = start_time.elapsed().as_secs();
                let mock_hashrate = 1500.0 + (uptime as f64 * 10.0) % 500.0; // Simulate varying hashrate
                
                if uptime % 10 == 0 && uptime > 0 {
                    share_count += 1; // Accept a share every 10 seconds
                }

                let stats = MiningStatsUpdate {
                    timestamp: Utc::now().timestamp_millis(),
                    hashrate_hs: mock_hashrate,
                    shares_accepted: share_count,
                    shares_rejected: share_count / 50, // Simulate 2% reject rate
                    uptime_seconds: uptime,
                    gpu_stats: vec![], // Could populate with real GPU data
                    miner_status: "running".to_string(),
                    error_count: 0,
                };

                debug!("Sending mining stats: {:.2} H/s, {} shares, {}s uptime", 
                       mock_hashrate, share_count, uptime);

                if tx.send(Ok(stats)).await.is_err() {
                    info!("Client disconnected from mining stats stream");
                    break;
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }

    async fn start_mining(
        &self,
        request: Request<StartMiningRequest>,
    ) -> Result<Response<StartMiningResponse>, Status> {
        let client_id = Self::get_client_id_from_request(&request);
        self.update_client_activity(client_id).await;

        let req = request.into_inner();
        info!("Start mining request - Pool: {}, Wallet: {}, Miner: {}", 
              req.pool_url, req.wallet_address, req.miner_executable);

        // In a real implementation, this would start the actual mining process
        // For PoC, we'll simulate starting a process
        let mock_process_id = (Utc::now().timestamp_nanos() % 65535) as u32;
        
        let response = StartMiningResponse {
            success: true,
            message: format!("Mock mining process started with PID {}", mock_process_id),
            process_id: mock_process_id,
            start_timestamp: Utc::now().timestamp_millis(),
        };

        info!("Mock mining started with PID: {}", mock_process_id);
        Ok(Response::new(response))
    }

    async fn stop_mining(
        &self,
        request: Request<StopMiningRequest>,
    ) -> Result<Response<StopMiningResponse>, Status> {
        let client_id = Self::get_client_id_from_request(&request);
        self.update_client_activity(client_id).await;

        let req = request.into_inner();
        info!("Stop mining request - PID: {}, Force: {}", 
              req.process_id, req.force_kill);

        let response = StopMiningResponse {
            success: true,
            message: format!("Mock mining process {} stopped", req.process_id),
            stop_timestamp: Utc::now().timestamp_millis(),
        };

        info!("Mock mining stopped for PID: {}", req.process_id);
        Ok(Response::new(response))
    }
}

async fn run_grpc_server(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port).parse()
        .context("Failed to parse server address")?;

    info!("Starting gRPC server on {}", addr);

    let daemon_service = DaemonServiceImpl::new();
    let svc = DaemonServiceServer::new(daemon_service);

    Server::builder()
        .add_service(svc)
        .serve(addr)
        .await
        .context("gRPC server failed")?;

    Ok(())
}

async fn run_client_test(port: u16) -> Result<()> {
    use daemon::daemon_service_client::DaemonServiceClient;

    let addr = format!("http://127.0.0.1:{}", port);
    info!("Connecting to gRPC server at {}", addr);

    let mut client = DaemonServiceClient::connect(addr).await
        .context("Failed to connect to gRPC server")?;

    // Test 1: Ping/Pong
    info!("=== Testing Ping/Pong ===");
    let ping_request = PingRequest {
        message: "Hello from PoC client!".to_string(),
        timestamp: Utc::now().timestamp_millis(),
    };

    let start_time = Instant::now();
    let response = client.ping(Request::new(ping_request)).await
        .context("Ping request failed")?;
    let latency = start_time.elapsed();

    let ping_response = response.into_inner();
    info!("Ping response: '{}'", ping_response.echo_message);
    info!("Server version: {}", ping_response.server_version);
    info!("Round-trip latency: {:?}", latency);

    // Test 2: System Information
    info!("\n=== Testing System Information ===");
    let system_request = SystemInfoRequest {
        include_gpu_details: true,
        include_cpu_details: true,
    };

    let response = client.get_system_info(Request::new(system_request)).await
        .context("System info request failed")?;

    let system_info = response.into_inner();
    info!("Platform: {}", system_info.platform);
    info!("Daemon version: {}", system_info.daemon_version);
    info!("GPUs found: {}", system_info.gpus.len());
    if let Some(cpu) = &system_info.cpu {
        info!("CPU: {} ({} cores)", cpu.name, cpu.cores_physical);
    }
    info!("Memory: {}GB total, {}GB available", 
          system_info.total_memory_gb, system_info.available_memory_gb);

    // Test 3: Streaming (for 15 seconds)
    info!("\n=== Testing Mining Stats Streaming ===");
    let stream_request = StreamingRequest {
        update_interval_ms: 2000, // 2 second intervals
    };

    let mut stream = client.stream_mining_stats(Request::new(stream_request)).await
        .context("Failed to start stream")?
        .into_inner();

    let mut received_count = 0;
    let stream_start = Instant::now();
    let max_stream_duration = Duration::from_secs(15);

    while let Some(result) = stream.next().await {
        if stream_start.elapsed() > max_stream_duration {
            info!("Stream test duration reached, stopping...");
            break;
        }

        match result {
            Ok(stats) => {
                received_count += 1;
                info!("Stream update #{}: {:.2} H/s, {} shares, {}s uptime", 
                      received_count, stats.hashrate_hs, stats.shares_accepted, stats.uptime_seconds);
            }
            Err(e) => {
                error!("Stream error: {}", e);
                break;
            }
        }
    }

    info!("Received {} stream updates", received_count);

    // Test 4: Mining Control
    info!("\n=== Testing Mining Control ===");
    let start_request = StartMiningRequest {
        pool_url: "stratum+tcp://pool.example.com:4444".to_string(),
        wallet_address: "48abcdef...".to_string(),
        miner_executable: "xmrig".to_string(),
        miner_args: vec!["--threads=4".to_string()],
        algorithm: "randomx".to_string(),
    };

    let response = client.start_mining(Request::new(start_request)).await
        .context("Start mining request failed")?;

    let start_response = response.into_inner();
    info!("Start mining: {} - {}", start_response.success, start_response.message);

    if start_response.success {
        let stop_request = StopMiningRequest {
            process_id: start_response.process_id,
            force_kill: false,
        };

        let response = client.stop_mining(Request::new(stop_request)).await
            .context("Stop mining request failed")?;

        let stop_response = response.into_inner();
        info!("Stop mining: {} - {}", stop_response.success, stop_response.message);
    }

    info!("✅ All gRPC client tests completed successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("BUNKER MINER - gRPC Server/Client PoC");

    let matches = Command::new("grpc-server")
        .about("BUNKER MINER gRPC Communication Proof of Concept")
        .arg(
            Arg::new("port")
                .long("port")
                .help("Server port")
                .default_value("50051")
                .value_parser(clap::value_parser!(u16))
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .help("Run mode: server, client, or test (both)")
                .default_value("test")
                .value_parser(["server", "client", "test"])
        )
        .get_matches();

    let port = *matches.get_one::<u16>("port").unwrap();
    let mode = matches.get_one::<String>("mode").unwrap();

    match mode.as_str() {
        "server" => {
            run_grpc_server(port).await?;
        }
        "client" => {
            run_client_test(port).await?;
        }
        "test" => {
            // Run both server and client for comprehensive testing
            info!("Starting integrated server/client test...");
            
            // Start server in background
            let server_handle = tokio::spawn(async move {
                if let Err(e) = run_grpc_server(port).await {
                    error!("Server error: {}", e);
                }
            });

            // Wait for server to start
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Run client test
            match run_client_test(port).await {
                Ok(_) => info!("✅ gRPC PoC completed successfully!"),
                Err(e) => error!("❌ Client test failed: {}", e),
            }

            // Cleanup
            server_handle.abort();
        }
        _ => unreachable!(),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_service_creation() {
        let service = DaemonServiceImpl::new();
        assert_eq!(service.version, "0.1.0-poc");
    }

    #[test]
    fn test_client_id_generation() {
        // Test that we can generate unique client IDs
        let id1 = format!("test-{}", Utc::now().timestamp_nanos());
        let id2 = format!("test-{}", Utc::now().timestamp_nanos());
        assert_ne!(id1, id2);
    }
}