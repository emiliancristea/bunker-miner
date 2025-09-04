use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};
use tracing::{debug, error, info, warn};
use sysinfo::System;

use crate::config::{Config, GrpcConfig};
use crate::hardware::{HardwareDetector, MiningDevice};
use crate::miners::{MinerManager, ProcessSupervisor, Telemetry};

// Include the generated gRPC code
include!("generated/bunker.daemon.v1.rs");

use bunker_miner_daemon_server::{BunkerMinerDaemon, BunkerMinerDaemonServer};
use google::protobuf::{Empty, Timestamp};

// Type aliases for better readability
type TelemetryStream = Pin<Box<dyn Stream<Item = Result<Telemetry, Status>> + Send>>;

// Central daemon state shared across all gRPC handlers
#[derive(Debug)]
pub struct DaemonState {
    pub config: RwLock<Config>,
    pub hardware_detector: RwLock<HardwareDetector>,
    pub miner_manager: RwLock<MinerManager>,
    pub process_supervisors: RwLock<HashMap<String, ProcessSupervisor>>,
    pub telemetry_broadcaster: TelemetryBroadcaster,
    pub daemon_version: String,
    pub api_version: String,
    pub build_timestamp: String,
    pub git_commit: String,
    pub start_time: SystemTime,
}

impl DaemonState {
    pub fn new(
        config: Config,
        hardware_detector: HardwareDetector,
        miner_manager: MinerManager,
    ) -> Self {
        Self {
            config: RwLock::new(config),
            hardware_detector: RwLock::new(hardware_detector),
            miner_manager: RwLock::new(miner_manager),
            process_supervisors: RwLock::new(HashMap::new()),
            telemetry_broadcaster: TelemetryBroadcaster::new(),
            daemon_version: env!("CARGO_PKG_VERSION").to_string(),
            api_version: "0.1.0".to_string(),
            build_timestamp: option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown").to_string(),
            git_commit: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown").to_string(),
            start_time: SystemTime::now(),
        }
    }
}

// Telemetry broadcasting system for real-time streaming
#[derive(Debug)]
pub struct TelemetryBroadcaster {
    sender: broadcast::Sender<Telemetry>,
}

impl TelemetryBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000); // Buffer up to 1000 telemetry messages
        Self { sender }
    }
    
    pub fn broadcast(&self, telemetry: Telemetry) {
        if let Err(e) = self.sender.send(telemetry) {
            // Only log if there are actually receivers
            if self.sender.receiver_count() > 0 {
                warn!("Failed to broadcast telemetry: {}", e);
            }
        }
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<Telemetry> {
        self.sender.subscribe()
    }
    
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

// gRPC service implementation
pub struct BunkerMinerDaemonImpl {
    state: Arc<DaemonState>,
}

impl BunkerMinerDaemonImpl {
    pub fn new(state: Arc<DaemonState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl BunkerMinerDaemon for BunkerMinerDaemonImpl {
    async fn get_system_info(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<SystemInfoResponse>, Status> {
        debug!("Received GetSystemInfo request from {:?}", request.remote_addr());
        
        let hardware_detector = self.state.hardware_detector.read().await;
        let devices = hardware_detector.detect_devices()
            .map_err(|e| Status::internal(format!("Failed to detect devices: {}", e)))?;
        
        // Convert internal MiningDevice to gRPC DeviceInfo
        let grpc_devices: Vec<DeviceInfo> = devices.into_iter()
            .map(|device| convert_mining_device_to_device_info(device))
            .collect();
        
        // Get system information
        let mut system = System::new_all();
        system.refresh_all();
        
        let system_info = Some(system_info_response::SystemInfo {
            os_name: std::env::consts::OS.to_string(),
            os_version: "Unknown".to_string(), // TODO: Get actual OS version
            total_memory_gb: (system.total_memory() / 1024 / 1024 / 1024) as u32,
            available_memory_gb: (system.available_memory() / 1024 / 1024 / 1024) as u32,
            cpu_name: system.cpus().first()
                .map(|cpu| cpu.brand().to_string())
                .unwrap_or_else(|| "Unknown CPU".to_string()),
            cpu_cores: system.cpus().len() as u32,
            cpu_threads: system.cpus().len() as u32, // Simplified, actual thread count may differ
            uptime_seconds: system.uptime(),
        });
        
        let version_info = Some(system_info_response::VersionInfo {
            daemon_version: self.state.daemon_version.clone(),
            api_version: self.state.api_version.clone(),
            build_timestamp: self.state.build_timestamp.clone(),
            git_commit: self.state.git_commit.clone(),
        });
        
        let timestamp = Some(Timestamp {
            seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            nanos: 0,
        });
        
        let response = SystemInfoResponse {
            devices: grpc_devices,
            system_info,
            version_info,
            timestamp,
        };
        
        info!("Successfully returned system information with {} devices", response.devices.len());
        Ok(Response::new(response))
    }
    
    async fn health_check(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        debug!("Received HealthCheck request from {:?}", request.remote_addr());
        
        let uptime_seconds = self.state.start_time
            .elapsed()
            .unwrap_or_default()
            .as_secs();
        
        // TODO: Implement comprehensive health checking for all components
        let component_health = vec![
            health_check_response::ComponentHealth {
                component_name: "hardware_detector".to_string(),
                status: health_check_response::HealthStatus::HealthHealthy as i32,
                status_message: "Hardware detection operational".to_string(),
                last_check: Some(Timestamp {
                    seconds: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    nanos: 0,
                }),
            },
            health_check_response::ComponentHealth {
                component_name: "miner_manager".to_string(),
                status: health_check_response::HealthStatus::HealthHealthy as i32,
                status_message: "Miner management operational".to_string(),
                last_check: Some(Timestamp {
                    seconds: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    nanos: 0,
                }),
            },
            health_check_response::ComponentHealth {
                component_name: "telemetry_broadcaster".to_string(),
                status: health_check_response::HealthStatus::HealthHealthy as i32,
                status_message: format!("Broadcasting to {} subscribers", 
                                      self.state.telemetry_broadcaster.subscriber_count()),
                last_check: Some(Timestamp {
                    seconds: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    nanos: 0,
                }),
            },
        ];
        
        let response = HealthCheckResponse {
            status: health_check_response::HealthStatus::HealthHealthy as i32,
            component_health,
            timestamp: Some(Timestamp {
                seconds: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                nanos: 0,
            }),
            uptime_seconds,
        };
        
        Ok(Response::new(response))
    }
    
    async fn start_mining(
        &self,
        request: Request<StartMiningRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        let req = request.into_inner();
        info!("Received StartMining request");
        
        // TODO: Implement mining start logic
        // This would involve:
        // 1. Validating the mining configuration
        // 2. Creating a ProcessSupervisor with the configuration
        // 3. Starting the mining process
        // 4. Adding the supervisor to the state
        
        let response = CommandResponse {
            status: command_response::Status::StatusError as i32,
            message: "Mining start functionality not yet implemented".to_string(),
            error_details: None,
            timestamp: Some(Timestamp {
                seconds: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                nanos: 0,
            }),
            execution_duration_ms: 0,
        };
        
        Ok(Response::new(response))
    }
    
    async fn stop_mining(
        &self,
        request: Request<StopMiningRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        let req = request.into_inner();
        info!("Received StopMining request");
        
        // TODO: Implement mining stop logic
        
        let response = CommandResponse {
            status: command_response::Status::StatusError as i32,
            message: "Mining stop functionality not yet implemented".to_string(),
            error_details: None,
            timestamp: Some(Timestamp {
                seconds: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                nanos: 0,
            }),
            execution_duration_ms: 0,
        };
        
        Ok(Response::new(response))
    }
    
    type StreamTelemetryStream = TelemetryStream;
    
    async fn stream_telemetry(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Self::StreamTelemetryStream>, Status> {
        info!("New telemetry streaming client connected from {:?}", request.remote_addr());
        
        let mut receiver = self.state.telemetry_broadcaster.subscribe();
        let (tx, rx) = mpsc::channel(100);
        
        // Spawn a task to handle the broadcast receiver and forward to gRPC stream
        tokio::spawn(async move {
            while let Ok(telemetry) = receiver.recv().await {
                let grpc_telemetry = convert_telemetry_to_grpc(telemetry);
                
                if let Err(e) = tx.send(Ok(grpc_telemetry)).await {
                    debug!("Telemetry stream client disconnected: {}", e);
                    break;
                }
            }
        });
        
        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::StreamTelemetryStream))
    }
    
    async fn get_profitability(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<ProfitabilityResponse>, Status> {
        debug!("Received GetProfitability request from {:?}", request.remote_addr());
        
        // TODO: Implement profitability calculation
        let response = ProfitabilityResponse {
            profitability_info: vec![],
            recommended_algorithm: "ethereum_ethash".to_string(),
            timestamp: Some(Timestamp {
                seconds: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                nanos: 0,
            }),
            data_age_seconds: 0,
        };
        
        Ok(Response::new(response))
    }
    
    async fn get_config(
        &self,
        request: Request<GetConfigRequest>,
    ) -> Result<Response<GetConfigResponse>, Status> {
        let req = request.into_inner();
        debug!("Received GetConfig request for section: {:?}", req.section);
        
        let config = self.state.config.read().await;
        let config_json = serde_json::to_string_pretty(&*config)
            .map_err(|e| Status::internal(format!("Failed to serialize config: {}", e)))?;
        
        let response = GetConfigResponse {
            config_json,
            config_version: "1.0".to_string(), // TODO: Implement config versioning
            timestamp: Some(Timestamp {
                seconds: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                nanos: 0,
            }),
        };
        
        Ok(Response::new(response))
    }
    
    async fn set_config(
        &self,
        request: Request<SetConfigRequest>,
    ) -> Result<Response<SetConfigResponse>, Status> {
        let req = request.into_inner();
        info!("Received SetConfig request (validate_only: {})", req.validate_only);
        
        // TODO: Implement configuration updates
        let response = SetConfigResponse {
            status: command_response::Status::StatusError as i32,
            validation_errors: vec!["Configuration updates not yet implemented".to_string()],
            services_requiring_restart: vec![],
            timestamp: Some(Timestamp {
                seconds: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                nanos: 0,
            }),
        };
        
        Ok(Response::new(response))
    }
}

// Conversion functions
fn convert_mining_device_to_device_info(device: MiningDevice) -> DeviceInfo {
    let vendor = match device.name.to_lowercase() {
        name if name.contains("nvidia") || name.contains("geforce") || name.contains("rtx") || name.contains("gtx") => {
            device_info::Vendor::VendorNvidia
        },
        name if name.contains("amd") || name.contains("radeon") || name.contains("rx") => {
            device_info::Vendor::VendorAmd
        },
        name if name.contains("intel") => {
            device_info::Vendor::VendorIntel
        },
        _ => device_info::Vendor::VendorUnknown,
    };
    
    let device_type = match device.device_type {
        crate::hardware::DeviceType::GPU => device_info::DeviceType::DeviceTypeGpu,
        crate::hardware::DeviceType::CPU => device_info::DeviceType::DeviceTypeCpu,
    };
    
    DeviceInfo {
        device_id: device.id,
        name: device.name,
        vendor: vendor as i32,
        device_type: device_type as i32,
        vram_mb: device.memory_mb as u32,
        core_count: 0, // TODO: Extract from device properties
        driver_version: device.driver_version.unwrap_or_default(),
        compute_capability: "".to_string(), // TODO: Extract from device properties
        base_clock_mhz: 0, // TODO: Extract from device properties
        memory_clock_mhz: 0, // TODO: Extract from device properties
        power_limit_watts: 0, // TODO: Extract from device properties
        capabilities: device.supported_algorithms,
    }
}

fn convert_telemetry_to_grpc(telemetry: crate::miners::Telemetry) -> Telemetry {
    Telemetry {
        device_id: "device_0".to_string(), // TODO: Map actual device ID
        timestamp: Some(Timestamp {
            seconds: telemetry.timestamp as i64,
            nanos: 0,
        }),
        algorithm: telemetry.algorithm,
        hashrate_mhs: telemetry.hashrate_hs / 1_000_000.0, // Convert H/s to MH/s
        power_watts: telemetry.power_watts.unwrap_or(0.0) as u32,
        temperature_celsius: telemetry.temperature_c.unwrap_or(0.0) as u32,
        fan_speed_percent: telemetry.fan_speed_percent.unwrap_or(0.0) as u32,
        utilization_percent: 0, // TODO: Add utilization to internal telemetry
        memory_utilization_percent: 0,
        core_clock_mhz: 0, // TODO: Add clock info to internal telemetry
        memory_clock_mhz: 0,
        shares: Some(ShareStats {
            accepted: telemetry.shares_accepted as u64,
            rejected: telemetry.shares_rejected as u64,
            stale: telemetry.shares_stale as u64,
            acceptance_rate: if telemetry.shares_accepted + telemetry.shares_rejected > 0 {
                telemetry.shares_accepted as f32 / (telemetry.shares_accepted + telemetry.shares_rejected) as f32
            } else {
                0.0
            },
            avg_share_time_seconds: 0.0, // TODO: Calculate share timing
            last_share_time: None, // TODO: Track last share time
        }),
        device_status: telemetry::DeviceStatus::DeviceStatusMining as i32,
        error_message: "".to_string(),
        pool_status: telemetry::PoolStatus::PoolStatusConnected as i32,
        pool_url: "".to_string(), // TODO: Get pool URL from mining config
    }
}

// gRPC server management
pub struct GrpcServer {
    state: Arc<DaemonState>,
    config: GrpcConfig,
}

impl GrpcServer {
    pub fn new(state: Arc<DaemonState>, config: GrpcConfig) -> Self {
        Self { state, config }
    }
    
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            info!("gRPC server disabled in configuration");
            return Ok(());
        }
        
        let addr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse()
            .map_err(|e| anyhow!("Invalid gRPC bind address: {}", e))?;
        
        let service = BunkerMinerDaemonImpl::new(self.state.clone());
        let server = BunkerMinerDaemonServer::new(service);
        
        info!("Starting gRPC server on {}", addr);
        
        if self.config.tls_enabled {
            // TODO: Implement TLS configuration
            warn!("TLS configuration not yet implemented, falling back to insecure connection");
        }
        
        Server::builder()
            .add_service(server)
            .serve(addr)
            .await
            .map_err(|e| anyhow!("gRPC server error: {}", e))?;
        
        Ok(())
    }
}