/*!
 * BUNKER MINER - Hardware Detection & Monitoring Module
 *
 * This module provides comprehensive hardware detection and monitoring capabilities
 * for NVIDIA GPUs, AMD GPUs, and CPU resources. It serves as the foundation for
 * intelligent mining operations and profit optimization.
 *
 * Key Features:
 * - Cross-platform GPU detection (NVIDIA via NVML, AMD via rocm-smi/lspci)
 * - CPU detection and capability assessment
 * - Real-time monitoring of power usage, temperatures, and utilization
 * - Unified abstraction layer for all mining device types
 * - Security-focused design with privilege validation
 */

use anyhow::{Context, Result};
use nvml_wrapper::{Device, Nvml};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use sysinfo::{CpuExt, System, SystemExt};
use tracing::{debug, error, info, warn};

/// Unified representation of a mining device (GPU or CPU)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningDevice {
    /// Unique identifier for this device instance
    pub id: String,
    /// Human-readable device name
    pub name: String,
    /// Device type (NVIDIA_GPU, AMD_GPU, CPU)
    pub device_type: DeviceType,
    /// Device memory in MB (0 for CPUs)
    pub memory_mb: u64,
    /// Driver version (if applicable)
    pub driver_version: Option<String>,
    /// PCI bus information
    pub pci_info: Option<PciInfo>,
    /// Supported mining algorithms
    pub supported_algorithms: Vec<String>,
    /// Current device metrics (temperature, power, etc.)
    pub metrics: DeviceMetrics,
    /// Device-specific properties
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    #[serde(rename = "nvidia_gpu")]
    NvidiaGpu,
    #[serde(rename = "amd_gpu")]
    AmdGpu,
    #[serde(rename = "cpu")]
    Cpu,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::NvidiaGpu => formatter.write_str("nvidia_gpu"),
            DeviceType::AmdGpu => formatter.write_str("amd_gpu"),
            DeviceType::Cpu => formatter.write_str("cpu"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PciInfo {
    pub bus_id: String,
    pub device_id: String,
    pub vendor_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceMetrics {
    /// Current temperature in Celsius
    pub temperature_c: Option<f32>,
    /// Current power consumption in Watts
    pub power_watts: Option<f32>,
    /// Current utilization percentage (0-100)
    pub utilization_percent: Option<f32>,
    /// Current memory utilization percentage (0-100)
    pub memory_utilization_percent: Option<f32>,
    /// Current fan speed percentage (0-100)
    pub fan_speed_percent: Option<f32>,
    /// Current core clock in MHz
    pub core_clock_mhz: Option<u32>,
    /// Current memory clock in MHz
    pub memory_clock_mhz: Option<u32>,
}

/// Hardware detection and monitoring service
pub struct HardwareDetector {
    nvml: Option<Nvml>,
    system: System,
    cached_devices: Option<Vec<MiningDevice>>,
}

impl HardwareDetector {
    /// Create a new hardware detector instance
    pub fn new() -> Result<Self> {
        info!("Initializing hardware detector");

        // Initialize NVML for NVIDIA GPU detection
        let nvml = match Nvml::init() {
            Ok(nvml) => {
                info!("NVML initialized successfully");
                Some(nvml)
            }
            Err(e) => {
                warn!(
                    "Failed to initialize NVML: {}. NVIDIA GPU detection disabled",
                    e
                );
                None
            }
        };

        // Initialize system information
        let mut system = System::new_all();
        system.refresh_all();

        Ok(Self {
            nvml,
            system,
            cached_devices: None,
        })
    }

    /// Detect all available mining devices
    pub fn detect_devices(&mut self) -> Result<Vec<MiningDevice>> {
        info!("Starting comprehensive hardware detection");

        let mut devices = Vec::new();

        // Detect NVIDIA GPUs
        if let Some(ref nvml) = self.nvml {
            match self.detect_nvidia_devices(nvml) {
                Ok(mut nvidia_devices) => {
                    info!("Detected {} NVIDIA GPU(s)", nvidia_devices.len());
                    devices.append(&mut nvidia_devices);
                }
                Err(e) => {
                    warn!("Failed to detect NVIDIA devices: {}", e);
                }
            }
        }

        // Detect AMD GPUs
        match self.detect_amd_devices() {
            Ok(mut amd_devices) => {
                info!("Detected {} AMD GPU(s)", amd_devices.len());
                devices.append(&mut amd_devices);
            }
            Err(e) => {
                warn!("Failed to detect AMD devices: {}", e);
            }
        }

        // Detect CPU
        match self.detect_cpu() {
            Ok(cpu_device) => {
                info!("Detected CPU: {}", cpu_device.name);
                devices.push(cpu_device);
            }
            Err(e) => {
                warn!("Failed to detect CPU: {}", e);
            }
        }

        info!(
            "Hardware detection completed. Found {} total devices",
            devices.len()
        );

        // Cache the detected devices
        self.cached_devices = Some(devices.clone());

        Ok(devices)
    }

    /// Detect NVIDIA GPUs using NVML
    fn detect_nvidia_devices(&self, nvml: &Nvml) -> Result<Vec<MiningDevice>> {
        let device_count = nvml
            .device_count()
            .context("Failed to get NVIDIA device count")?;

        debug!("NVML reports {} NVIDIA device(s)", device_count);

        let mut devices = Vec::new();

        for i in 0..device_count {
            match self.create_nvidia_device(nvml, i) {
                Ok(device) => {
                    debug!("Successfully created NVIDIA device: {}", device.name);
                    devices.push(device);
                }
                Err(e) => {
                    error!("Failed to create NVIDIA device {}: {}", i, e);
                }
            }
        }

        Ok(devices)
    }

    /// Create a MiningDevice from NVIDIA GPU information
    fn create_nvidia_device(&self, nvml: &Nvml, index: u32) -> Result<MiningDevice> {
        let device = nvml
            .device_by_index(index)
            .context(format!("Failed to get NVIDIA device {}", index))?;

        let name = device.name().context("Failed to get NVIDIA device name")?;

        let memory_info = device
            .memory_info()
            .context("Failed to get NVIDIA memory info")?;

        let driver_version = nvml.sys_driver_version().ok();

        // Get PCI information
        let pci_info = match device.pci_info() {
            Ok(pci) => Some(PciInfo {
                bus_id: pci.bus_id,
                device_id: format!("{:04x}", pci.pci_device_id),
                vendor_id: "10de".to_string(), // NVIDIA vendor ID
            }),
            Err(e) => {
                debug!("Could not get PCI info for NVIDIA device {}: {}", index, e);
                None
            }
        };

        // Get current metrics
        let metrics = self.get_nvidia_metrics(&device)?;

        // Create device properties
        let mut properties = HashMap::new();

        // Add compute capability if available
        if let Ok(capability) = device.cuda_compute_capability() {
            properties.insert(
                "compute_capability".to_string(),
                format!("{}.{}", capability.major, capability.minor),
            );
        }

        // Add architecture if available
        if let Ok(arch) = device.architecture() {
            properties.insert("architecture".to_string(), format!("{:?}", arch));
        }

        // Add power management info
        if let Ok(power_limit) = device.power_management_limit_default() {
            properties.insert("power_limit_watts".to_string(), power_limit.to_string());
        }

        Ok(MiningDevice {
            id: format!("nvidia_gpu_{}", index),
            name,
            device_type: DeviceType::NvidiaGpu,
            memory_mb: memory_info.total / 1024 / 1024,
            driver_version,
            pci_info,
            supported_algorithms: self.get_nvidia_supported_algorithms(),
            metrics,
            properties,
        })
    }

    /// Get current metrics for NVIDIA device
    fn get_nvidia_metrics(&self, device: &Device) -> Result<DeviceMetrics> {
        let mut metrics = DeviceMetrics::default();

        // Temperature
        if let Ok(temp) =
            device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        {
            metrics.temperature_c = Some(temp as f32);
        }

        // Power consumption
        if let Ok(power) = device.power_usage() {
            metrics.power_watts = Some(power as f32 / 1000.0); // Convert mW to W
        }

        // GPU utilization
        if let Ok(utilization) = device.utilization_rates() {
            metrics.utilization_percent = Some(utilization.gpu as f32);
            metrics.memory_utilization_percent = Some(utilization.memory as f32);
        }

        // Fan speed
        if let Ok(fan_speed) = device.fan_speed(0) {
            metrics.fan_speed_percent = Some(fan_speed as f32);
        }

        // Clock speeds
        if let Ok(graphics_clock) =
            device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
        {
            metrics.core_clock_mhz = Some(graphics_clock);
        }

        if let Ok(memory_clock) =
            device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory)
        {
            metrics.memory_clock_mhz = Some(memory_clock);
        }

        Ok(metrics)
    }

    /// Static version of get_nvidia_metrics to avoid borrowing issues
    fn get_nvidia_metrics_static(device: &nvml_wrapper::Device) -> Result<DeviceMetrics> {
        let mut metrics = DeviceMetrics::default();

        // Temperature
        if let Ok(temp) =
            device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
        {
            metrics.temperature_c = Some(temp as f32);
        }

        // Power consumption
        if let Ok(power) = device.power_usage() {
            metrics.power_watts = Some(power as f32 / 1000.0); // Convert mW to W
        }

        // GPU utilization
        if let Ok(utilization) = device.utilization_rates() {
            metrics.utilization_percent = Some(utilization.gpu as f32);
            metrics.memory_utilization_percent = Some(utilization.memory as f32);
        }

        // Fan speed
        if let Ok(fan_speed) = device.fan_speed(0) {
            metrics.fan_speed_percent = Some(fan_speed as f32);
        }

        // Clock speeds
        if let Ok(graphics_clock) =
            device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
        {
            metrics.core_clock_mhz = Some(graphics_clock);
        }

        if let Ok(memory_clock) =
            device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory)
        {
            metrics.memory_clock_mhz = Some(memory_clock);
        }

        Ok(metrics)
    }

    /// Get supported algorithms for NVIDIA GPUs
    fn get_nvidia_supported_algorithms(&self) -> Vec<String> {
        vec![
            "ethash".to_string(),
            "etchash".to_string(),
            "kheavyhash".to_string(), // Kaspa
            "autolykos2".to_string(), // Ergo
            "octopus".to_string(),    // Conflux
            "kawpow".to_string(),     // Ravencoin
            "beamv3".to_string(),     // Beam
            "neoscrypt".to_string(),  // Feathercoin
            "zhash".to_string(),      // ZelCash
        ]
    }

    /// Detect AMD GPUs using command-line tools
    fn detect_amd_devices(&mut self) -> Result<Vec<MiningDevice>> {
        debug!("Attempting AMD GPU detection");

        // First try rocm-smi if available
        if let Ok(devices) = self.detect_amd_with_rocm_smi() {
            if !devices.is_empty() {
                return Ok(devices);
            }
        }

        // Fallback to lspci for basic AMD GPU detection
        self.detect_amd_with_lspci()
    }

    /// Detect AMD GPUs using rocm-smi
    fn detect_amd_with_rocm_smi(&mut self) -> Result<Vec<MiningDevice>> {
        let output = Command::new("rocm-smi")
            .args(["--showproductname", "--showmeminfo", "--json"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.parse_rocm_smi_output(&stdout)
            }
            Ok(_) => {
                debug!("rocm-smi command failed");
                Ok(vec![])
            }
            Err(_) => {
                debug!("rocm-smi not found or not accessible");
                Ok(vec![])
            }
        }
    }

    /// Parse rocm-smi JSON output
    fn parse_rocm_smi_output(&mut self, _output: &str) -> Result<Vec<MiningDevice>> {
        // This is a simplified parser - in production, we would use proper JSON parsing
        // For now, return empty vec as AMD detection is secondary priority
        debug!("rocm-smi output parsing not yet fully implemented");
        Ok(vec![])
    }

    /// Detect AMD GPUs using lspci
    fn detect_amd_with_lspci(&mut self) -> Result<Vec<MiningDevice>> {
        #[cfg(target_os = "linux")]
        {
            let output = Command::new("lspci").args(&["-nn", "-d", "1002:"]).output();

            match output {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    self.parse_lspci_amd_output(&stdout)
                }
                _ => {
                    debug!("lspci command failed or not available");
                    Ok(vec![])
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, AMD GPU detection would require WMI or DirectX APIs
            // For now, return empty vec
            debug!("AMD GPU detection on Windows not yet implemented");
            Ok(vec![])
        }
    }

    /// Parse lspci output for AMD GPUs
    #[cfg(target_os = "linux")]
    fn parse_lspci_amd_output(&mut self, output: &str) -> Result<Vec<MiningDevice>> {
        let mut devices = Vec::new();
        let re = regex::Regex::new(
            r"([0-9a-f]{2}:[0-9a-f]{2}\.[0-9]) VGA compatible controller: (.+?) \[([0-9a-f]{4}):([0-9a-f]{4})\]",
        )?;

        for (index, line) in output.lines().enumerate() {
            if let Some(captures) = re.captures(line) {
                let bus_id = captures.get(1).unwrap().as_str();
                let name = captures.get(2).unwrap().as_str();
                let device_id = captures.get(4).unwrap().as_str();

                let device = MiningDevice {
                    id: format!("amd_gpu_{}", index),
                    name: format!("AMD {}", name),
                    device_type: DeviceType::AmdGpu,
                    memory_mb: 0, // Would need additional queries to get memory info
                    driver_version: None,
                    pci_info: Some(PciInfo {
                        bus_id: bus_id.to_string(),
                        device_id: device_id.to_string(),
                        vendor_id: "1002".to_string(), // AMD vendor ID
                    }),
                    supported_algorithms: self.get_amd_supported_algorithms(),
                    metrics: DeviceMetrics::default(),
                    properties: HashMap::new(),
                };

                devices.push(device);
            }
        }

        Ok(devices)
    }

    /// Get supported algorithms for AMD GPUs
    #[cfg(target_os = "linux")]
    fn get_amd_supported_algorithms(&self) -> Vec<String> {
        vec![
            "ethash".to_string(),
            "etchash".to_string(),
            "autolykos2".to_string(), // Ergo
            "kheavyhash".to_string(), // Kaspa
            "kawpow".to_string(),     // Ravencoin
            "verthash".to_string(),   // Vertcoin
        ]
    }

    /// Detect CPU information
    fn detect_cpu(&mut self) -> Result<MiningDevice> {
        self.system.refresh_cpu();

        let cpus = self.system.cpus();
        let cpu = cpus.first().context("No CPU information available")?;

        let name = cpu.brand().to_string();
        let core_count = self.system.physical_core_count().unwrap_or(cpus.len());

        let mut properties = HashMap::new();
        properties.insert("core_count".to_string(), core_count.to_string());
        properties.insert("thread_count".to_string(), cpus.len().to_string());
        properties.insert("frequency_mhz".to_string(), cpu.frequency().to_string());

        // Get CPU metrics
        let metrics = DeviceMetrics {
            temperature_c: None, // CPU temp detection varies by platform
            power_watts: None,   // CPU power detection requires additional tools
            utilization_percent: Some(cpu.cpu_usage()),
            memory_utilization_percent: None,
            fan_speed_percent: None,
            core_clock_mhz: Some(cpu.frequency() as u32),
            memory_clock_mhz: None,
        };

        Ok(MiningDevice {
            id: "cpu_0".to_string(),
            name: format!("{} ({} cores)", name, core_count),
            device_type: DeviceType::Cpu,
            memory_mb: self.system.total_memory() / 1024 / 1024,
            driver_version: None,
            pci_info: None,
            supported_algorithms: self.get_cpu_supported_algorithms(),
            metrics,
            properties,
        })
    }

    /// Get supported algorithms for CPU mining
    fn get_cpu_supported_algorithms(&self) -> Vec<String> {
        vec![
            "randomx".to_string(),     // Monero
            "cryptonight".to_string(), // Various CN coins
            "yescrypt".to_string(),    // Yenten
            "cpupower".to_string(),    // CPUcoin
        ]
    }

    /// Update metrics for all cached devices
    pub fn update_metrics(&mut self) -> Result<()> {
        if let Some(ref mut devices) = self.cached_devices {
            for device in devices.iter_mut() {
                match device.device_type {
                    DeviceType::NvidiaGpu => {
                        if let Some(ref nvml) = self.nvml {
                            // Parse device index from ID
                            if let Some(index_str) = device.id.strip_prefix("nvidia_gpu_") {
                                if let Ok(index) = index_str.parse::<u32>() {
                                    if let Ok(nvml_device) = nvml.device_by_index(index) {
                                        device.metrics =
                                            Self::get_nvidia_metrics_static(&nvml_device)?;
                                    }
                                }
                            }
                        }
                    }
                    DeviceType::AmdGpu => {
                        // AMD metrics would be updated via rocm-smi or similar
                        debug!("AMD metrics update not yet implemented");
                    }
                    DeviceType::Cpu => {
                        self.system.refresh_cpu();
                        if let Some(cpu) = self.system.cpus().first() {
                            device.metrics.utilization_percent = Some(cpu.cpu_usage());
                            device.metrics.core_clock_mhz = Some(cpu.frequency() as u32);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get cached devices (if available)
    pub fn get_cached_devices(&self) -> Option<&Vec<MiningDevice>> {
        self.cached_devices.as_ref()
    }

    /// Check if the current user has necessary permissions for hardware access
    pub fn check_permissions(&self) -> Result<PermissionStatus> {
        let mut status = PermissionStatus::default();

        // Check NVML access
        if self.nvml.is_some() {
            status.nvml_access = true;
        }

        // Check for rocm-smi access
        if Command::new("rocm-smi").arg("--help").output().is_ok() {
            status.rocm_smi_access = true;
        }

        // Check for general system access
        status.system_access = true; // sysinfo generally works without special permissions

        Ok(status)
    }
}

#[derive(Debug, Default)]
pub struct PermissionStatus {
    pub nvml_access: bool,
    pub rocm_smi_access: bool,
    pub system_access: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detector_creation() {
        // Test that HardwareDetector can be created
        let detector = HardwareDetector::new();
        assert!(detector.is_ok());
    }

    #[test]
    fn test_device_type_serialization() {
        let device_type = DeviceType::NvidiaGpu;
        let serialized = serde_json::to_string(&device_type).unwrap();
        assert_eq!(serialized, "\"nvidia_gpu\"");

        let deserialized: DeviceType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, DeviceType::NvidiaGpu);
    }

    #[test]
    fn test_mining_device_serialization() {
        let device = MiningDevice {
            id: "test_gpu_0".to_string(),
            name: "Test GPU".to_string(),
            device_type: DeviceType::NvidiaGpu,
            memory_mb: 8192,
            driver_version: Some("470.123".to_string()),
            pci_info: None,
            supported_algorithms: vec!["ethash".to_string()],
            metrics: DeviceMetrics::default(),
            properties: HashMap::new(),
        };

        let serialized = serde_json::to_string(&device).unwrap();
        let deserialized: MiningDevice = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, device.id);
        assert_eq!(deserialized.name, device.name);
        assert_eq!(deserialized.device_type, device.device_type);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_lspci_parsing() {
        let mut detector = HardwareDetector::new().unwrap();
        let test_output = "01:00.0 VGA compatible controller: Advanced Micro Devices, Inc. [AMD/ATI] Navi 21 [Radeon RX 6800/6800 XT / 6900 XT] [1002:73bf]";

        let devices = detector.parse_lspci_amd_output(test_output).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_type, DeviceType::AmdGpu);
        assert!(devices[0].name.contains("AMD"));
    }
}
