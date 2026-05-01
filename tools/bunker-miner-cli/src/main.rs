use anyhow::{anyhow, bail, Context, Result};
use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tracing::{debug, info};

include!("generated/bunker.daemon.v1.rs");

use bunker_miner_daemon_client::BunkerMinerDaemonClient;

#[derive(Debug, Parser)]
#[command(
    name = "bunker-miner-cli",
    version,
    author,
    about = "Command-line control surface for the BUNKER MINER daemon"
)]
struct Cli {
    #[arg(
        long,
        short = 'a',
        value_name = "ADDRESS",
        default_value = "http://127.0.0.1:50051"
    )]
    address: String,

    #[arg(long, short = 't', value_name = "SECONDS", default_value_t = 30)]
    timeout: u64,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Info,
    Health,
    Start(StartArgs),
    Stop(StopArgs),
    Watch(WatchArgs),
    Profitability,
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
struct StartArgs {
    #[arg(long, short = 'A', value_name = "ALGORITHM")]
    algorithm: String,

    #[arg(long, short = 'p', value_name = "HOST:PORT")]
    pool: String,

    #[arg(long, short = 'w', value_name = "ADDRESS")]
    wallet: String,

    #[arg(long, value_name = "NAME", default_value = "bunker-miner-cli")]
    worker: String,

    #[arg(long, value_name = "PASSWORD", default_value = "x")]
    password: String,

    #[arg(long = "device", value_name = "ID", action = ArgAction::Append)]
    devices: Vec<String>,

    #[arg(long, value_name = "0.0-1.0", default_value_t = 1.0)]
    intensity: f32,

    #[arg(long = "param", value_name = "KEY=VALUE", action = ArgAction::Append)]
    params: Vec<String>,

    #[arg(long = "timeout-seconds", value_name = "SECONDS", default_value_t = 30)]
    timeout_seconds: u32,

    #[arg(
        long = "keep-existing",
        action = ArgAction::SetFalse,
        default_value_t = true
    )]
    stop_existing: bool,
}

#[derive(Debug, Args)]
struct StopArgs {
    #[arg(long = "device", value_name = "ID", action = ArgAction::Append)]
    devices: Vec<String>,

    #[arg(long, short = 'f', action = ArgAction::SetTrue)]
    force: bool,

    #[arg(long = "timeout-seconds", value_name = "SECONDS", default_value_t = 30)]
    timeout_seconds: u32,
}

#[derive(Debug, Args)]
struct WatchArgs {
    #[arg(long, short = 'i', value_name = "SECONDS", default_value_t = 1)]
    interval: u64,
}

#[derive(Debug, Args)]
struct ConfigArgs {
    #[command(subcommand)]
    command: Option<ConfigCommands>,
}

#[derive(Debug, Subcommand)]
enum ConfigCommands {
    Get {
        #[arg(long, short = 's', value_name = "SECTION")]
        section: Option<String>,
    },
    Set {
        #[arg(long, short = 'c', value_name = "JSON", conflicts_with = "file")]
        config: Option<String>,

        #[arg(long, short = 'f', value_name = "PATH", conflicts_with = "config")]
        file: Option<PathBuf>,

        #[arg(long = "validate-only", action = ArgAction::SetTrue)]
        validate_only: bool,

        #[arg(
            long = "no-restart",
            action = ArgAction::SetFalse,
            default_value_t = true
        )]
        restart_services: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };

    info!("Connecting to daemon at {}", cli.address);
    let channel = Channel::from_shared(cli.address.clone())?
        .timeout(Duration::from_secs(cli.timeout))
        .connect()
        .await
        .with_context(|| format!("failed to connect to daemon at {}", cli.address))?;

    let mut client = BunkerMinerDaemonClient::new(channel);
    debug!("Connected to daemon");

    match command {
        Commands::Info => info_command(&mut client).await?,
        Commands::Health => health_command(&mut client).await?,
        Commands::Start(args) => start_command(&mut client, args).await?,
        Commands::Stop(args) => stop_command(&mut client, args).await?,
        Commands::Watch(args) => watch_command(&mut client, args).await?,
        Commands::Profitability => profitability_command(&mut client).await?,
        Commands::Config(args) => config_command(&mut client, args).await?,
    }

    Ok(())
}

async fn info_command(client: &mut BunkerMinerDaemonClient<Channel>) -> Result<()> {
    println!("BUNKER MINER - System Information");
    println!("=================================");

    let response = client.get_system_info(()).await?.into_inner();

    if let Some(system_info) = response.system_info {
        println!();
        println!("System");
        println!("  OS: {} {}", system_info.os_name, system_info.os_version);
        println!(
            "  CPU: {} ({} cores, {} threads)",
            system_info.cpu_name, system_info.cpu_cores, system_info.cpu_threads
        );
        println!(
            "  Memory: {} GB total, {} GB available",
            system_info.total_memory_gb, system_info.available_memory_gb
        );
        println!("  Uptime: {} seconds", system_info.uptime_seconds);
    }

    if let Some(version_info) = response.version_info {
        println!();
        println!("Version");
        println!("  Daemon: {}", version_info.daemon_version);
        println!("  API: {}", version_info.api_version);
        println!("  Build: {}", version_info.build_timestamp);
        println!("  Git: {}", version_info.git_commit);
    }

    println!();
    println!("Mining Devices ({})", response.devices.len());
    for (index, device) in response.devices.iter().enumerate() {
        println!();
        println!("{}. {} ({})", index + 1, device.name, device.device_id);
        println!("   Vendor: {}", vendor_label(device.vendor));
        println!("   Type: {}", device_type_label(device.device_type));

        if device.vram_mb > 0 {
            println!("   VRAM: {:.1} GB", device.vram_mb as f64 / 1024.0);
        }
        if device.core_count > 0 {
            println!("   Cores: {}", device.core_count);
        }
        if !device.driver_version.is_empty() {
            println!("   Driver: {}", device.driver_version);
        }
        if !device.compute_capability.is_empty() {
            println!("   Compute: {}", device.compute_capability);
        }
        if !device.capabilities.is_empty() {
            println!("   Capabilities: {}", device.capabilities.join(", "));
        }
    }

    if let Some(timestamp) = format_timestamp(response.timestamp.as_ref()) {
        println!();
        println!("Response time: {timestamp}");
    }

    Ok(())
}

async fn health_command(client: &mut BunkerMinerDaemonClient<Channel>) -> Result<()> {
    println!("BUNKER MINER - Health Status");
    println!("============================");

    let response = client.health_check(()).await?.into_inner();

    println!();
    println!("Overall Status: {}", health_status_label(response.status));
    println!("Uptime: {} seconds", response.uptime_seconds);

    println!();
    println!("Components");
    for component in response.component_health {
        println!(
            "  {}: {} ({})",
            component.component_name,
            health_status_label(component.status),
            component.status_message
        );
    }

    if let Some(timestamp) = format_timestamp(response.timestamp.as_ref()) {
        println!();
        println!("Check time: {timestamp}");
    }

    Ok(())
}

async fn start_command(
    client: &mut BunkerMinerDaemonClient<Channel>,
    args: StartArgs,
) -> Result<()> {
    let request = build_start_request(args)?;

    println!("BUNKER MINER - Start Mining");
    println!("===========================");

    let response = client.start_mining(request).await?.into_inner();
    print_command_response(&response);

    Ok(())
}

async fn stop_command(client: &mut BunkerMinerDaemonClient<Channel>, args: StopArgs) -> Result<()> {
    println!("BUNKER MINER - Stop Mining");
    println!("==========================");

    let request = StopMiningRequest {
        device_ids: args.devices,
        force_stop: args.force,
        timeout_seconds: bounded_timeout(args.timeout_seconds, 30, 60),
    };

    let response = client.stop_mining(request).await?.into_inner();
    print_command_response(&response);

    Ok(())
}

async fn watch_command(
    client: &mut BunkerMinerDaemonClient<Channel>,
    args: WatchArgs,
) -> Result<()> {
    println!("BUNKER MINER - Live Telemetry Stream");
    println!("====================================");
    println!("Press Ctrl+C to stop watching");
    println!();

    let mut stream = client.stream_telemetry(()).await?.into_inner();
    let min_interval = Duration::from_secs(args.interval.max(1));
    let mut last_print = Instant::now()
        .checked_sub(min_interval)
        .unwrap_or_else(Instant::now);

    while let Some(telemetry) = stream.next().await {
        let telemetry = telemetry?;
        if last_print.elapsed() < min_interval {
            continue;
        }
        last_print = Instant::now();

        let timestamp =
            format_timestamp(telemetry.timestamp.as_ref()).unwrap_or_else(|| "unknown".to_string());
        let shares = telemetry
            .shares
            .as_ref()
            .map(|shares| {
                format!(
                    "shares accepted={} rejected={} stale={} acceptance={:.1}%",
                    shares.accepted,
                    shares.rejected,
                    shares.stale,
                    shares.acceptance_rate * 100.0
                )
            })
            .unwrap_or_else(|| "shares unavailable".to_string());

        println!(
            "[{}] device={} status={} algorithm={} hashrate={:.2} MH/s power={}W temp={}C fan={} util={} mem_util={} {} pool={}",
            timestamp,
            telemetry.device_id,
            device_status_label(telemetry.device_status),
            telemetry.algorithm,
            telemetry.hashrate_mhs,
            telemetry.power_watts,
            telemetry.temperature_celsius,
            telemetry.fan_speed_percent,
            telemetry.utilization_percent,
            telemetry.memory_utilization_percent,
            shares,
            telemetry.pool_url,
        );

        io::stdout().flush().ok();
    }

    println!();
    println!("Telemetry stream ended");
    Ok(())
}

async fn profitability_command(client: &mut BunkerMinerDaemonClient<Channel>) -> Result<()> {
    println!("BUNKER MINER - Profitability Information");
    println!("========================================");

    let response = client.get_profitability(()).await?.into_inner();

    if response.profitability_info.is_empty() {
        println!();
        println!("No profitability data available from the daemon.");
    } else {
        println!();
        println!("Recommended Algorithm: {}", response.recommended_algorithm);

        for info in response.profitability_info {
            println!();
            println!("{} ({})", info.algorithm, info.coin);
            println!("  Revenue: EUR {:.4}/day", info.revenue_eur_day);
            println!("  Cost: EUR {:.4}/day", info.cost_eur_day);
            println!("  Profit: EUR {:.4}/day", info.profit_eur_day);
            println!("  Coin Price: EUR {:.4}", info.coin_price_eur);
            println!("  Confidence: {:.1}%", info.confidence * 100.0);
            println!("  Source: {}", info.data_source);
        }
    }

    if let Some(timestamp) = format_timestamp(response.timestamp.as_ref()) {
        println!();
        println!("Data time: {timestamp}");
    }
    println!("Data age: {} seconds", response.data_age_seconds);

    Ok(())
}

async fn config_command(
    client: &mut BunkerMinerDaemonClient<Channel>,
    args: ConfigArgs,
) -> Result<()> {
    match args.command {
        Some(ConfigCommands::Get { section }) => {
            let section = section.unwrap_or_default();
            let response = client
                .get_config(GetConfigRequest {
                    section: section.clone(),
                })
                .await?
                .into_inner();

            println!("BUNKER MINER - Configuration");
            println!("============================");
            if !section.is_empty() {
                println!("Section: {section}");
            }
            println!();

            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&response.config_json)
            {
                println!("{}", serde_json::to_string_pretty(&json_value)?);
            } else {
                println!("{}", response.config_json);
            }

            println!();
            println!("Version: {}", response.config_version);
        }
        Some(ConfigCommands::Set {
            config,
            file,
            validate_only,
            restart_services,
        }) => {
            let config_json = match (config, file) {
                (Some(config), None) => config,
                (None, Some(path)) => fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?,
                _ => bail!("provide either --config JSON or --file PATH"),
            };

            serde_json::from_str::<serde_json::Value>(&config_json)
                .context("configuration payload must be valid JSON")?;

            let response = client
                .set_config(SetConfigRequest {
                    config_json,
                    validate_only,
                    restart_services,
                })
                .await?
                .into_inner();

            println!("BUNKER MINER - Set Configuration");
            println!("================================");
            println!();
            println!("Result: {}", command_status_label(response.status));

            if !response.validation_errors.is_empty() {
                println!();
                println!("Validation Errors");
                for (index, error) in response.validation_errors.iter().enumerate() {
                    println!("  {}. {}", index + 1, error);
                }
            }

            if !response.services_requiring_restart.is_empty() {
                println!();
                println!("Services requiring restart");
                for service in response.services_requiring_restart {
                    println!("  - {service}");
                }
            }
        }
        None => {
            println!("BUNKER MINER - Configuration Management");
            println!("=======================================");
            println!("Use 'config get' or 'config set --help'.");
        }
    }

    Ok(())
}

fn build_start_request(args: StartArgs) -> Result<StartMiningRequest> {
    let algorithm = require_non_empty("algorithm", args.algorithm)?;
    let wallet_address = require_non_empty("wallet", args.wallet)?;
    let worker_name = require_non_empty("worker", args.worker)?;
    let (pool_url, pool_port) = parse_pool_endpoint(&args.pool)?;

    if !(0.0..=1.0).contains(&args.intensity) {
        bail!("intensity must be between 0.0 and 1.0");
    }

    Ok(StartMiningRequest {
        config: Some(MiningConfig {
            algorithm,
            pool_url,
            pool_port,
            worker_name,
            wallet_address,
            password: args.password,
            target_device_ids: args.devices,
            intensity: args.intensity,
            extra_params: parse_extra_params(args.params)?,
        }),
        stop_existing: args.stop_existing,
        timeout_seconds: bounded_timeout(args.timeout_seconds, 30, 300),
    })
}

fn parse_pool_endpoint(pool: &str) -> Result<(String, u32)> {
    let pool = pool.trim().trim_end_matches('/');
    if pool.is_empty() {
        bail!("pool endpoint must not be empty");
    }

    let host_start = pool.find("://").map_or(0, |index| index + 3);
    let endpoint = &pool[host_start..];
    let colon_index = endpoint.rfind(':').ok_or_else(|| {
        anyhow!("pool endpoint must include a port, for example pool.example:3333")
    })?;

    let port = &endpoint[colon_index + 1..];
    if port.is_empty() || port.contains('/') {
        bail!("pool endpoint port is invalid");
    }

    let port = port
        .parse::<u32>()
        .context("pool endpoint port must be a number")?;
    if port == 0 || port > u16::MAX as u32 {
        bail!("pool endpoint port must be between 1 and 65535");
    }

    let host = &endpoint[..colon_index];
    if host.is_empty() {
        bail!("pool endpoint host must not be empty");
    }

    Ok((format!("{}{}", &pool[..host_start], host), port))
}

fn parse_extra_params(params: Vec<String>) -> Result<HashMap<String, String>> {
    let mut parsed = HashMap::new();
    for param in params {
        let (key, value) = param
            .split_once('=')
            .ok_or_else(|| anyhow!("--param values must use KEY=VALUE format"))?;
        let key = require_non_empty("param key", key.to_string())?;
        parsed.insert(key, value.to_string());
    }
    Ok(parsed)
}

fn require_non_empty(field: &str, value: String) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{field} must not be empty");
    }
    Ok(trimmed.to_string())
}

fn bounded_timeout(value: u32, default_value: u32, max_value: u32) -> u32 {
    if value == 0 {
        default_value
    } else {
        value.min(max_value)
    }
}

fn print_command_response(response: &CommandResponse) {
    println!();
    println!("Result: {}", command_status_label(response.status));
    println!("Message: {}", response.message);
    println!("Execution time: {}ms", response.execution_duration_ms);

    if let Some(error_details) = response.error_details.as_ref() {
        println!();
        println!("Error Details");
        println!("  Code: {}", error_details.error_code);
        println!("  Description: {}", error_details.error_description);

        if !error_details.affected_devices.is_empty() {
            println!(
                "  Affected devices: {}",
                error_details.affected_devices.join(", ")
            );
        }

        if !error_details.remediation_steps.is_empty() {
            println!("  Remediation");
            for (index, step) in error_details.remediation_steps.iter().enumerate() {
                println!("    {}. {}", index + 1, step);
            }
        }
    }

    if let Some(timestamp) = format_timestamp(response.timestamp.as_ref()) {
        println!("Timestamp: {timestamp}");
    }
}

fn vendor_label(value: i32) -> &'static str {
    match enum_value(value, device_info::Vendor::Unknown) {
        device_info::Vendor::Nvidia => "NVIDIA",
        device_info::Vendor::Amd => "AMD",
        device_info::Vendor::Intel => "Intel",
        device_info::Vendor::Other => "Other",
        device_info::Vendor::Unknown => "Unknown",
    }
}

fn device_type_label(value: i32) -> &'static str {
    match enum_value(value, device_info::DeviceType::Unknown) {
        device_info::DeviceType::Gpu => "GPU",
        device_info::DeviceType::Cpu => "CPU",
        device_info::DeviceType::Asic => "ASIC",
        device_info::DeviceType::Fpga => "FPGA",
        device_info::DeviceType::Unknown => "Unknown",
    }
}

fn device_status_label(value: i32) -> &'static str {
    match enum_value(value, telemetry::DeviceStatus::Unknown) {
        telemetry::DeviceStatus::Idle => "idle",
        telemetry::DeviceStatus::Mining => "mining",
        telemetry::DeviceStatus::Error => "error",
        telemetry::DeviceStatus::ThermalThrottling => "thermal-throttling",
        telemetry::DeviceStatus::PowerThrottling => "power-throttling",
        telemetry::DeviceStatus::Offline => "offline",
        telemetry::DeviceStatus::Unknown => "unknown",
    }
}

fn command_status_label(value: i32) -> &'static str {
    match enum_value(value, command_response::Status::Unknown) {
        command_response::Status::Success => "SUCCESS",
        command_response::Status::Error => "ERROR",
        command_response::Status::Timeout => "TIMEOUT",
        command_response::Status::PartialSuccess => "PARTIAL SUCCESS",
        command_response::Status::Unknown => "UNKNOWN",
    }
}

fn health_status_label(value: i32) -> &'static str {
    match enum_value(value, health_check_response::HealthStatus::HealthUnknown) {
        health_check_response::HealthStatus::HealthHealthy => "HEALTHY",
        health_check_response::HealthStatus::HealthDegraded => "DEGRADED",
        health_check_response::HealthStatus::HealthUnhealthy => "UNHEALTHY",
        health_check_response::HealthStatus::HealthUnknown => "UNKNOWN",
    }
}

fn enum_value<T>(value: i32, default_value: T) -> T
where
    T: TryFrom<i32>,
{
    T::try_from(value).unwrap_or(default_value)
}

fn format_timestamp(timestamp: Option<&prost_types::Timestamp>) -> Option<String> {
    let timestamp = timestamp?;
    let nanos = u32::try_from(timestamp.nanos).ok()?;
    chrono::DateTime::from_timestamp(timestamp.seconds, nanos)
        .map(|datetime| datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pool_endpoint_accepts_host_port() {
        let (host, port) = parse_pool_endpoint("pool.example.com:3333").unwrap();

        assert_eq!(host, "pool.example.com");
        assert_eq!(port, 3333);
    }

    #[test]
    fn parse_pool_endpoint_accepts_scheme_host_port() {
        let (host, port) = parse_pool_endpoint("stratum+tcp://pool.example.com:4444").unwrap();

        assert_eq!(host, "stratum+tcp://pool.example.com");
        assert_eq!(port, 4444);
    }

    #[test]
    fn parse_pool_endpoint_rejects_missing_port() {
        let error = parse_pool_endpoint("pool.example.com").unwrap_err();

        assert!(error.to_string().contains("must include a port"));
    }

    #[test]
    fn parse_extra_params_requires_key_value_pairs() {
        let params = parse_extra_params(vec!["rig=alpha".to_string()]).unwrap();

        assert_eq!(params.get("rig").map(String::as_str), Some("alpha"));
        assert!(parse_extra_params(vec!["rig".to_string()]).is_err());
    }
}
