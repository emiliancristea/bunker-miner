/*!
 * BUNKER MINER - Hardware Detection PoC
 * 
 * This PoC validates our ability to reliably detect and query hardware information
 * on both Windows and Linux systems using nvml-wrapper for NVIDIA GPUs and sysinfo
 * for CPU detection.
 * 
 * Success Criteria:
 * - Correctly identify all NVIDIA GPUs with name, temperature, power, clocks
 * - Identify CPU model and core count
 * - Stable operation without crashes
 * - Cross-platform compatibility
 */

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub uuid: String,
    pub memory_total_mb: u64,
    pub memory_free_mb: u64,
    pub driver_version: String,
    pub temperature_c: u32,
    pub power_usage_w: u32,
    pub gpu_utilization: u32,
    pub memory_utilization: u32,
    pub core_clock_mhz: u32,
    pub memory_clock_mhz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub name: String,
    pub vendor: String,
    pub cores_physical: usize,
    pub cores_logical: usize,
    pub frequency_mhz: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub timestamp: chrono::DateTime<Utc>,
    pub gpus: Vec<GpuInfo>,
    pub cpu: CpuInfo,
    pub total_memory_gb: u64,
    pub available_memory_gb: u64,
    pub platform: String,
}

fn detect_nvidia_gpus() -> Result<Vec<GpuInfo>> {
    info!("Detecting NVIDIA GPUs using NVML...");
    
    let nvml = match nvml_wrapper::Nvml::init() {
        Ok(nvml) => {
            info!("NVML initialized successfully");
            nvml
        },
        Err(e) => {
            warn!("NVML initialization failed: {}. NVIDIA GPUs may not be available.", e);
            return Ok(vec![]);
        }
    };

    let device_count = nvml.device_count()
        .context("Failed to get NVIDIA device count")?;
    
    info!("Found {} NVIDIA device(s)", device_count);
    
    let mut gpus = Vec::new();
    
    for i in 0..device_count {
        match nvml.device_by_index(i) {
            Ok(device) => {
                info!("Querying NVIDIA GPU #{}", i);
                
                let gpu_info = GpuInfo {
                    index: i,
                    name: device.name().unwrap_or_else(|_| "Unknown GPU".to_string()),
                    uuid: device.uuid().unwrap_or_else(|_| "Unknown UUID".to_string()),
                    memory_total_mb: device.memory_info().map(|m| m.total / 1024 / 1024).unwrap_or(0),
                    memory_free_mb: device.memory_info().map(|m| m.free / 1024 / 1024).unwrap_or(0),
                    driver_version: nvml.sys_driver_version().unwrap_or_else(|_| "Unknown".to_string()),
                    temperature_c: device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).unwrap_or(0),
                    power_usage_w: device.power_usage().unwrap_or(0) / 1000, // Convert mW to W
                    gpu_utilization: device.utilization_rates().map(|u| u.gpu).unwrap_or(0),
                    memory_utilization: device.utilization_rates().map(|u| u.memory).unwrap_or(0),
                    core_clock_mhz: device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics).unwrap_or(0),
                    memory_clock_mhz: device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory).unwrap_or(0),
                };
                
                info!("GPU #{}: {} ({}MB VRAM, {}°C, {}W)", 
                      i, gpu_info.name, gpu_info.memory_total_mb, 
                      gpu_info.temperature_c, gpu_info.power_usage_w);
                
                gpus.push(gpu_info);
            },
            Err(e) => {
                error!("Failed to get device #{}: {}", i, e);
            }
        }
    }
    
    Ok(gpus)
}

fn detect_cpu_info() -> Result<CpuInfo> {
    info!("Detecting CPU information using sysinfo...");
    
    let mut system = sysinfo::System::new_all();
    system.refresh_all();
    
    let cpu = system.global_cpu_info();
    let cpus = system.cpus();
    
    let cpu_info = CpuInfo {
        name: cpu.brand().to_string(),
        vendor: cpu.vendor_id().to_string(),
        cores_physical: system.physical_core_count().unwrap_or(0),
        cores_logical: cpus.len(),
        frequency_mhz: cpu.frequency(),
        usage_percent: cpu.cpu_usage(),
    };
    
    info!("CPU: {} ({} physical cores, {} logical cores, {}MHz)", 
          cpu_info.name, cpu_info.cores_physical, cpu_info.cores_logical, cpu_info.frequency_mhz);
    
    Ok(cpu_info)
}

fn detect_system_memory() -> Result<(u64, u64)> {
    let mut system = sysinfo::System::new_all();
    system.refresh_memory();
    
    let total_gb = system.total_memory() / 1024 / 1024 / 1024;
    let available_gb = system.available_memory() / 1024 / 1024 / 1024;
    
    info!("Memory: {}GB total, {}GB available", total_gb, available_gb);
    
    Ok((total_gb, available_gb))
}

fn run_detection() -> Result<SystemInfo> {
    info!("Starting comprehensive hardware detection...");
    
    let gpus = detect_nvidia_gpus()
        .context("GPU detection failed")?;
    
    let cpu = detect_cpu_info()
        .context("CPU detection failed")?;
    
    let (total_memory_gb, available_memory_gb) = detect_system_memory()
        .context("Memory detection failed")?;
    
    let platform = if cfg!(windows) {
        "Windows".to_string()
    } else if cfg!(target_os = "linux") {
        "Linux".to_string()
    } else {
        "Unknown".to_string()
    };
    
    let system_info = SystemInfo {
        timestamp: Utc::now(),
        gpus,
        cpu,
        total_memory_gb,
        available_memory_gb,
        platform,
    };
    
    Ok(system_info)
}

fn run_continuous_monitoring(interval_seconds: u64, iterations: u32) -> Result<()> {
    info!("Starting continuous hardware monitoring for {} iterations ({}s interval)", iterations, interval_seconds);
    
    for i in 1..=iterations {
        info!("=== Monitoring Iteration {}/{} ===", i, iterations);
        
        match run_detection() {
            Ok(system_info) => {
                println!("System Detection Results:");
                println!("{}", serde_json::to_string_pretty(&system_info)?);
                
                // Validate data integrity
                if system_info.gpus.is_empty() {
                    warn!("No GPUs detected - may indicate driver issues or no NVIDIA hardware");
                }
                
                if system_info.cpu.cores_physical == 0 {
                    error!("CPU detection failed - invalid core count");
                }
                
                // Check for reasonable temperature values
                for gpu in &system_info.gpus {
                    if gpu.temperature_c > 100 {
                        warn!("GPU #{} temperature seems high: {}°C", gpu.index, gpu.temperature_c);
                    }
                    
                    if gpu.power_usage_w > 500 {
                        warn!("GPU #{} power usage seems high: {}W", gpu.index, gpu.power_usage_w);
                    }
                }
                
            },
            Err(e) => {
                error!("Detection failed in iteration {}: {}", i, e);
            }
        }
        
        if i < iterations {
            info!("Waiting {}s before next iteration...", interval_seconds);
            thread::sleep(Duration::from_secs(interval_seconds));
        }
    }
    
    Ok(())
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("BUNKER MINER - Hardware Detection PoC");
    info!("Platform: {}", std::env::consts::OS);
    
    let matches = Command::new("hardware-detection")
        .about("BUNKER MINER Hardware Detection Proof of Concept")
        .arg(
            Arg::new("monitor")
                .long("monitor")
                .help("Enable continuous monitoring mode")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("interval")
                .long("interval")
                .help("Monitoring interval in seconds")
                .default_value("5")
                .value_parser(clap::value_parser!(u64))
        )
        .arg(
            Arg::new("iterations")
                .long("iterations")
                .help("Number of monitoring iterations")
                .default_value("12")
                .value_parser(clap::value_parser!(u32))
        )
        .get_matches();

    if matches.get_flag("monitor") {
        let interval = *matches.get_one::<u64>("interval").unwrap();
        let iterations = *matches.get_one::<u32>("iterations").unwrap();
        run_continuous_monitoring(interval, iterations)?;
    } else {
        // Single detection run
        match run_detection() {
            Ok(system_info) => {
                println!("\n🎯 Hardware Detection Results:");
                println!("{}", serde_json::to_string_pretty(&system_info)?);
                
                // Summary report
                println!("\n📊 Detection Summary:");
                println!("  Platform: {}", system_info.platform);
                println!("  GPUs: {} NVIDIA device(s)", system_info.gpus.len());
                println!("  CPU: {} ({} cores)", system_info.cpu.name, system_info.cpu.cores_physical);
                println!("  Memory: {}GB total", system_info.total_memory_gb);
                
                info!("✅ Hardware detection PoC completed successfully!");
            },
            Err(e) => {
                error!("❌ Hardware detection PoC failed: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_detection() {
        let cpu_info = detect_cpu_info().expect("CPU detection should work");
        assert!(!cpu_info.name.is_empty(), "CPU name should not be empty");
        assert!(cpu_info.cores_physical > 0, "Should detect at least one physical core");
        assert!(cpu_info.cores_logical > 0, "Should detect at least one logical core");
    }

    #[test]
    fn test_memory_detection() {
        let (total, available) = detect_system_memory().expect("Memory detection should work");
        assert!(total > 0, "Total memory should be greater than 0");
        assert!(available <= total, "Available memory should not exceed total");
    }

    #[test]
    fn test_gpu_detection() {
        // This test will pass even if no GPUs are found, as that's a valid state
        let gpus = detect_nvidia_gpus().expect("GPU detection should not crash");
        
        // If GPUs are found, validate their data
        for gpu in gpus {
            assert!(!gpu.name.is_empty(), "GPU name should not be empty");
            assert!(gpu.temperature_c < 200, "GPU temperature should be reasonable");
        }
    }
}