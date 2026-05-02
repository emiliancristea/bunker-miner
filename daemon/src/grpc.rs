use anyhow::anyhow;
use prost_types::Timestamp;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{transport::Server, Request, Response, Status};
use tracing::{debug, info, warn};

use crate::config::{Config, GrpcConfig, PoolConfig, WalletConfig};
use crate::hardware::{DeviceType as HardwareDeviceType, HardwareDetector, MiningDevice};
use crate::miner_installer::MinerInstaller;
use crate::miners::{MinerManager, ProcessStatus, ProcessSupervisor, Telemetry as MinerTelemetry};
use crate::profit_engine::ProfitEngineService;

include!("generated/bunker.daemon.v1.rs");

use bunker_miner_daemon_server::{BunkerMinerDaemon, BunkerMinerDaemonServer};

type TelemetryStream =
    Pin<Box<dyn Stream<Item = std::result::Result<Telemetry, Status>> + Send + 'static>>;
type OverclockStateStream =
    Pin<Box<dyn Stream<Item = std::result::Result<OverclockState, Status>> + Send + 'static>>;

pub struct DaemonState {
    pub config: RwLock<Config>,
    pub hardware_detector: RwLock<HardwareDetector>,
    pub miner_manager: RwLock<MinerManager>,
    pub process_supervisors: RwLock<HashMap<String, ProcessSupervisor>>,
    pub mining_lifecycle: RwLock<MiningLifecycleSnapshot>,
    pub telemetry_broadcaster: TelemetryBroadcaster,
    pub profit_engine_service: Option<Arc<ProfitEngineService>>,
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
            mining_lifecycle: RwLock::new(MiningLifecycleSnapshot::idle()),
            telemetry_broadcaster: TelemetryBroadcaster::new(),
            profit_engine_service: None,
            daemon_version: env!("CARGO_PKG_VERSION").to_string(),
            api_version: "0.1.0".to_string(),
            build_timestamp: option_env!("VERGEN_BUILD_TIMESTAMP")
                .unwrap_or("unknown")
                .to_string(),
            git_commit: option_env!("VERGEN_GIT_SHA")
                .unwrap_or("unknown")
                .to_string(),
            start_time: SystemTime::now(),
        }
    }

    pub fn set_profit_engine(&mut self, profit_service: Arc<ProfitEngineService>) {
        self.profit_engine_service = Some(profit_service);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningLifecycleState {
    Idle,
    Installing,
    Starting,
    Running,
    Stopping,
    Stopped,
    Error,
    Crashed,
    Restarting,
    Degraded,
}

#[derive(Debug, Clone)]
pub struct MiningLifecycleSnapshot {
    pub state: MiningLifecycleState,
    pub status_message: String,
    pub error_details: Option<command_response::ErrorDetails>,
}

impl MiningLifecycleSnapshot {
    pub fn idle() -> Self {
        Self::new(MiningLifecycleState::Idle, "No active mining process")
    }

    pub fn new(state: MiningLifecycleState, status_message: impl Into<String>) -> Self {
        Self {
            state,
            status_message: status_message.into(),
            error_details: None,
        }
    }

    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        remediation_steps: &[&str],
    ) -> Self {
        let message = message.into();
        Self {
            state: MiningLifecycleState::Error,
            status_message: message.clone(),
            error_details: Some(command_response::ErrorDetails {
                error_code: code.into(),
                error_description: message,
                affected_devices: vec![],
                remediation_steps: remediation_steps
                    .iter()
                    .map(|step| step.to_string())
                    .collect(),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryBroadcaster {
    sender: broadcast::Sender<MinerTelemetry>,
}

impl TelemetryBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    pub fn broadcast(&self, telemetry: MinerTelemetry) {
        if let Err(error) = self.sender.send(telemetry) {
            if self.sender.receiver_count() > 0 {
                warn!("Failed to broadcast telemetry: {}", error);
            }
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MinerTelemetry> {
        self.sender.subscribe()
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for TelemetryBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

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
        request: Request<()>,
    ) -> std::result::Result<Response<SystemInfoResponse>, Status> {
        debug!(
            "Received GetSystemInfo request from {:?}",
            request.remote_addr()
        );

        let devices = {
            let mut hardware_detector = self.state.hardware_detector.write().await;
            hardware_detector
                .detect_devices()
                .map_err(|error| Status::internal(format!("Failed to detect devices: {error}")))?
        };

        let grpc_devices = devices
            .into_iter()
            .map(convert_mining_device_to_device_info)
            .collect();

        let mut system = System::new_all();
        system.refresh_all();

        let response = SystemInfoResponse {
            devices: grpc_devices,
            system_info: Some(system_info_response::SystemInfo {
                os_name: std::env::consts::OS.to_string(),
                os_version: "Unknown".to_string(),
                total_memory_gb: (system.total_memory() / 1024 / 1024 / 1024) as u32,
                available_memory_gb: (system.available_memory() / 1024 / 1024 / 1024) as u32,
                cpu_name: system
                    .cpus()
                    .first()
                    .map(|cpu| cpu.brand().to_string())
                    .unwrap_or_else(|| "Unknown CPU".to_string()),
                cpu_cores: system
                    .physical_core_count()
                    .unwrap_or_else(|| system.cpus().len()) as u32,
                cpu_threads: system.cpus().len() as u32,
                uptime_seconds: system.uptime(),
            }),
            version_info: Some(system_info_response::VersionInfo {
                daemon_version: self.state.daemon_version.clone(),
                api_version: self.state.api_version.clone(),
                build_timestamp: self.state.build_timestamp.clone(),
                git_commit: self.state.git_commit.clone(),
            }),
            timestamp: Some(now_timestamp()),
        };

        info!(
            "Returned system information with {} devices",
            response.devices.len()
        );
        Ok(Response::new(response))
    }

    async fn health_check(
        &self,
        request: Request<()>,
    ) -> std::result::Result<Response<HealthCheckResponse>, Status> {
        debug!(
            "Received HealthCheck request from {:?}",
            request.remote_addr()
        );

        let miner_count = self.state.process_supervisors.read().await.len();
        let uptime_seconds = self
            .state
            .start_time
            .elapsed()
            .unwrap_or_default()
            .as_secs();

        let component_health = vec![
            health_component("hardware_detector", "Hardware detection initialized"),
            health_component(
                "miner_processes",
                format!("{miner_count} active process supervisor(s)"),
            ),
            health_component(
                "telemetry_broadcaster",
                format!(
                    "Broadcasting to {} subscriber(s)",
                    self.state.telemetry_broadcaster.subscriber_count()
                ),
            ),
        ];

        Ok(Response::new(HealthCheckResponse {
            status: health_check_response::HealthStatus::HealthHealthy as i32,
            component_health,
            timestamp: Some(now_timestamp()),
            uptime_seconds,
        }))
    }

    async fn start_mining(
        &self,
        request: Request<StartMiningRequest>,
    ) -> std::result::Result<Response<CommandResponse>, Status> {
        let started_at = Instant::now();
        let req = request.into_inner();
        let start_timeout = bounded_timeout(req.timeout_seconds, 30, 300);

        info!("Received StartMining request");

        if !self.state.process_supervisors.read().await.is_empty() {
            if !req.stop_existing {
                return Ok(Response::new(command_error(
                    "ALREADY_RUNNING",
                    "Mining is already running; set stop_existing=true to replace the current process",
                    ["Stop the existing miner first or request replacement"].as_slice(),
                    started_at,
                )));
            }

            *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::new(
                MiningLifecycleState::Stopping,
                "Stopping existing miner before replacement",
            );
            let existing = drain_supervisors(&self.state).await;
            if let Err(status) =
                stop_supervisors(existing, Duration::from_secs(start_timeout)).await
            {
                *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::error(
                    "STOP_BEFORE_REPLACE_FAILED",
                    status.message().to_string(),
                    ["Stop the existing miner manually before retrying"].as_slice(),
                );
                return Err(status);
            }
        }

        let mut config = self.state.config.read().await.clone();
        let target_device_ids =
            apply_grpc_mining_config(&mut config, req.config).map_err(Status::invalid_argument)?;
        config
            .validate_mining_ready()
            .map_err(|error| Status::invalid_argument(error.to_string()))?;
        let miner_device_ids =
            normalize_miner_device_ids(&target_device_ids, &config.mining.active_coin);

        *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::new(
            MiningLifecycleState::Starting,
            format!("Starting mining for {}", config.mining.active_coin),
        );

        let adapter = {
            let miner_manager = self.state.miner_manager.read().await;
            match miner_manager.get_adapter_for_coin(&config.mining.active_coin) {
                Some(adapter) => adapter,
                None => {
                    *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::error(
                        "MINER_ADAPTER_UNAVAILABLE",
                        format!(
                            "No miner adapter available for coin '{}'",
                            config.mining.active_coin
                        ),
                        ["Select a supported coin/algorithm"].as_slice(),
                    );
                    return Err(Status::failed_precondition(format!(
                        "No miner adapter available for coin '{}'",
                        config.mining.active_coin
                    )));
                }
            }
        };

        let binary_path = {
            let miner_manager = self.state.miner_manager.read().await;
            match miner_manager.ensure_binary_available(&adapter).await {
                Ok(binary_path) => binary_path,
                Err(error) => {
                    *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::error(
                        "MINER_BINARY_UNAVAILABLE",
                        error.to_string(),
                        [
                            "Run bunker-miner-cli miner install with a trusted manifest entry",
                            "Or install the selected miner binary manually and provide a trusted SHA-256 sidecar file or environment variable",
                            "Set BUNKER_MINERS_PATH or BUNKER_MINER_<MINER>_PATH if the binary is outside the managed directory",
                        ]
                        .as_slice(),
                    );
                    return Ok(Response::new(command_error(
                        "MINER_BINARY_UNAVAILABLE",
                        error.to_string(),
                        [
                            "Run bunker-miner-cli miner install with a trusted manifest entry",
                            "Or install the selected miner binary manually and provide a trusted SHA-256 sidecar file or environment variable",
                            "Set BUNKER_MINERS_PATH or BUNKER_MINER_<MINER>_PATH if the binary is outside the managed directory",
                        ]
                        .as_slice(),
                        started_at,
                    )));
                }
            }
        };

        let (telemetry_tx, mut telemetry_rx) = mpsc::unbounded_channel::<MinerTelemetry>();
        let mut supervisor = ProcessSupervisor::new(
            config.clone(),
            adapter.clone(),
            binary_path,
            miner_device_ids,
        );

        match tokio::time::timeout(
            Duration::from_secs(start_timeout),
            supervisor.start(telemetry_tx),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::error(
                    "START_FAILED",
                    format!("Failed to start miner process: {error}"),
                    ["Verify miner binary, pool, wallet, and device configuration"].as_slice(),
                );
                return Ok(Response::new(command_error(
                    "START_FAILED",
                    format!("Failed to start miner process: {error}"),
                    ["Verify miner binary, pool, wallet, and device configuration"].as_slice(),
                    started_at,
                )));
            }
            Err(_) => {
                *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::error(
                    "START_TIMEOUT",
                    "Timed out while starting miner process",
                    ["Check miner binary and system load, then retry"].as_slice(),
                );
                return Ok(Response::new(command_with_status(
                    command_response::Status::Timeout,
                    "Timed out while starting miner process",
                    started_at,
                )));
            }
        }

        let broadcaster = self.state.telemetry_broadcaster.clone();
        tokio::spawn(async move {
            while let Some(telemetry) = telemetry_rx.recv().await {
                broadcaster.broadcast(telemetry);
            }
        });

        self.state
            .process_supervisors
            .write()
            .await
            .insert("default".to_string(), supervisor);

        *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::new(
            MiningLifecycleState::Running,
            format!(
                "Started {} mining for {}",
                adapter.get_name(),
                config.mining.active_coin
            ),
        );

        Ok(Response::new(command_with_status(
            command_response::Status::Success,
            format!(
                "Started {} mining for {}",
                adapter.get_name(),
                config.mining.active_coin
            ),
            started_at,
        )))
    }

    async fn stop_mining(
        &self,
        request: Request<StopMiningRequest>,
    ) -> std::result::Result<Response<CommandResponse>, Status> {
        let started_at = Instant::now();
        let req = request.into_inner();
        let stop_timeout = bounded_timeout(req.timeout_seconds, 10, 60);
        let stop_mode = if req.force_stop { "force" } else { "standard" };

        info!("Received StopMining request ({stop_mode} mode)");

        let selected = select_supervisors_for_stop(&self.state, &req.device_ids).await;
        if selected.is_empty() {
            return Ok(Response::new(command_error(
                "NO_MATCHING_MINERS",
                "No matching miner process is running",
                ["Check active mining status before issuing stop"].as_slice(),
                started_at,
            )));
        }

        let stopped = selected.len();
        *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::new(
            MiningLifecycleState::Stopping,
            format!("Stopping {stopped} miner process supervisor(s)"),
        );
        if let Err(status) = stop_supervisors(selected, Duration::from_secs(stop_timeout)).await {
            *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::error(
                "STOP_FAILED",
                status.message().to_string(),
                ["Retry with --force or inspect daemon logs"].as_slice(),
            );
            return Err(status);
        }
        *self.state.mining_lifecycle.write().await = MiningLifecycleSnapshot::new(
            MiningLifecycleState::Stopped,
            format!("Stopped {stopped} miner process supervisor(s)"),
        );

        Ok(Response::new(command_with_status(
            command_response::Status::Success,
            format!("Stopped {stopped} miner process supervisor(s)"),
            started_at,
        )))
    }

    async fn get_mining_state(
        &self,
        request: Request<()>,
    ) -> std::result::Result<Response<MiningStateResponse>, Status> {
        debug!(
            "Received GetMiningState request from {:?}",
            request.remote_addr()
        );

        let lifecycle = self.state.mining_lifecycle.read().await.clone();
        let supervisors = self.state.process_supervisors.read().await;
        if let Some((_key, supervisor)) = supervisors.iter().next() {
            let latest_telemetry = supervisor.get_latest_telemetry().await;
            let process_status = *supervisor.get_status();
            let (state, status_message, error_details) =
                if lifecycle_overrides_process(lifecycle.state) {
                    (
                        mining_lifecycle_to_grpc(lifecycle.state),
                        lifecycle.status_message,
                        lifecycle.error_details,
                    )
                } else {
                    (
                        mining_lifecycle_from_process_status(process_status),
                        process_status_message(process_status).to_string(),
                        mining_state_error_details(process_status),
                    )
                };
            let response = mining_state_response(MiningStateResponseInput {
                state,
                status_message,
                miner_name: Some(supervisor.miner_name()),
                config: supervisor.config(),
                device_ids: supervisor.device_ids(),
                restart_count: supervisor.get_restart_count(),
                latest_telemetry,
                error_details,
            });

            return Ok(Response::new(response));
        }
        drop(supervisors);

        let config = self.state.config.read().await;
        Ok(Response::new(mining_state_response(
            MiningStateResponseInput {
                state: mining_lifecycle_to_grpc(lifecycle.state),
                status_message: lifecycle.status_message,
                miner_name: None,
                config: &config,
                device_ids: &[],
                restart_count: 0,
                latest_telemetry: None,
                error_details: lifecycle.error_details,
            },
        )))
    }

    async fn install_miner(
        &self,
        request: Request<InstallMinerRequest>,
    ) -> std::result::Result<Response<InstallMinerResponse>, Status> {
        let started_at = Instant::now();
        let req = request.into_inner();
        let miner_name = req.miner_name.trim().to_string();
        let version = req.version.trim().to_string();
        let install_timeout = bounded_timeout(req.timeout_seconds, 120, 600);

        if miner_name.is_empty() {
            return Err(Status::invalid_argument("miner_name is required"));
        }

        info!(
            "Received InstallMiner request for {} {}",
            miner_name,
            if version.is_empty() {
                "(manifest-selected version)"
            } else {
                version.as_str()
            }
        );

        let binaries_dir = {
            let miner_manager = self.state.miner_manager.read().await;
            miner_manager.binaries_dir().to_path_buf()
        };
        let installer = MinerInstaller::new(binaries_dir);
        let version_ref = if version.is_empty() {
            None
        } else {
            Some(version.as_str())
        };

        match tokio::time::timeout(
            Duration::from_secs(install_timeout),
            installer.install_from_manifest(&miner_name, version_ref, req.force),
        )
        .await
        {
            Ok(Ok(result)) => Ok(Response::new(install_miner_success(result, started_at))),
            Ok(Err(error)) => Ok(Response::new(install_miner_error(
                "MINER_INSTALL_FAILED",
                error.to_string(),
                [
                    "Set BUNKER_MINER_MANIFEST_PATH or provide managed miner-manifest.toml",
                    "Ensure the manifest has archive_sha256 and executable sha256 for this platform",
                    "Use force only when intentionally replacing a mismatched existing executable",
                ]
                .as_slice(),
                started_at,
            ))),
            Err(_) => Ok(Response::new(InstallMinerResponse {
                status: command_response::Status::Timeout as i32,
                message: "Timed out while installing miner".to_string(),
                installed_miner: None,
                error_details: None,
                timestamp: Some(now_timestamp()),
                execution_duration_ms: elapsed_ms(started_at),
            })),
        }
    }

    type StreamTelemetryStream = TelemetryStream;

    async fn stream_telemetry(
        &self,
        request: Request<()>,
    ) -> std::result::Result<Response<Self::StreamTelemetryStream>, Status> {
        info!(
            "New telemetry streaming client connected from {:?}",
            request.remote_addr()
        );

        let mut receiver = self.state.telemetry_broadcaster.subscribe();
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            while let Ok(telemetry) = receiver.recv().await {
                if tx
                    .send(Ok(convert_telemetry_to_grpc(telemetry)))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::StreamTelemetryStream
        ))
    }

    async fn get_profitability(
        &self,
        request: Request<()>,
    ) -> std::result::Result<Response<ProfitabilityResponse>, Status> {
        debug!(
            "Received GetProfitability request from {:?}",
            request.remote_addr()
        );

        let Some(profit_service) = &self.state.profit_engine_service else {
            return Ok(Response::new(ProfitabilityResponse {
                profitability_info: vec![],
                recommended_algorithm: String::new(),
                timestamp: Some(now_timestamp()),
                data_age_seconds: 0,
            }));
        };

        let profitability_data = profit_service.get_profitability_rankings().await;
        let current_time = unix_now();
        let recommended_algorithm = profitability_data
            .first()
            .map(|data| data.algorithm.clone())
            .unwrap_or_default();
        let data_age_seconds = profitability_data
            .first()
            .map(|data| current_time.saturating_sub(data.last_updated) as u32)
            .unwrap_or(0);

        let profitability_info = profitability_data
            .iter()
            .map(|data| ProfitabilityInfo {
                algorithm: data.algorithm.clone(),
                coin: data.coin_symbol.clone(),
                revenue_eur_day: data.revenue_eur_per_day,
                cost_eur_day: data.cost_eur_per_day,
                profit_eur_day: data.net_profit_eur_per_day,
                network_difficulty: 0.0,
                coin_price_eur: 0.0,
                calculated_at: Some(Timestamp {
                    seconds: data.last_updated as i64,
                    nanos: 0,
                }),
                confidence: 0.95,
                data_source: "BUNKER MINER Engine".to_string(),
            })
            .collect();

        Ok(Response::new(ProfitabilityResponse {
            profitability_info,
            recommended_algorithm,
            timestamp: Some(Timestamp {
                seconds: current_time as i64,
                nanos: 0,
            }),
            data_age_seconds,
        }))
    }

    async fn get_config(
        &self,
        request: Request<GetConfigRequest>,
    ) -> std::result::Result<Response<GetConfigResponse>, Status> {
        let req = request.into_inner();
        debug!("Received GetConfig request for section: {:?}", req.section);

        let config = self.state.config.read().await;
        let config_json = serde_json::to_string_pretty(&*config)
            .map_err(|error| Status::internal(format!("Failed to serialize config: {error}")))?;

        Ok(Response::new(GetConfigResponse {
            config_json,
            config_version: "1.0".to_string(),
            timestamp: Some(now_timestamp()),
        }))
    }

    async fn set_config(
        &self,
        request: Request<SetConfigRequest>,
    ) -> std::result::Result<Response<SetConfigResponse>, Status> {
        let req = request.into_inner();
        info!(
            "Received SetConfig request (validate_only: {})",
            req.validate_only
        );

        Ok(Response::new(SetConfigResponse {
            status: command_response::Status::Error as i32,
            validation_errors: vec![
                "Configuration updates are not implemented for the daemon API yet".to_string(),
            ],
            services_requiring_restart: vec![],
            timestamp: Some(now_timestamp()),
        }))
    }

    async fn list_overclock_profiles(
        &self,
        request: Request<ListOverclockProfilesRequest>,
    ) -> std::result::Result<Response<ListOverclockProfilesResponse>, Status> {
        let filter = request.into_inner().algorithm_filter;
        let config = self.state.config.read().await;
        let profiles = config
            .overclocking
            .profiles
            .iter()
            .filter(|(algorithm, _)| filter.is_empty() || algorithm.as_str() == filter)
            .map(|(_, profile)| convert_overclock_profile(profile))
            .collect();

        Ok(Response::new(ListOverclockProfilesResponse {
            profiles,
            expert_mode_enabled: config.overclocking.expert_mode_enabled,
            safety_settings: Some(list_overclock_profiles_response::SafetySettings {
                max_core_clock_offset_mhz: config
                    .overclocking
                    .safety_settings
                    .max_core_clock_offset_mhz,
                max_memory_clock_offset_mhz: config
                    .overclocking
                    .safety_settings
                    .max_memory_clock_offset_mhz,
                max_power_limit_watts: config.overclocking.safety_settings.max_power_limit_watts,
                max_temperature_limit_c: config
                    .overclocking
                    .safety_settings
                    .max_temperature_limit_c,
                emergency_temperature_c: config
                    .overclocking
                    .safety_settings
                    .emergency_temperature_c,
            }),
            timestamp: Some(now_timestamp()),
        }))
    }

    async fn apply_overclock_profile(
        &self,
        request: Request<ApplyOverclockProfileRequest>,
    ) -> std::result::Result<Response<CommandResponse>, Status> {
        let started_at = Instant::now();
        let req = request.into_inner();

        Ok(Response::new(command_error(
            "OVERCLOCKING_UNSUPPORTED",
            format!(
                "Overclocking is not implemented in this build; profile '{}' was not applied to {}",
                req.algorithm, req.device_id
            ),
            ["Keep hardware clocks at driver defaults"].as_slice(),
            started_at,
        )))
    }

    async fn revert_overclock(
        &self,
        request: Request<RevertOverclockRequest>,
    ) -> std::result::Result<Response<CommandResponse>, Status> {
        let started_at = Instant::now();
        let req = request.into_inner();
        let target = if req.device_ids.is_empty() {
            "all devices".to_string()
        } else {
            req.device_ids.join(", ")
        };

        Ok(Response::new(command_with_status(
            command_response::Status::Success,
            format!("No overclocking is active; defaults remain in effect for {target}"),
            started_at,
        )))
    }

    async fn get_hardware_defaults(
        &self,
        request: Request<GetHardwareDefaultsRequest>,
    ) -> std::result::Result<Response<HardwareDefaults>, Status> {
        let device_id = request.into_inner().device_id;
        if device_id.is_empty() {
            return Err(Status::invalid_argument("device_id is required"));
        }

        let devices = {
            let mut hardware_detector = self.state.hardware_detector.write().await;
            hardware_detector
                .detect_devices()
                .map_err(|error| Status::internal(format!("Failed to detect devices: {error}")))?
        };

        let device = devices
            .into_iter()
            .find(|device| device.id == device_id)
            .ok_or_else(|| Status::not_found(format!("Device '{device_id}' was not found")))?;

        Ok(Response::new(hardware_defaults_from_device(&device)))
    }

    type GetOverclockStatesStream = OverclockStateStream;

    async fn get_overclock_states(
        &self,
        request: Request<()>,
    ) -> std::result::Result<Response<Self::GetOverclockStatesStream>, Status> {
        debug!(
            "Received GetOverclockStates request from {:?}",
            request.remote_addr()
        );

        let devices = {
            let mut hardware_detector = self.state.hardware_detector.write().await;
            hardware_detector
                .detect_devices()
                .map_err(|error| Status::internal(format!("Failed to detect devices: {error}")))?
        };

        let states = devices
            .iter()
            .map(|device| OverclockState {
                device_id: device.id.clone(),
                applied_profile: None,
                defaults: Some(hardware_defaults_from_device(device)),
                is_overclocked: false,
                last_applied: None,
                safety_status: overclock_state::SafetyStatus::Safe as i32,
            })
            .collect::<Vec<_>>();

        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            for state in states {
                if tx.send(Ok(state)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::GetOverclockStatesStream
        ))
    }
}

async fn drain_supervisors(state: &DaemonState) -> Vec<(String, ProcessSupervisor)> {
    state.process_supervisors.write().await.drain().collect()
}

struct MiningStateResponseInput<'a> {
    state: mining_state_response::LifecycleState,
    status_message: String,
    miner_name: Option<&'a str>,
    config: &'a Config,
    device_ids: &'a [String],
    restart_count: u32,
    latest_telemetry: Option<MinerTelemetry>,
    error_details: Option<command_response::ErrorDetails>,
}

fn mining_state_response(input: MiningStateResponseInput<'_>) -> MiningStateResponse {
    let active_wallet = input.config.get_active_wallet().ok();
    let active_pool = input.config.get_active_pool().ok();
    let latest_telemetry = input.latest_telemetry.map(convert_telemetry_to_grpc);

    MiningStateResponse {
        state: input.state as i32,
        status_message: input.status_message,
        miner_name: input.miner_name.unwrap_or_default().to_string(),
        active_coin: input.config.mining.active_coin.clone(),
        algorithm: algorithm_for_coin(&input.config.mining.active_coin).to_string(),
        pool_url: active_pool.map(format_pool_endpoint).unwrap_or_default(),
        wallet_label: active_wallet
            .and_then(|wallet| wallet.label.clone())
            .unwrap_or_else(|| input.config.mining.active_wallet.clone()),
        wallet_address_redacted: active_wallet
            .map(|wallet| redact_wallet_address(&wallet.address))
            .unwrap_or_default(),
        target_device_ids: input.device_ids.to_vec(),
        restart_count: input.restart_count,
        telemetry_available: latest_telemetry.is_some(),
        latest_telemetry,
        error_details: input.error_details,
        timestamp: Some(now_timestamp()),
    }
}

fn mining_lifecycle_from_process_status(
    status: ProcessStatus,
) -> mining_state_response::LifecycleState {
    match status {
        ProcessStatus::Starting => {
            mining_state_response::LifecycleState::MiningLifecycleStateStarting
        }
        ProcessStatus::Running => {
            mining_state_response::LifecycleState::MiningLifecycleStateRunning
        }
        ProcessStatus::Stopped => {
            mining_state_response::LifecycleState::MiningLifecycleStateStopped
        }
        ProcessStatus::Crashed => {
            mining_state_response::LifecycleState::MiningLifecycleStateCrashed
        }
        ProcessStatus::Restarting => {
            mining_state_response::LifecycleState::MiningLifecycleStateRestarting
        }
    }
}

fn mining_lifecycle_to_grpc(state: MiningLifecycleState) -> mining_state_response::LifecycleState {
    match state {
        MiningLifecycleState::Idle => {
            mining_state_response::LifecycleState::MiningLifecycleStateIdle
        }
        MiningLifecycleState::Installing => {
            mining_state_response::LifecycleState::MiningLifecycleStateInstalling
        }
        MiningLifecycleState::Starting => {
            mining_state_response::LifecycleState::MiningLifecycleStateStarting
        }
        MiningLifecycleState::Running => {
            mining_state_response::LifecycleState::MiningLifecycleStateRunning
        }
        MiningLifecycleState::Stopping => {
            mining_state_response::LifecycleState::MiningLifecycleStateStopping
        }
        MiningLifecycleState::Stopped => {
            mining_state_response::LifecycleState::MiningLifecycleStateStopped
        }
        MiningLifecycleState::Error => {
            mining_state_response::LifecycleState::MiningLifecycleStateError
        }
        MiningLifecycleState::Crashed => {
            mining_state_response::LifecycleState::MiningLifecycleStateCrashed
        }
        MiningLifecycleState::Restarting => {
            mining_state_response::LifecycleState::MiningLifecycleStateRestarting
        }
        MiningLifecycleState::Degraded => {
            mining_state_response::LifecycleState::MiningLifecycleStateDegraded
        }
    }
}

fn lifecycle_overrides_process(state: MiningLifecycleState) -> bool {
    matches!(
        state,
        MiningLifecycleState::Installing
            | MiningLifecycleState::Starting
            | MiningLifecycleState::Stopping
            | MiningLifecycleState::Error
    )
}

fn process_status_message(status: ProcessStatus) -> &'static str {
    match status {
        ProcessStatus::Starting => "Miner process is starting",
        ProcessStatus::Running => "Miner process is running",
        ProcessStatus::Stopped => "Miner process is stopped",
        ProcessStatus::Crashed => "Miner process crashed",
        ProcessStatus::Restarting => "Miner process is restarting",
    }
}

fn mining_state_error_details(status: ProcessStatus) -> Option<command_response::ErrorDetails> {
    if status != ProcessStatus::Crashed {
        return None;
    }

    Some(command_response::ErrorDetails {
        error_code: "MINER_PROCESS_CRASHED".to_string(),
        error_description: "Miner process exited unexpectedly".to_string(),
        affected_devices: vec![],
        remediation_steps: vec![
            "Check pool, wallet, miner binary, and device configuration".to_string(),
            "Review daemon logs for the miner exit reason".to_string(),
        ],
    })
}

fn algorithm_for_coin(coin: &str) -> &'static str {
    match coin {
        "monero" => "randomx",
        "wownero" => "randomwow",
        "ethereum" => "ethash",
        "ethereum_classic" => "etchash",
        "beam" => "beamhash",
        _ => "unknown",
    }
}

fn format_pool_endpoint(pool: &PoolConfig) -> String {
    let url = pool.url.trim().trim_end_matches('/');
    format!("{url}:{}", pool.port)
}

fn redact_wallet_address(address: &str) -> String {
    let address = address.trim();
    if address.is_empty() {
        return String::new();
    }

    let char_count = address.chars().count();
    if char_count <= 12 {
        return "<redacted>".to_string();
    }

    let prefix = address.chars().take(6).collect::<String>();
    let suffix = address
        .chars()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();

    format!("{prefix}...{suffix}")
}

async fn select_supervisors_for_stop(
    state: &DaemonState,
    device_ids: &[String],
) -> Vec<(String, ProcessSupervisor)> {
    let mut supervisors = state.process_supervisors.write().await;
    let selected_keys = if device_ids.is_empty() {
        supervisors.keys().cloned().collect::<Vec<_>>()
    } else {
        supervisors
            .iter()
            .filter(|(key, supervisor)| {
                device_ids.contains(key)
                    || device_ids
                        .iter()
                        .any(|device_id| supervisor.device_ids().contains(device_id))
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>()
    };

    selected_keys
        .into_iter()
        .filter_map(|key| supervisors.remove(&key).map(|supervisor| (key, supervisor)))
        .collect()
}

async fn stop_supervisors(
    supervisors: Vec<(String, ProcessSupervisor)>,
    timeout: Duration,
) -> std::result::Result<(), Status> {
    for (key, mut supervisor) in supervisors {
        match tokio::time::timeout(timeout, supervisor.stop()).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                return Err(Status::internal(format!(
                    "Failed to stop miner supervisor '{key}': {error}"
                )));
            }
            Err(_) => {
                return Err(Status::deadline_exceeded(format!(
                    "Timed out stopping miner supervisor '{key}'"
                )));
            }
        }
    }

    Ok(())
}

fn apply_grpc_mining_config(
    config: &mut Config,
    grpc_config: Option<MiningConfig>,
) -> std::result::Result<Vec<String>, String> {
    let Some(grpc_config) = grpc_config else {
        return Ok(Vec::new());
    };

    if !grpc_config.algorithm.is_empty() {
        if grpc_config.algorithm.len() > 32 {
            return Err("algorithm must be at most 32 characters".to_string());
        }

        if let Some(coin) = infer_coin_from_algorithm(&grpc_config.algorithm) {
            config.mining.active_coin = coin.to_string();
        }
    }

    if !grpc_config.wallet_address.is_empty() {
        if grpc_config.wallet_address.len() > 256 {
            return Err("wallet_address must be at most 256 characters".to_string());
        }

        let wallet_key = "grpc_request_wallet".to_string();
        config.wallets.insert(
            wallet_key.clone(),
            WalletConfig {
                coin: config.mining.active_coin.clone(),
                address: grpc_config.wallet_address.clone(),
                label: Some("gRPC request wallet".to_string()),
            },
        );
        config.mining.active_wallet = wallet_key;
    }

    if !grpc_config.pool_url.is_empty() {
        if grpc_config.pool_url.len() > 256 {
            return Err("pool_url must be at most 256 characters".to_string());
        }
        if grpc_config.pool_port == 0 || grpc_config.pool_port > u16::MAX as u32 {
            return Err("pool_port must be between 1 and 65535".to_string());
        }

        let pool_key = "grpc_request_pool".to_string();
        config.pools.insert(
            pool_key.clone(),
            PoolConfig {
                coin: config.mining.active_coin.clone(),
                url: grpc_config.pool_url.clone(),
                port: grpc_config.pool_port as u16,
                username: None,
                worker_name: non_empty_string(grpc_config.worker_name.clone()),
                password: non_empty_string(grpc_config.password.clone()),
                ssl: grpc_config.pool_url.starts_with("ssl://")
                    || grpc_config.pool_url.starts_with("tls://")
                    || grpc_config.pool_url.starts_with("stratum+ssl://"),
                priority: 0,
            },
        );
        config.mining.active_pool = pool_key;
    } else if !grpc_config.worker_name.is_empty() {
        if let Some(pool) = config.pools.get_mut(&config.mining.active_pool) {
            pool.worker_name = Some(grpc_config.worker_name.clone());
        }
    }

    validate_extra_params(&grpc_config.extra_params)?;
    config.mining.extra_params = grpc_config.extra_params;

    Ok(grpc_config.target_device_ids)
}

fn validate_extra_params(
    extra_params: &HashMap<String, String>,
) -> std::result::Result<(), String> {
    if extra_params.len() > 16 {
        return Err("extra_params may contain at most 16 entries".to_string());
    }

    for (key, value) in extra_params {
        if key.is_empty() || key.len() > 64 {
            return Err("extra_params keys must be 1-64 characters".to_string());
        }
        if !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(
                "extra_params keys may only contain ASCII letters, numbers, dots, underscores, and hyphens"
                    .to_string(),
            );
        }
        if value.len() > 256 {
            return Err(format!(
                "extra_params value for '{key}' must be at most 256 characters"
            ));
        }
        if value.chars().any(char::is_control) {
            return Err(format!(
                "extra_params value for '{key}' must not contain control characters"
            ));
        }
    }

    Ok(())
}

fn infer_coin_from_algorithm(algorithm: &str) -> Option<&'static str> {
    let normalized = algorithm.to_ascii_lowercase();
    match normalized.as_str() {
        "randomx" | "rx/0" | "rx" => Some("monero"),
        "etchash" => Some("ethereum_classic"),
        "ethash" => Some("ethereum"),
        "beamhash" | "beamv3" => Some("beam"),
        _ => None,
    }
}

fn normalize_miner_device_ids(device_ids: &[String], coin: &str) -> Vec<String> {
    if coin == "monero" {
        return device_ids.to_vec();
    }

    device_ids
        .iter()
        .map(|device_id| {
            device_id
                .rsplit_once('_')
                .and_then(|(_, suffix)| suffix.parse::<u32>().ok())
                .map(|index| index.to_string())
                .unwrap_or_else(|| device_id.clone())
        })
        .collect()
}

fn non_empty_string(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn health_component(
    name: impl Into<String>,
    message: impl Into<String>,
) -> health_check_response::ComponentHealth {
    health_check_response::ComponentHealth {
        component_name: name.into(),
        status: health_check_response::HealthStatus::HealthHealthy as i32,
        status_message: message.into(),
        last_check: Some(now_timestamp()),
    }
}

fn command_with_status(
    status: command_response::Status,
    message: impl Into<String>,
    started_at: Instant,
) -> CommandResponse {
    CommandResponse {
        status: status as i32,
        message: message.into(),
        error_details: None,
        timestamp: Some(now_timestamp()),
        execution_duration_ms: elapsed_ms(started_at),
    }
}

fn command_error(
    code: impl Into<String>,
    message: impl Into<String>,
    remediation_steps: &[&str],
    started_at: Instant,
) -> CommandResponse {
    CommandResponse {
        status: command_response::Status::Error as i32,
        message: message.into(),
        error_details: Some(command_response::ErrorDetails {
            error_code: code.into(),
            error_description: "Command could not be completed".to_string(),
            affected_devices: vec![],
            remediation_steps: remediation_steps
                .iter()
                .map(|step| step.to_string())
                .collect(),
        }),
        timestamp: Some(now_timestamp()),
        execution_duration_ms: elapsed_ms(started_at),
    }
}

fn install_miner_success(
    result: crate::miner_installer::MinerInstallResult,
    started_at: Instant,
) -> InstallMinerResponse {
    InstallMinerResponse {
        status: command_response::Status::Success as i32,
        message: format!("Installed {} {}", result.name, result.version),
        installed_miner: Some(InstallMinerResult {
            miner_name: result.name,
            version: result.version,
            executable_path: result.executable_path.display().to_string(),
            executable_sha256: result.executable_sha256,
            source_url: result.source_url,
        }),
        error_details: None,
        timestamp: Some(now_timestamp()),
        execution_duration_ms: elapsed_ms(started_at),
    }
}

fn install_miner_error(
    code: impl Into<String>,
    message: impl Into<String>,
    remediation_steps: &[&str],
    started_at: Instant,
) -> InstallMinerResponse {
    InstallMinerResponse {
        status: command_response::Status::Error as i32,
        message: message.into(),
        installed_miner: None,
        error_details: Some(command_response::ErrorDetails {
            error_code: code.into(),
            error_description: "Miner install could not be completed".to_string(),
            affected_devices: vec![],
            remediation_steps: remediation_steps
                .iter()
                .map(|step| step.to_string())
                .collect(),
        }),
        timestamp: Some(now_timestamp()),
        execution_duration_ms: elapsed_ms(started_at),
    }
}

fn elapsed_ms(started_at: Instant) -> u32 {
    started_at.elapsed().as_millis().min(u32::MAX as u128) as u32
}

fn bounded_timeout(requested_seconds: u32, default_seconds: u64, max_seconds: u64) -> u64 {
    if requested_seconds == 0 {
        default_seconds
    } else {
        (requested_seconds as u64).min(max_seconds)
    }
}

fn now_timestamp() -> Timestamp {
    Timestamp {
        seconds: unix_now() as i64,
        nanos: 0,
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn convert_mining_device_to_device_info(device: MiningDevice) -> DeviceInfo {
    let name_lower = device.name.to_ascii_lowercase();
    let vendor = if name_lower.contains("nvidia")
        || name_lower.contains("geforce")
        || name_lower.contains("rtx")
        || name_lower.contains("gtx")
    {
        device_info::Vendor::Nvidia
    } else if name_lower.contains("amd")
        || name_lower.contains("radeon")
        || name_lower.contains("rx")
    {
        device_info::Vendor::Amd
    } else if name_lower.contains("intel") {
        device_info::Vendor::Intel
    } else {
        device_info::Vendor::Unknown
    };

    let device_type = match device.device_type {
        HardwareDeviceType::NvidiaGpu | HardwareDeviceType::AmdGpu => device_info::DeviceType::Gpu,
        HardwareDeviceType::Cpu => device_info::DeviceType::Cpu,
    };
    let power_limit_watts = parse_power_limit_watts(&device);
    let driver_version = device.driver_version.clone().unwrap_or_default();

    DeviceInfo {
        device_id: device.id,
        name: device.name,
        vendor: vendor as i32,
        device_type: device_type as i32,
        vram_mb: device.memory_mb.min(u32::MAX as u64) as u32,
        core_count: device
            .properties
            .get("core_count")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0),
        driver_version,
        compute_capability: device
            .properties
            .get("compute_capability")
            .cloned()
            .unwrap_or_default(),
        base_clock_mhz: device.metrics.core_clock_mhz.unwrap_or(0),
        memory_clock_mhz: device.metrics.memory_clock_mhz.unwrap_or(0),
        power_limit_watts,
        capabilities: device.supported_algorithms,
    }
}

fn convert_telemetry_to_grpc(telemetry: MinerTelemetry) -> Telemetry {
    let total_shares = telemetry.shares_accepted + telemetry.shares_rejected;
    Telemetry {
        device_id: "device_0".to_string(),
        timestamp: Some(Timestamp {
            seconds: telemetry.timestamp as i64,
            nanos: 0,
        }),
        algorithm: telemetry.algorithm,
        hashrate_mhs: telemetry.hashrate_hs / 1_000_000.0,
        power_watts: telemetry.power_watts.unwrap_or(0.0) as u32,
        temperature_celsius: telemetry.temperature_c.unwrap_or(0.0) as u32,
        fan_speed_percent: telemetry.fan_speed_percent.unwrap_or(0.0) as u32,
        utilization_percent: 0,
        memory_utilization_percent: 0,
        core_clock_mhz: 0,
        memory_clock_mhz: 0,
        shares: Some(ShareStats {
            accepted: telemetry.shares_accepted as u64,
            rejected: telemetry.shares_rejected as u64,
            stale: telemetry.shares_stale as u64,
            acceptance_rate: if total_shares > 0 {
                telemetry.shares_accepted as f32 / total_shares as f32
            } else {
                0.0
            },
            avg_share_time_seconds: 0.0,
            last_share_time: None,
        }),
        device_status: if telemetry.error_message.is_empty() {
            telemetry::DeviceStatus::Mining as i32
        } else {
            telemetry::DeviceStatus::Error as i32
        },
        error_message: telemetry.error_message,
        pool_status: if (telemetry::PoolStatus::Unknown as i32
            ..=telemetry::PoolStatus::Error as i32)
            .contains(&telemetry.pool_status)
        {
            telemetry.pool_status
        } else {
            telemetry::PoolStatus::Unknown as i32
        },
        pool_url: telemetry.pool_url,
    }
}

fn convert_overclock_profile(profile: &crate::overclocking::OverclockProfile) -> OverclockProfile {
    OverclockProfile {
        algorithm: profile.algorithm.clone(),
        core_clock_offset: profile.core_clock_offset,
        memory_clock_offset: profile.memory_clock_offset,
        power_limit_watts: profile.power_limit_watts,
        temperature_limit_c: profile.temperature_limit_c,
        fan_speed_percent: profile.fan_speed_percent,
        enabled: profile.enabled,
        name: profile.name.clone(),
    }
}

fn hardware_defaults_from_device(device: &MiningDevice) -> HardwareDefaults {
    HardwareDefaults {
        device_id: device.id.clone(),
        core_clock_mhz: device.metrics.core_clock_mhz.unwrap_or(0),
        memory_clock_mhz: device.metrics.memory_clock_mhz.unwrap_or(0),
        power_limit_watts: parse_power_limit_watts(device),
        temperature_limit_c: device
            .metrics
            .temperature_c
            .map(|temperature| temperature as u32)
            .unwrap_or(0),
        fan_speed_percent: device
            .metrics
            .fan_speed_percent
            .map(|fan_speed| fan_speed as u32)
            .unwrap_or(0),
    }
}

fn parse_power_limit_watts(device: &MiningDevice) -> u32 {
    device
        .properties
        .get("power_limit_watts")
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| if value > 2_000 { value / 1_000 } else { value })
        .unwrap_or(0)
}

pub struct GrpcServer {
    state: Arc<DaemonState>,
    config: GrpcConfig,
}

impl GrpcServer {
    pub fn new(state: Arc<DaemonState>, config: GrpcConfig) -> Self {
        Self { state, config }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        if !self.config.enabled {
            info!("gRPC server disabled in configuration");
            return Ok(());
        }

        let addr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse()
            .map_err(|error| anyhow!("Invalid gRPC bind address: {error}"))?;

        let service = BunkerMinerDaemonImpl::new(self.state.clone());
        let server = BunkerMinerDaemonServer::new(service);

        info!("Starting gRPC server on {}", addr);

        if self.config.tls_enabled {
            warn!("TLS configuration not yet implemented, falling back to insecure connection");
        }

        Server::builder()
            .add_service(server)
            .serve(addr)
            .await
            .map_err(|error| anyhow!("gRPC server error: {error}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_redaction_keeps_only_edges() {
        assert_eq!(
            redact_wallet_address("42ey1afDFnn4886T7196doS9GPMzexD9gXpsZJDwVjeRVdFCSoHnv7KPbBeGpzJBzHRCAs9UxqeoyFQMYbqSWYTfJJQAWDm"),
            "42ey1a...JQAWDm"
        );
        assert_eq!(redact_wallet_address("short"), "<redacted>");
        assert_eq!(redact_wallet_address(""), "");
    }

    #[test]
    fn idle_mining_state_uses_config_summary_without_raw_wallet() {
        let mut config = Config::default();
        config.mining.active_coin = "monero".to_string();
        config.mining.active_wallet = "monero_main".to_string();
        config.mining.active_pool = "minexmr".to_string();
        config.wallets.get_mut("monero_main").unwrap().address =
            "42ey1afDFnn4886T7196doS9GPMzexD9gXpsZJDwVjeRVdFCSoHnv7KPbBeGpzJBzHRCAs9UxqeoyFQMYbqSWYTfJJQAWDm".to_string();

        let response = mining_state_response(MiningStateResponseInput {
            state: mining_state_response::LifecycleState::MiningLifecycleStateIdle,
            status_message: "No active mining process".to_string(),
            miner_name: None,
            config: &config,
            device_ids: &[],
            restart_count: 0,
            latest_telemetry: None,
            error_details: None,
        });

        assert_eq!(
            response.state,
            mining_state_response::LifecycleState::MiningLifecycleStateIdle as i32
        );
        assert_eq!(response.active_coin, "monero");
        assert_eq!(response.algorithm, "randomx");
        assert_eq!(response.pool_url, "pool.minexmr.com:4444");
        assert_eq!(response.wallet_address_redacted, "42ey1a...JQAWDm");
        assert!(!response.wallet_address_redacted.contains("VjeRVdFC"));
        assert!(!response.telemetry_available);
    }
}
