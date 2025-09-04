use anyhow::{anyhow, Result};
use clap::{Arg, Command};
use std::time::Duration;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tracing::{debug, error, info};

// Include the generated gRPC client code
include!("generated/bunker.daemon.v1.rs");

use bunker_miner_daemon_client::BunkerMinerDaemonClient;
use google::protobuf::Empty;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let matches = Command::new("bunker-miner-cli")
        .version("0.1.0")
        .author("Emilian Cristea <emilian@bunkercorpo.com>")
        .about("BUNKER MINER CLI - Command-line interface for the BUNKER MINER daemon")
        .arg(
            Arg::new("address")
                .long("address")
                .short('a')
                .help("Daemon gRPC server address")
                .value_name("ADDRESS")
                .default_value("http://127.0.0.1:50051")
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .short('t')
                .help("Request timeout in seconds")
                .value_name("SECONDS")
                .default_value("30")
        )
        .subcommand(
            Command::new("info")
                .about("Get system and device information")
        )
        .subcommand(
            Command::new("health")
                .about("Check daemon health status")
        )
        .subcommand(
            Command::new("start")
                .about("Start mining operations")
                .arg(
                    Arg::new("algorithm")
                        .long("algorithm")
                        .short('A')
                        .help("Mining algorithm")
                        .value_name("ALGORITHM")
                )
                .arg(
                    Arg::new("pool")
                        .long("pool")
                        .short('p')
                        .help("Mining pool URL")
                        .value_name("URL")
                )
                .arg(
                    Arg::new("wallet")
                        .long("wallet")
                        .short('w')
                        .help("Wallet address")
                        .value_name("ADDRESS")
                )
                .arg(
                    Arg::new("worker")
                        .long("worker")
                        .help("Worker name")
                        .value_name("NAME")
                        .default_value("bunker-miner-cli")
                )
        )
        .subcommand(
            Command::new("stop")
                .about("Stop mining operations")
                .arg(
                    Arg::new("force")
                        .long("force")
                        .short('f')
                        .help("Force stop mining processes")
                        .action(clap::ArgAction::SetTrue)
                )
        )
        .subcommand(
            Command::new("watch")
                .about("Watch real-time mining telemetry")
                .arg(
                    Arg::new("interval")
                        .long("interval")
                        .short('i')
                        .help("Update interval in seconds")
                        .value_name("SECONDS")
                        .default_value("5")
                )
        )
        .subcommand(
            Command::new("profitability")
                .about("Get profitability information")
        )
        .subcommand(
            Command::new("config")
                .about("Configuration management")
                .subcommand(
                    Command::new("get")
                        .about("Get configuration")
                        .arg(
                            Arg::new("section")
                                .long("section")
                                .short('s')
                                .help("Configuration section")
                                .value_name("SECTION")
                        )
                )
                .subcommand(
                    Command::new("set")
                        .about("Set configuration")
                        .arg(
                            Arg::new("config")
                                .long("config")
                                .short('c')
                                .help("Configuration JSON")
                                .value_name("JSON")
                                .required(true)
                        )
                        .arg(
                            Arg::new("validate-only")
                                .long("validate-only")
                                .help("Only validate, don't apply")
                                .action(clap::ArgAction::SetTrue)
                        )
                )
        )
        .get_matches();

    let address = matches.get_one::<String>("address").unwrap();
    let timeout_secs: u64 = matches.get_one::<String>("timeout").unwrap().parse()
        .map_err(|_| anyhow!("Invalid timeout value"))?;

    info!("Connecting to daemon at {}", address);

    // Create gRPC client with timeout
    let channel = Channel::from_shared(address.clone())?
        .timeout(Duration::from_secs(timeout_secs))
        .connect()
        .await?;
    
    let mut client = BunkerMinerDaemonClient::new(channel);

    debug!("Connected to daemon, executing command");

    match matches.subcommand() {
        Some(("info", _)) => {
            info_command(&mut client).await?;
        }
        Some(("health", _)) => {
            health_command(&mut client).await?;
        }
        Some(("start", sub_matches)) => {
            start_command(&mut client, sub_matches).await?;
        }
        Some(("stop", sub_matches)) => {
            stop_command(&mut client, sub_matches).await?;
        }
        Some(("watch", sub_matches)) => {
            watch_command(&mut client, sub_matches).await?;
        }
        Some(("profitability", _)) => {
            profitability_command(&mut client).await?;
        }
        Some(("config", sub_matches)) => {
            config_command(&mut client, sub_matches).await?;
        }
        _ => {
            println!("BUNKER MINER CLI v0.1.0");
            println!("Use --help to see available commands");
        }
    }

    Ok(())
}

async fn info_command(client: &mut BunkerMinerDaemonClient<Channel>) -> Result<()> {
    println!("BUNKER MINER - System Information");
    println!("=================================");

    let response = client
        .get_system_info(Empty {})
        .await?
        .into_inner();

    // Display system information
    if let Some(system_info) = response.system_info {
        println!("\n📋 System Information:");
        println!("  OS: {} {}", system_info.os_name, system_info.os_version);
        println!("  CPU: {} ({} cores, {} threads)", 
                 system_info.cpu_name, 
                 system_info.cpu_cores, 
                 system_info.cpu_threads);
        println!("  Memory: {:.1} GB total, {:.1} GB available", 
                 system_info.total_memory_gb, 
                 system_info.available_memory_gb);
        println!("  Uptime: {} seconds", system_info.uptime_seconds);
    }

    // Display version information
    if let Some(version_info) = response.version_info {
        println!("\n📦 Version Information:");
        println!("  Daemon: {}", version_info.daemon_version);
        println!("  API: {}", version_info.api_version);
        println!("  Build: {}", version_info.build_timestamp);
        println!("  Git: {}", version_info.git_commit);
    }

    // Display devices
    println!("\n🔧 Mining Devices ({}):", response.devices.len());
    for (i, device) in response.devices.iter().enumerate() {
        println!("\n{}. {} (ID: {})", i + 1, device.name, device.device_id);
        
        let vendor_name = match device_info::Vendor::from_i32(device.vendor) {
            Some(device_info::Vendor::VendorNvidia) => "NVIDIA",
            Some(device_info::Vendor::VendorAmd) => "AMD", 
            Some(device_info::Vendor::VendorIntel) => "Intel",
            _ => "Unknown",
        };
        
        let device_type = match device_info::DeviceType::from_i32(device.device_type) {
            Some(device_info::DeviceType::DeviceTypeGpu) => "GPU",
            Some(device_info::DeviceType::DeviceTypeCpu) => "CPU",
            Some(device_info::DeviceType::DeviceTypeAsic) => "ASIC",
            Some(device_info::DeviceType::DeviceTypeFpga) => "FPGA",
            _ => "Unknown",
        };
        
        println!("   Vendor: {}", vendor_name);
        println!("   Type: {}", device_type);
        
        if device.vram_mb > 0 {
            println!("   VRAM: {:.1} GB", device.vram_mb as f64 / 1024.0);
        }
        
        if device.core_count > 0 {
            println!("   Cores: {}", device.core_count);
        }
        
        if !device.driver_version.is_empty() {
            println!("   Driver: {}", device.driver_version);
        }
        
        if !device.capabilities.is_empty() {
            println!("   Capabilities: {}", device.capabilities.join(", "));
        }
    }

    if let Some(timestamp) = response.timestamp {
        let dt = chrono::DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32);
        if let Some(dt) = dt {
            println!("\n🕒 Response time: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    }

    Ok(())
}

async fn health_command(client: &mut BunkerMinerDaemonClient<Channel>) -> Result<()> {
    println!("BUNKER MINER - Health Status");
    println!("============================");

    let response = client
        .health_check(Empty {})
        .await?
        .into_inner();

    let overall_status = match health_check_response::HealthStatus::from_i32(response.status) {
        Some(health_check_response::HealthStatus::HealthHealthy) => "🟢 HEALTHY",
        Some(health_check_response::HealthStatus::HealthDegraded) => "🟡 DEGRADED",
        Some(health_check_response::HealthStatus::HealthUnhealthy) => "🔴 UNHEALTHY",
        _ => "❓ UNKNOWN",
    };

    println!("\n📊 Overall Status: {}", overall_status);
    println!("⏱️  Uptime: {} seconds", response.uptime_seconds);

    println!("\n🔍 Component Health:");
    for component in response.component_health {
        let status_icon = match health_check_response::HealthStatus::from_i32(component.status) {
            Some(health_check_response::HealthStatus::HealthHealthy) => "✅",
            Some(health_check_response::HealthStatus::HealthDegraded) => "⚠️",
            Some(health_check_response::HealthStatus::HealthUnhealthy) => "❌",
            _ => "❓",
        };

        println!("  {} {}: {}", 
                 status_icon, 
                 component.component_name, 
                 component.status_message);
    }

    if let Some(timestamp) = response.timestamp {
        let dt = chrono::DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32);
        if let Some(dt) = dt {
            println!("\n🕒 Check time: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    }

    Ok(())
}

async fn start_command(
    client: &mut BunkerMinerDaemonClient<Channel>,
    matches: &clap::ArgMatches,
) -> Result<()> {
    println!("BUNKER MINER - Start Mining");
    println!("==========================");

    // TODO: Build proper MiningConfig from arguments
    let request = StartMiningRequest {
        config: None, // Will be implemented with actual mining config
        stop_existing: true,
        timeout_seconds: 30,
    };

    let response = client.start_mining(request).await?.into_inner();

    let status = match command_response::Status::from_i32(response.status) {
        Some(command_response::Status::StatusSuccess) => "✅ SUCCESS",
        Some(command_response::Status::StatusError) => "❌ ERROR",
        Some(command_response::Status::StatusTimeout) => "⏱️ TIMEOUT",
        Some(command_response::Status::StatusPartialSuccess) => "⚠️ PARTIAL SUCCESS",
        _ => "❓ UNKNOWN",
    };

    println!("\n📊 Result: {}", status);
    println!("💬 Message: {}", response.message);

    if let Some(error_details) = response.error_details {
        println!("\n❌ Error Details:");
        println!("  Code: {}", error_details.error_code);
        println!("  Description: {}", error_details.error_description);
        
        if !error_details.affected_devices.is_empty() {
            println!("  Affected devices: {}", error_details.affected_devices.join(", "));
        }
        
        if !error_details.remediation_steps.is_empty() {
            println!("  Remediation:");
            for (i, step) in error_details.remediation_steps.iter().enumerate() {
                println!("    {}. {}", i + 1, step);
            }
        }
    }

    println!("⏱️ Execution time: {}ms", response.execution_duration_ms);

    Ok(())
}

async fn stop_command(
    client: &mut BunkerMinerDaemonClient<Channel>,
    matches: &clap::ArgMatches,
) -> Result<()> {
    println!("BUNKER MINER - Stop Mining");
    println!("=========================");

    let force_stop = matches.get_flag("force");

    let request = StopMiningRequest {
        device_ids: vec![], // Empty = stop all
        force_stop,
        timeout_seconds: 30,
    };

    let response = client.stop_mining(request).await?.into_inner();

    let status = match command_response::Status::from_i32(response.status) {
        Some(command_response::Status::StatusSuccess) => "✅ SUCCESS",
        Some(command_response::Status::StatusError) => "❌ ERROR", 
        Some(command_response::Status::StatusTimeout) => "⏱️ TIMEOUT",
        Some(command_response::Status::StatusPartialSuccess) => "⚠️ PARTIAL SUCCESS",
        _ => "❓ UNKNOWN",
    };

    println!("\n📊 Result: {}", status);
    println!("💬 Message: {}", response.message);
    println!("⏱️ Execution time: {}ms", response.execution_duration_ms);

    Ok(())
}

async fn watch_command(
    client: &mut BunkerMinerDaemonClient<Channel>,
    matches: &clap::ArgMatches,
) -> Result<()> {
    println!("BUNKER MINER - Live Telemetry Stream");
    println!("====================================");
    println!("💡 Press Ctrl+C to stop watching\n");

    let mut stream = client
        .stream_telemetry(Empty {})
        .await?
        .into_inner();

    while let Some(telemetry) = stream.next().await {
        let telemetry = telemetry?;

        let timestamp = if let Some(ts) = telemetry.timestamp {
            chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                .map(|dt| dt.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        };

        let device_status = match telemetry::DeviceStatus::from_i32(telemetry.device_status) {
            Some(telemetry::DeviceStatus::DeviceStatusMining) => "⛏️ MINING",
            Some(telemetry::DeviceStatus::DeviceStatusIdle) => "💤 IDLE",
            Some(telemetry::DeviceStatus::DeviceStatusError) => "❌ ERROR",
            Some(telemetry::DeviceStatus::DeviceStatusThermalThrottling) => "🔥 THERMAL",
            Some(telemetry::DeviceStatus::DeviceStatusPowerThrottling) => "⚡ POWER",
            Some(telemetry::DeviceStatus::DeviceStatusOffline) => "📴 OFFLINE",
            _ => "❓ UNKNOWN",
        };

        print!("\r[{}] Device: {} | Status: {} | ", 
               timestamp, 
               telemetry.device_id, 
               device_status);
        print!("Hashrate: {:.2} MH/s | Power: {}W | Temp: {}°C | ",
               telemetry.hashrate_mhs,
               telemetry.power_watts,
               telemetry.temperature_celsius);

        if let Some(shares) = telemetry.shares {
            print!("Shares: A:{} R:{} ({:.1}%)",
                   shares.accepted,
                   shares.rejected,
                   shares.acceptance_rate * 100.0);
        }

        print!("                    "); // Clear any remaining text
        std::io::Write::flush(&mut std::io::stdout()).ok();

        // Small delay to prevent overwhelming the terminal
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!("\n\n📡 Telemetry stream ended");
    Ok(())
}

async fn profitability_command(client: &mut BunkerMinerDaemonClient<Channel>) -> Result<()> {
    println!("BUNKER MINER - Profitability Information");
    println!("=======================================");

    let response = client
        .get_profitability(Empty {})
        .await?
        .into_inner();

    if response.profitability_info.is_empty() {
        println!("📊 No profitability data available yet");
        println!("💡 This feature will be implemented in a future release");
    } else {
        println!("\n💰 Recommended Algorithm: {}", response.recommended_algorithm);
        
        for info in response.profitability_info {
            println!("\n🪙 {} ({}):", info.algorithm, info.coin);
            println!("  Revenue: €{:.4}/day", info.revenue_eur_day);
            println!("  Cost: €{:.4}/day", info.cost_eur_day);
            println!("  Profit: €{:.4}/day", info.profit_eur_day);
            println!("  Coin Price: €{:.4}", info.coin_price_eur);
            println!("  Confidence: {:.1}%", info.confidence * 100.0);
        }
    }

    if let Some(timestamp) = response.timestamp {
        let dt = chrono::DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32);
        if let Some(dt) = dt {
            println!("\n🕒 Data time: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    }
    
    println!("📅 Data age: {} seconds", response.data_age_seconds);

    Ok(())
}

async fn config_command(
    client: &mut BunkerMinerDaemonClient<Channel>,
    matches: &clap::ArgMatches,
) -> Result<()> {
    match matches.subcommand() {
        Some(("get", sub_matches)) => {
            let section = sub_matches.get_one::<String>("section")
                .map(|s| s.as_str())
                .unwrap_or("");

            let request = GetConfigRequest {
                section: section.to_string(),
            };

            let response = client.get_config(request).await?.into_inner();

            println!("BUNKER MINER - Configuration");
            println!("============================");
            
            if !section.is_empty() {
                println!("📁 Section: {}", section);
            }
            
            println!("📋 Configuration:\n");
            
            // Pretty print the JSON
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&response.config_json) {
                println!("{}", serde_json::to_string_pretty(&json_value)?);
            } else {
                println!("{}", response.config_json);
            }
            
            println!("\n🏷️ Version: {}", response.config_version);
        }
        Some(("set", sub_matches)) => {
            let config_json = sub_matches.get_one::<String>("config").unwrap();
            let validate_only = sub_matches.get_flag("validate-only");

            let request = SetConfigRequest {
                config_json: config_json.clone(),
                validate_only,
                restart_services: true,
            };

            let response = client.set_config(request).await?.into_inner();

            println!("BUNKER MINER - Set Configuration");
            println!("===============================");

            let status = match command_response::Status::from_i32(response.status) {
                Some(command_response::Status::StatusSuccess) => "✅ SUCCESS",
                Some(command_response::Status::StatusError) => "❌ ERROR",
                _ => "❓ UNKNOWN",
            };

            println!("\n📊 Result: {}", status);

            if !response.validation_errors.is_empty() {
                println!("\n❌ Validation Errors:");
                for (i, error) in response.validation_errors.iter().enumerate() {
                    println!("  {}. {}", i + 1, error);
                }
            }

            if !response.services_requiring_restart.is_empty() {
                println!("\n🔄 Services requiring restart:");
                for service in response.services_requiring_restart {
                    println!("  - {}", service);
                }
            }
        }
        _ => {
            println!("BUNKER MINER - Configuration Management");
            println!("======================================");
            println!("Use 'config get' or 'config set' subcommands");
        }
    }

    Ok(())
}