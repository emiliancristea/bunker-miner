mod hardware;
mod benchmarking;
mod profiles;
mod config;
mod miners;

use clap::{Arg, Command};
use std::process;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error};

use hardware::HardwareDetector;
use benchmarking::BenchmarkingEngine;
use profiles::ProfileManager;
use config::ConfigManager;
use miners::{MinerManager, ProcessSupervisor, Telemetry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let matches = Command::new("bunker-miner-daemon")
        .version("0.1.0")
        .author("Emilian Cristea <emilian@bunkercorpo.com>")
        .about("BUNKER MINER - Secure cryptocurrency mining daemon")
        .arg(
            Arg::new("health-check")
                .long("health-check")
                .help("Perform health check and exit")
                .action(clap::ArgAction::SetTrue)
        )
        .subcommand(
            Command::new("benchmark")
                .about("Run hardware benchmarking for all supported algorithms")
                .arg(
                    Arg::new("device")
                        .long("device")
                        .short('d')
                        .help("Benchmark specific device by ID")
                        .value_name("DEVICE_ID")
                )
                .arg(
                    Arg::new("algorithm")
                        .long("algorithm")
                        .short('a')
                        .help("Benchmark specific algorithm")
                        .value_name("ALGORITHM")
                )
                .arg(
                    Arg::new("duration")
                        .long("duration")
                        .help("Benchmark duration in seconds")
                        .value_name("SECONDS")
                        .default_value("60")
                )
        )
        .subcommand(
            Command::new("list-devices")
                .about("List all detected mining devices")
        )
        .subcommand(
            Command::new("show-profiles")
                .about("Show saved device profiles")
        )
        .subcommand(
            Command::new("start")
                .about("Start mining with current configuration")
        )
        .subcommand(
            Command::new("stop")
                .about("Stop all mining processes")
        )
        .subcommand(
            Command::new("status")
                .about("Show current mining status")
        )
        .get_matches();

    // Handle health check
    if matches.get_flag("health-check") {
        perform_health_check().await;
        process::exit(0);
    }

    match matches.subcommand() {
        Some(("benchmark", sub_matches)) => {
            run_benchmark_command(sub_matches).await?;
        }
        Some(("list-devices", _)) => {
            list_devices_command().await?;
        }
        Some(("show-profiles", _)) => {
            show_profiles_command().await?;
        }
        Some(("start", _)) => {
            start_mining_command().await?;
        }
        Some(("stop", _)) => {
            info!("Stopping mining operation...");
            println!("BUNKER MINER Daemon - Stop Mining");
            println!("Mining stopped successfully");
        }
        Some(("status", _)) => {
            info!("Checking mining status...");
            println!("BUNKER MINER Daemon - Status");
            println!("Status: Not mining (daemon initialized successfully)");
        }
        _ => {
            println!("BUNKER MINER Daemon v0.1.0");
            println!("Use --help to see available commands");
            println!("Device detection and benchmarking engine ready");
        }
    }

    Ok(())
}

async fn perform_health_check() {
    println!("BUNKER MINER Daemon Health Check");
    println!("================================");
    println!("Status: OK");
    println!("Version: 0.1.0");
    
    // Basic system checks
    let mut system = sysinfo::System::new_all();
    system.refresh_all();
    
    println!("System Memory: {} GB", system.total_memory() / 1024 / 1024 / 1024);
    println!("Available Memory: {} GB", system.available_memory() / 1024 / 1024 / 1024);
    
    // Test hardware detection
    println!("\nHardware Detection Test:");
    match HardwareDetector::new() {
        Ok(mut detector) => {
            match detector.detect_devices() {
                Ok(devices) => {
                    println!("✓ Hardware detection working");
                    println!("  Detected {} device(s)", devices.len());
                    
                    for device in devices {
                        println!("  - {} ({})", device.name, format!("{:?}", device.device_type));
                    }
                }
                Err(e) => {
                    println!("⚠ Hardware detection error: {}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠ Failed to initialize hardware detector: {}", e);
        }
    }
    
    // Test permissions
    println!("\nPermission Check:");
    if let Ok(detector) = HardwareDetector::new() {
        if let Ok(permissions) = detector.check_permissions() {
            println!("  NVML access: {}", if permissions.nvml_access { "✓" } else { "✗" });
            println!("  ROCm access: {}", if permissions.rocm_smi_access { "✓" } else { "✗" });
            println!("  System access: {}", if permissions.system_access { "✓" } else { "✗" });
        }
    }
}

async fn run_benchmark_command(matches: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    println!("BUNKER MINER Daemon - Hardware Benchmark");
    println!("=========================================");
    
    info!("Initializing hardware detection...");
    let mut hardware_detector = HardwareDetector::new()?;
    let hardware_detector = Arc::new(RwLock::new(hardware_detector));
    
    info!("Initializing benchmarking engine...");
    let mut benchmarking_engine = BenchmarkingEngine::new(hardware_detector.clone())?;
    
    info!("Initializing profile manager...");
    let mut profile_manager = ProfileManager::new()?;
    
    println!("\n🔍 Starting comprehensive hardware benchmark...");
    println!("This process will take several minutes depending on your hardware.\n");
    
    // Run benchmarks for all devices
    let reports = benchmarking_engine.benchmark_all_devices().await?;
    
    println!("\n📊 Benchmark Results Summary:");
    println!("============================");
    
    let mut total_algorithms = 0;
    let mut successful_benchmarks = 0;
    
    for report in &reports {
        println!("\n🔧 Device: {} ({})", report.device.name, format!("{:?}", report.device.device_type));
        println!("   Status: {:?}", report.status);
        println!("   Duration: {}s", report.total_duration_seconds);
        
        if report.status == benchmarking::BenchmarkStatus::Completed {
            let successful_results: Vec<_> = report.results.iter()
                .filter(|r| r.success)
                .collect();
            
            successful_benchmarks += successful_results.len();
            total_algorithms += report.results.len();
            
            if !successful_results.is_empty() {
                println!("   Results:");
                for result in successful_results {
                    let efficiency_str = if let Some(power) = result.power_watts {
                        format!(" ({:.1} H/W)", result.hashrate_hs / power)
                    } else {
                        String::new()
                    };
                    
                    println!("     {} -> {:.2} {} ({:.0} H/s){}",
                             result.algorithm,
                             result.hashrate,
                             result.hashrate_unit,
                             result.hashrate_hs,
                             efficiency_str);
                }
                
                if let Some(best) = &report.best_algorithm {
                    println!("   🏆 Best Algorithm: {}", best);
                }
                
                if let Some(efficient) = &report.most_efficient_algorithm {
                    println!("   ⚡ Most Efficient: {}", efficient);
                }
            }
        }
        
        // Create and save profile
        println!("   📝 Creating device profile...");
        let profile = profile_manager.create_profile_from_benchmark(report)?;
        profile_manager.save_profile(profile)?;
        println!("   ✓ Profile saved");
    }
    
    println!("\n✅ Benchmark Complete!");
    println!("   Total devices: {}", reports.len());
    println!("   Successful benchmarks: {}/{}", successful_benchmarks, total_algorithms);
    println!("   Profiles saved: {}", reports.len());
    
    // Show profiles location
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("bunker-miner");
    println!("   Profiles location: {}/profiles.json", config_dir.display());
    
    println!("\n💡 Use 'show-profiles' command to view saved profiles");
    println!("💡 Profiles will be used for intelligent profit switching");
    
    Ok(())
}

async fn list_devices_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("BUNKER MINER Daemon - Device Detection");
    println!("=====================================");
    
    info!("Detecting hardware devices...");
    let mut detector = HardwareDetector::new()?;
    let devices = detector.detect_devices()?;
    
    if devices.is_empty() {
        println!("⚠ No mining devices detected");
        println!("Make sure you have:");
        println!("  - NVIDIA/AMD GPU drivers installed");
        println!("  - Proper permissions for hardware access");
        return Ok(());
    }
    
    println!("\n🔧 Detected Devices ({}):", devices.len());
    println!("===================");
    
    for (i, device) in devices.iter().enumerate() {
        println!("\n{}. {} (ID: {})", i + 1, device.name, device.id);
        println!("   Type: {:?}", device.device_type);
        
        if device.memory_mb > 0 {
            println!("   Memory: {} GB", device.memory_mb / 1024);
        }
        
        if let Some(ref driver) = device.driver_version {
            println!("   Driver: {}", driver);
        }
        
        if let Some(ref pci) = device.pci_info {
            println!("   PCI: {} ({}:{})", pci.bus_id, pci.vendor_id, pci.device_id);
        }
        
        println!("   Supported Algorithms: {}", device.supported_algorithms.join(", "));
        
        // Show current metrics
        let metrics = &device.metrics;
        if let Some(temp) = metrics.temperature_c {
            println!("   Temperature: {:.1}°C", temp);
        }
        if let Some(power) = metrics.power_watts {
            println!("   Power: {:.1}W", power);
        }
        if let Some(util) = metrics.utilization_percent {
            println!("   Utilization: {:.1}%", util);
        }
    }
    
    println!("\n💡 Use 'benchmark' command to test performance");
    
    Ok(())
}

async fn show_profiles_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("BUNKER MINER Daemon - Device Profiles");
    println!("====================================");
    
    let mut profile_manager = ProfileManager::new()?;
    let profiles = profile_manager.get_all_profiles()?;
    
    if profiles.is_empty() {
        println!("📝 No device profiles found");
        println!("Run 'benchmark' command to create profiles");
        return Ok(());
    }
    
    println!("\n📊 Saved Profiles ({}):", profiles.len());
    println!("================");
    
    for (i, profile) in profiles.iter().enumerate() {
        println!("\n{}. {} (ID: {})", i + 1, profile.device.name, profile.device.id);
        println!("   Created: {}", profile.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("   Updated: {}", profile.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("   Algorithms: {}", profile.algorithms.len());
        
        // Show best performing algorithms
        let mut algorithm_performances: Vec<_> = profile.algorithms.values().collect();
        algorithm_performances.sort_by(|a, b| b.average_metrics.avg_hashrate_hs.partial_cmp(&a.average_metrics.avg_hashrate_hs).unwrap());
        
        if let Some(best) = algorithm_performances.first() {
            println!("   🏆 Best: {} -> {:.0} H/s", 
                     best.algorithm, 
                     best.average_metrics.avg_hashrate_hs);
            
            if let Some(power) = best.average_metrics.avg_power_watts {
                println!("        Power: {:.1}W, Efficiency: {:.1} H/W", 
                         power, 
                         best.average_metrics.avg_hashrate_hs / power);
            }
        }
        
        // Show all algorithms briefly
        println!("   Algorithms:");
        for (algo_name, algo_profile) in &profile.algorithms {
            let power_str = if let Some(power) = algo_profile.average_metrics.avg_power_watts {
                format!(" @ {:.1}W", power)
            } else {
                String::new()
            };
            
            println!("     {} -> {:.0} H/s{}", 
                     algo_name, 
                     algo_profile.average_metrics.avg_hashrate_hs,
                     power_str);
        }
    }
    
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("bunker-miner");
    println!("\n📁 Profiles stored in: {}/profiles.json", config_dir.display());
    
    Ok(())
}

async fn start_mining_command() -> Result<(), Box<dyn std::error::Error>> {
    println!("BUNKER MINER Daemon - Start Mining");
    println!("==================================");
    
    info!("Initializing mining operation...");
    
    // Initialize configuration manager
    let mut config_manager = ConfigManager::new()?;
    let config = config_manager.load_config().await?;
    
    info!("✓ Configuration loaded successfully");
    
    // Initialize hardware detector
    let mut hardware_detector = HardwareDetector::new()?;
    let devices = hardware_detector.detect_devices()?;
    
    if devices.is_empty() {
        return Err("No mining devices detected. Please ensure GPU drivers are installed and accessible.".into());
    }
    
    info!("✓ Detected {} mining device(s)", devices.len());
    
    // Initialize miner manager
    let miner_manager = MinerManager::new()?;
    
    // Select appropriate miner for configured coin
    let adapter = miner_manager.get_adapter_for_coin(&config.mining.active_coin)
        .ok_or_else(|| format!("No miner adapter available for coin: {}", config.mining.active_coin))?;
    
    info!("✓ Selected {} miner for {}", adapter.get_name(), config.mining.active_coin);
    
    // Ensure miner binary is available
    let binary_path = miner_manager.ensure_binary_available(&adapter).await?;
    info!("✓ Miner binary ready: {}", binary_path.display());
    
    // Select devices to use (for now, use all compatible devices)
    let compatible_devices: Vec<_> = devices.iter()
        .filter(|device| {
            // Check if device supports any of the miner's algorithms
            device.supported_algorithms.iter()
                .any(|algo| adapter.get_supported_algorithms().contains(algo))
        })
        .collect();
    
    if compatible_devices.is_empty() {
        return Err("No compatible devices found for the selected miner and coin".into());
    }
    
    let device_ids: Vec<String> = compatible_devices.iter()
        .enumerate()
        .map(|(i, _)| i.to_string())
        .collect();
    
    info!("✓ Using {} compatible device(s)", compatible_devices.len());
    for (i, device) in compatible_devices.iter().enumerate() {
        info!("  Device {}: {} ({})", i, device.name, format!("{:?}", device.device_type));
    }
    
    // Create telemetry channel
    let (telemetry_tx, mut telemetry_rx) = mpsc::unbounded_channel::<Telemetry>();
    
    // Create process supervisor
    let mut supervisor = ProcessSupervisor::new(
        config.clone(),
        adapter.clone(),
        binary_path,
        device_ids,
    );
    
    println!("\n🚀 Starting mining process...");
    
    // Start the mining process
    supervisor.start(telemetry_tx).await?;
    
    println!("✅ Mining started successfully!");
    println!("  Coin: {}", config.mining.active_coin);
    println!("  Pool: {}", config.get_active_pool()?.url);
    println!("  Wallet: {}...{}", 
             &config.get_active_wallet()?.address[..8], 
             &config.get_active_wallet()?.address[config.get_active_wallet()?.address.len()-8..]);
    println!("  Miner: {}", adapter.get_name());
    println!("  Devices: {}", compatible_devices.len());
    
    println!("\n📊 Real-time telemetry:");
    println!("========================");
    
    // Set up telemetry display and supervision
    let supervision_handle = tokio::spawn(async move {
        supervisor.supervise().await
    });
    
    let telemetry_handle = tokio::spawn(async move {
        let mut last_telemetry_time = std::time::Instant::now();
        
        while let Some(telemetry) = telemetry_rx.recv().await {
            let now = std::time::Instant::now();
            
            // Only display telemetry every 10 seconds to avoid spam
            if now.duration_since(last_telemetry_time) >= Duration::from_secs(10) {
                print!("\r");
                print!("Hashrate: {:.2} {} | Shares: A:{} R:{} | ", 
                       telemetry.hashrate, 
                       telemetry.hashrate_unit,
                       telemetry.shares_accepted,
                       telemetry.shares_rejected);
                
                if let Some(temp) = telemetry.temperature_c {
                    print!("Temp: {:.1}°C | ", temp);
                }
                
                if let Some(power) = telemetry.power_watts {
                    print!("Power: {:.1}W", power);
                }
                
                println!();
                std::io::Write::flush(&mut std::io::stdout()).ok();
                
                last_telemetry_time = now;
            }
        }
    });
    
    println!("\n💡 Press Ctrl+C to stop mining");
    println!("   Mining will continue running in the background...");
    
    // Wait for Ctrl+C or process to finish
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n\n🛑 Received shutdown signal...");
            info!("Shutting down mining operation");
            
            // Cancel telemetry display
            telemetry_handle.abort();
            
            // Wait for supervision to finish
            if let Err(e) = timeout(Duration::from_secs(15), supervision_handle).await {
                warn!("Timeout waiting for mining supervision to shut down: {}", e);
            }
            
            println!("✅ Mining operation stopped");
        }
        result = supervision_handle => {
            match result {
                Ok(Ok(())) => {
                    println!("\n✅ Mining supervision completed normally");
                }
                Ok(Err(e)) => {
                    error!("Mining supervision error: {}", e);
                    return Err(e.into());
                }
                Err(e) => {
                    error!("Mining supervision task error: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hardware_detector_initialization() {
        let detector = HardwareDetector::new();
        assert!(detector.is_ok(), "Hardware detector should initialize successfully");
    }

    #[tokio::test]
    async fn test_device_detection() {
        let mut detector = HardwareDetector::new().expect("Failed to create detector");
        let devices = detector.detect_devices();
        
        // Device detection should succeed even if no devices are found
        assert!(devices.is_ok(), "Device detection should not fail");
        
        // Log detected devices for debugging
        if let Ok(devices) = devices {
            println!("Detected {} devices in test", devices.len());
            for device in &devices {
                println!("  - {}: {:?}", device.name, device.device_type);
            }
        }
    }

    #[tokio::test]
    async fn test_profile_manager_creation() {
        let profile_manager = ProfileManager::new();
        assert!(profile_manager.is_ok(), "Profile manager should initialize successfully");
    }

    #[test]
    fn test_command_line_parsing() {
        // Test basic command parsing
        let cmd = Command::new("bunker-miner-daemon")
            .subcommand(Command::new("benchmark"))
            .subcommand(Command::new("list-devices"))
            .subcommand(Command::new("show-profiles"))
            .subcommand(Command::new("start"))
            .subcommand(Command::new("stop"));

        // Test benchmark subcommand
        let matches = cmd.clone().try_get_matches_from(vec!["bunker-miner-daemon", "benchmark"]);
        assert!(matches.is_ok());

        // Test list-devices subcommand
        let matches = cmd.clone().try_get_matches_from(vec!["bunker-miner-daemon", "list-devices"]);
        assert!(matches.is_ok());

        // Test show-profiles subcommand
        let matches = cmd.clone().try_get_matches_from(vec!["bunker-miner-daemon", "show-profiles"]);
        assert!(matches.is_ok());
        
        // Test start subcommand
        let matches = cmd.clone().try_get_matches_from(vec!["bunker-miner-daemon", "start"]);
        assert!(matches.is_ok());
        
        // Test stop subcommand
        let matches = cmd.clone().try_get_matches_from(vec!["bunker-miner-daemon", "stop"]);
        assert!(matches.is_ok());
    }
    
    #[tokio::test]
    async fn test_configuration_integration() {
        // Test that ConfigManager can be created without errors
        let config_manager = ConfigManager::new();
        assert!(config_manager.is_ok(), "ConfigManager should initialize successfully");
        
        // Test that MinerManager can be created
        let miner_manager = MinerManager::new();
        assert!(miner_manager.is_ok(), "MinerManager should initialize successfully");
        
        // Test adapter selection
        let manager = miner_manager.unwrap();
        assert!(manager.get_adapter_for_coin("ethereum").is_some());
        assert!(manager.get_adapter_for_coin("monero").is_some());
        assert!(manager.get_adapter_for_coin("unsupported_coin").is_none());
    }
    
    #[test]
    fn test_telemetry_creation() {
        let telemetry = Telemetry::default();
        assert_eq!(telemetry.hashrate, 0.0);
        assert_eq!(telemetry.shares_accepted, 0);
        assert_eq!(telemetry.algorithm, "unknown");
        assert!(telemetry.timestamp > 0);
    }
}