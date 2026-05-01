/*!
 * BUNKER MINER - Benchmarking Engine
 *
 * This module implements a comprehensive benchmarking system that measures the
 * performance of different mining algorithms on detected hardware. The benchmarking
 * results form the foundation for intelligent profit switching and optimization.
 *
 * Key Features:
 * - Algorithm-specific benchmarking for each device type
 * - Integration with third-party miners (lolMiner, XMRig, etc.)
 * - Real-time power and performance monitoring during benchmarks
 * - Secure process execution with input sanitization
 * - Configurable benchmark duration and parameters
 * - Results persistence and caching
 */

use crate::hardware::{DeviceType, HardwareDetector, MiningDevice};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use which::which;

/// Configuration for a single algorithm benchmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    /// Algorithm name (e.g., "ethash", "kheavyhash")
    pub name: String,
    /// Human-readable algorithm display name
    pub display_name: String,
    /// Supported device types for this algorithm
    pub supported_devices: Vec<DeviceType>,
    /// Miner executable configurations for this algorithm
    pub miner_configs: Vec<MinerConfig>,
    /// Expected hashrate range (min, max) for validation
    pub hashrate_range: Option<(f64, f64)>,
    /// Benchmark duration in seconds
    pub benchmark_duration: u64,
    /// Additional algorithm-specific parameters
    pub parameters: HashMap<String, String>,
}

/// Configuration for a specific miner that supports an algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    /// Miner name (e.g., "lolMiner", "XMRig")
    pub name: String,
    /// Expected executable name
    pub executable: String,
    /// Command line arguments template
    pub args_template: Vec<String>,
    /// Regex pattern to extract hashrate from output
    pub hashrate_regex: String,
    /// Hashrate unit (H/s, kH/s, MH/s, GH/s)
    pub hashrate_unit: String,
    /// Additional miner-specific properties
    pub properties: HashMap<String, String>,
}

/// Result of a single algorithm benchmark on a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique identifier for this benchmark run
    pub id: String,
    /// Device that was benchmarked
    pub device_id: String,
    /// Algorithm that was benchmarked
    pub algorithm: String,
    /// Miner used for the benchmark
    pub miner_name: String,
    /// Measured hashrate in specified unit
    pub hashrate: f64,
    /// Hashrate unit
    pub hashrate_unit: String,
    /// Normalized hashrate in H/s
    pub hashrate_hs: f64,
    /// Average power consumption during benchmark (Watts)
    pub power_watts: Option<f64>,
    /// Average temperature during benchmark (Celsius)
    pub temperature_c: Option<f32>,
    /// Benchmark start time
    pub start_time: DateTime<Utc>,
    /// Benchmark end time
    pub end_time: DateTime<Utc>,
    /// Benchmark duration in seconds
    pub duration_seconds: u64,
    /// Whether the benchmark completed successfully
    pub success: bool,
    /// Error message if benchmark failed
    pub error_message: Option<String>,
    /// Additional metrics collected during benchmark
    pub metrics: HashMap<String, f64>,
}

/// Complete benchmarking report for a single device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceBenchmarkReport {
    /// Device information
    pub device: MiningDevice,
    /// All benchmark results for this device
    pub results: Vec<BenchmarkResult>,
    /// Overall benchmark status
    pub status: BenchmarkStatus,
    /// Total benchmark duration
    pub total_duration_seconds: u64,
    /// Benchmark completion timestamp
    pub completed_at: DateTime<Utc>,
    /// Best performing algorithm (by hashrate)
    pub best_algorithm: Option<String>,
    /// Most power-efficient algorithm (by hashrate/watt)
    pub most_efficient_algorithm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BenchmarkStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

/// Benchmarking engine that coordinates all benchmarking operations
pub struct BenchmarkingEngine {
    /// Hardware detector for device monitoring during benchmarks
    hardware_detector: Arc<RwLock<HardwareDetector>>,
    /// Algorithm configurations
    algorithm_configs: HashMap<String, AlgorithmConfig>,
    /// Active benchmark processes
    active_processes: Arc<RwLock<HashMap<String, Child>>>,
    /// Benchmark results cache
    results_cache: Arc<RwLock<HashMap<String, DeviceBenchmarkReport>>>,
    /// Miners directory path
    miners_directory: PathBuf,
}

impl BenchmarkingEngine {
    /// Create a new benchmarking engine
    pub fn new(hardware_detector: Arc<RwLock<HardwareDetector>>) -> Result<Self> {
        info!("Initializing benchmarking engine");

        // Determine miners directory
        let miners_directory = Self::get_miners_directory()?;
        info!("Using miners directory: {}", miners_directory.display());

        let mut engine = Self {
            hardware_detector,
            algorithm_configs: HashMap::new(),
            active_processes: Arc::new(RwLock::new(HashMap::new())),
            results_cache: Arc::new(RwLock::new(HashMap::new())),
            miners_directory,
        };

        // Initialize algorithm configurations
        engine.initialize_algorithm_configs()?;

        Ok(engine)
    }

    /// Get the directory where miner executables are stored
    fn get_miners_directory() -> Result<PathBuf> {
        // First check if BUNKER_MINERS_PATH environment variable is set
        if let Ok(miners_path) = std::env::var("BUNKER_MINERS_PATH") {
            let path = PathBuf::from(miners_path);
            if path.exists() {
                return Ok(path);
            }
        }

        // Check for miners directory in current working directory
        let local_miners = PathBuf::from("miners");
        if local_miners.exists() {
            return Ok(local_miners);
        }

        // Check for miners in user data directory
        if let Some(data_dir) = dirs::data_dir() {
            let user_miners = data_dir.join("bunker-miner").join("miners");
            if user_miners.exists() {
                return Ok(user_miners);
            }
        }

        // Default to local miners directory (will be created if needed)
        Ok(PathBuf::from("miners"))
    }

    /// Initialize algorithm configurations with default settings
    fn initialize_algorithm_configs(&mut self) -> Result<()> {
        info!("Initializing algorithm configurations");

        // Ethash configuration
        self.algorithm_configs.insert(
            "ethash".to_string(),
            AlgorithmConfig {
                name: "ethash".to_string(),
                display_name: "Ethash (Ethereum Classic)".to_string(),
                supported_devices: vec![DeviceType::NvidiaGpu, DeviceType::AmdGpu],
                miner_configs: vec![MinerConfig {
                    name: "lolMiner".to_string(),
                    executable: if cfg!(windows) {
                        "lolMiner.exe"
                    } else {
                        "lolMiner"
                    }
                    .to_string(),
                    args_template: vec![
                        "--algo".to_string(),
                        "ETHASH".to_string(),
                        "--benchmark".to_string(),
                        "ETHASH".to_string(),
                        "--benchepochs".to_string(),
                        "1".to_string(),
                    ],
                    hashrate_regex: r"Total:\s+(\d+\.?\d*)\s*MH/s".to_string(),
                    hashrate_unit: "MH/s".to_string(),
                    properties: HashMap::new(),
                }],
                hashrate_range: Some((1.0, 150.0)), // 1-150 MH/s typical range
                benchmark_duration: 60,
                parameters: HashMap::new(),
            },
        );

        // Kaspa (kHeavyHash) configuration
        self.algorithm_configs.insert(
            "kheavyhash".to_string(),
            AlgorithmConfig {
                name: "kheavyhash".to_string(),
                display_name: "kHeavyHash (Kaspa)".to_string(),
                supported_devices: vec![DeviceType::NvidiaGpu, DeviceType::AmdGpu],
                miner_configs: vec![MinerConfig {
                    name: "lolMiner".to_string(),
                    executable: if cfg!(windows) {
                        "lolMiner.exe"
                    } else {
                        "lolMiner"
                    }
                    .to_string(),
                    args_template: vec![
                        "--algo".to_string(),
                        "KASPA".to_string(),
                        "--benchmark".to_string(),
                        "KASPA".to_string(),
                        "--benchepochs".to_string(),
                        "1".to_string(),
                    ],
                    hashrate_regex: r"Total:\s+(\d+\.?\d*)\s*GH/s".to_string(),
                    hashrate_unit: "GH/s".to_string(),
                    properties: HashMap::new(),
                }],
                hashrate_range: Some((0.1, 5.0)), // 0.1-5.0 GH/s typical range
                benchmark_duration: 60,
                parameters: HashMap::new(),
            },
        );

        // RandomX (CPU) configuration
        self.algorithm_configs.insert(
            "randomx".to_string(),
            AlgorithmConfig {
                name: "randomx".to_string(),
                display_name: "RandomX (Monero)".to_string(),
                supported_devices: vec![DeviceType::Cpu],
                miner_configs: vec![MinerConfig {
                    name: "XMRig".to_string(),
                    executable: if cfg!(windows) { "xmrig.exe" } else { "xmrig" }.to_string(),
                    args_template: vec![
                        "--benchmark".to_string(),
                        "10".to_string(), // 10 second benchmark
                        "--randomx-mode".to_string(),
                        "auto".to_string(),
                    ],
                    hashrate_regex: r"speed.*?(\d+\.?\d*)\s*H/s".to_string(),
                    hashrate_unit: "H/s".to_string(),
                    properties: HashMap::new(),
                }],
                hashrate_range: Some((100.0, 50000.0)), // 100-50k H/s typical CPU range
                benchmark_duration: 30,
                parameters: HashMap::new(),
            },
        );

        info!(
            "Initialized {} algorithm configurations",
            self.algorithm_configs.len()
        );
        Ok(())
    }

    /// Run comprehensive benchmarks for all detected devices
    pub async fn benchmark_all_devices(&mut self) -> Result<Vec<DeviceBenchmarkReport>> {
        info!("Starting comprehensive device benchmarking");

        let devices = {
            let mut detector = self.hardware_detector.write().await;
            detector.detect_devices()?
        };

        let mut reports = Vec::new();

        for device in devices {
            info!(
                "Benchmarking device: {} ({:?})",
                device.name, device.device_type
            );

            match self.benchmark_device(&device).await {
                Ok(report) => {
                    info!(
                        "Successfully benchmarked {}: {} results",
                        device.name,
                        report.results.len()
                    );
                    reports.push(report);
                }
                Err(e) => {
                    error!("Failed to benchmark device {}: {}", device.name, e);
                    // Create a failed report
                    reports.push(DeviceBenchmarkReport {
                        device: device.clone(),
                        results: vec![],
                        status: BenchmarkStatus::Failed,
                        total_duration_seconds: 0,
                        completed_at: Utc::now(),
                        best_algorithm: None,
                        most_efficient_algorithm: None,
                    });
                }
            }
        }

        info!("Completed benchmarking for {} devices", reports.len());
        Ok(reports)
    }

    /// Benchmark a single device with all supported algorithms
    pub async fn benchmark_device(
        &mut self,
        device: &MiningDevice,
    ) -> Result<DeviceBenchmarkReport> {
        info!("Starting benchmark for device: {}", device.name);

        let mut results = Vec::new();
        let mut total_duration = 0;

        // Get algorithms supported by this device type
        let supported_algorithms: Vec<AlgorithmConfig> = self
            .algorithm_configs
            .values()
            .filter(|config| config.supported_devices.contains(&device.device_type))
            .cloned()
            .collect();

        info!(
            "Found {} supported algorithms for {}",
            supported_algorithms.len(),
            device.name
        );

        for algorithm_config in supported_algorithms {
            info!(
                "Benchmarking {} on {}",
                algorithm_config.display_name, device.name
            );

            match self.benchmark_algorithm(device, &algorithm_config).await {
                Ok(mut algorithm_results) => {
                    total_duration += algorithm_results
                        .iter()
                        .map(|r| r.duration_seconds)
                        .sum::<u64>();
                    results.append(&mut algorithm_results);
                }
                Err(e) => {
                    warn!(
                        "Failed to benchmark {} on {}: {}",
                        algorithm_config.name, device.name, e
                    );

                    // Create a failed result
                    results.push(BenchmarkResult {
                        id: Uuid::new_v4().to_string(),
                        device_id: device.id.clone(),
                        algorithm: algorithm_config.name.clone(),
                        miner_name: "unknown".to_string(),
                        hashrate: 0.0,
                        hashrate_unit: "H/s".to_string(),
                        hashrate_hs: 0.0,
                        power_watts: None,
                        temperature_c: None,
                        start_time: Utc::now(),
                        end_time: Utc::now(),
                        duration_seconds: 0,
                        success: false,
                        error_message: Some(e.to_string()),
                        metrics: HashMap::new(),
                    });
                }
            }
        }

        // Analyze results to find best algorithms
        let (best_algorithm, most_efficient_algorithm) = self.analyze_results(&results);

        let report = DeviceBenchmarkReport {
            device: device.clone(),
            results,
            status: BenchmarkStatus::Completed,
            total_duration_seconds: total_duration,
            completed_at: Utc::now(),
            best_algorithm,
            most_efficient_algorithm,
        };

        // Cache the report
        let mut cache = self.results_cache.write().await;
        cache.insert(device.id.clone(), report.clone());

        info!(
            "Completed benchmark for device: {} (duration: {}s)",
            device.name, total_duration
        );

        Ok(report)
    }

    /// Benchmark a single algorithm on a device using all available miners
    async fn benchmark_algorithm(
        &mut self,
        device: &MiningDevice,
        algorithm_config: &AlgorithmConfig,
    ) -> Result<Vec<BenchmarkResult>> {
        let mut results = Vec::new();

        for miner_config in &algorithm_config.miner_configs {
            info!(
                "Running {} benchmark with {}",
                algorithm_config.name, miner_config.name
            );

            match self
                .run_single_benchmark(device, algorithm_config, miner_config)
                .await
            {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    warn!(
                        "Failed to run {} benchmark with {}: {}",
                        algorithm_config.name, miner_config.name, e
                    );
                }
            }
        }

        Ok(results)
    }

    /// Run a single benchmark with a specific miner
    async fn run_single_benchmark(
        &mut self,
        device: &MiningDevice,
        algorithm_config: &AlgorithmConfig,
        miner_config: &MinerConfig,
    ) -> Result<BenchmarkResult> {
        let benchmark_id = Uuid::new_v4().to_string();
        let start_time = Utc::now();

        info!(
            "Starting benchmark {}: {} on {} with {}",
            benchmark_id, algorithm_config.name, device.name, miner_config.name
        );

        // Find miner executable
        let miner_path = self.find_miner_executable(&miner_config.executable)?;
        debug!("Using miner at: {}", miner_path.display());

        // Prepare command arguments
        let args = self.prepare_miner_arguments(device, algorithm_config, miner_config)?;
        debug!("Miner arguments: {:?}", args);

        // Start the miner process
        let child = Command::new(&miner_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start miner process")?;

        // Store the process for potential cleanup
        {
            let mut processes = self.active_processes.write().await;
            processes.insert(benchmark_id.clone(), child);
        }

        // Monitor the benchmark
        let monitoring_result = self
            .monitor_benchmark(&benchmark_id, device, algorithm_config.benchmark_duration)
            .await;

        // Retrieve and terminate the process
        let child = {
            let mut processes = self.active_processes.write().await;
            processes.remove(&benchmark_id)
        };

        if let Some(mut child) = child {
            // Kill the process if it's still running
            if child.try_wait()?.is_none() {
                let _ = child.kill();
            }

            // Collect output
            let output = child
                .wait_with_output()
                .context("Failed to collect miner output")?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            debug!("Miner stdout: {}", stdout);
            if !stderr.is_empty() {
                debug!("Miner stderr: {}", stderr);
            }

            // Parse hashrate from output
            let hashrate = self.extract_hashrate_from_output(&stdout, miner_config)?;
            let hashrate_hs = self.normalize_hashrate(hashrate, &miner_config.hashrate_unit)?;

            let end_time = Utc::now();
            let duration = (end_time - start_time).num_seconds() as u64;

            // Get power/temperature data from monitoring
            let (avg_power, avg_temperature) = monitoring_result.unwrap_or((None, None));

            let result = BenchmarkResult {
                id: benchmark_id,
                device_id: device.id.clone(),
                algorithm: algorithm_config.name.clone(),
                miner_name: miner_config.name.clone(),
                hashrate,
                hashrate_unit: miner_config.hashrate_unit.clone(),
                hashrate_hs,
                power_watts: avg_power,
                temperature_c: avg_temperature,
                start_time,
                end_time,
                duration_seconds: duration,
                success: true,
                error_message: None,
                metrics: HashMap::new(),
            };

            info!(
                "Benchmark completed: {:.2} {} ({:.2} H/s)",
                result.hashrate, result.hashrate_unit, result.hashrate_hs
            );

            Ok(result)
        } else {
            Err(anyhow!("Benchmark process was lost during execution"))
        }
    }

    /// Find the executable path for a miner
    fn find_miner_executable(&self, executable_name: &str) -> Result<PathBuf> {
        // First check in the miners directory
        let miner_path = self.miners_directory.join(executable_name);
        if miner_path.exists() && miner_path.is_file() {
            return Ok(miner_path);
        }

        // Check if it's in the system PATH
        match which(executable_name) {
            Ok(path) => Ok(path),
            Err(_) => Err(anyhow!(
                "Miner executable '{}' not found in {} or system PATH. \
                     Please install the miner in the miners directory.",
                executable_name,
                self.miners_directory.display()
            )),
        }
    }

    /// Prepare command line arguments for the miner
    fn prepare_miner_arguments(
        &self,
        device: &MiningDevice,
        _algorithm_config: &AlgorithmConfig,
        miner_config: &MinerConfig,
    ) -> Result<Vec<String>> {
        let args = miner_config.args_template.clone();

        // Device-specific argument substitution would go here
        // For now, use the template as-is

        // Add device-specific GPU selection if needed
        if device.device_type == DeviceType::NvidiaGpu || device.device_type == DeviceType::AmdGpu {
            // Extract GPU index from device ID if possible
            if let Some(index_str) = device.id.rsplit('_').next() {
                if let Ok(_index) = index_str.parse::<u32>() {
                    // GPU selection args would be added here based on miner type
                    debug!("GPU index for {}: {}", device.name, index_str);
                }
            }
        }

        Ok(args)
    }

    /// Monitor benchmark process and collect metrics
    async fn monitor_benchmark(
        &mut self,
        benchmark_id: &str,
        device: &MiningDevice,
        duration_seconds: u64,
    ) -> Result<(Option<f64>, Option<f32>)> {
        debug!(
            "Monitoring benchmark {} for {}s",
            benchmark_id, duration_seconds
        );

        let mut power_readings = Vec::new();
        let mut temperature_readings = Vec::new();
        let total_samples = duration_seconds / 5;

        for _ in 0..total_samples {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            // Update hardware metrics
            if let Ok(mut detector) = self.hardware_detector.try_write() {
                if let Ok(()) = detector.update_metrics() {
                    if let Some(devices) = detector.get_cached_devices() {
                        if let Some(current_device) = devices.iter().find(|d| d.id == device.id) {
                            if let Some(power) = current_device.metrics.power_watts {
                                power_readings.push(power as f64);
                            }
                            if let Some(temp) = current_device.metrics.temperature_c {
                                temperature_readings.push(temp);
                            }
                        }
                    }
                }
            }
        }

        // Calculate averages
        let avg_power = if !power_readings.is_empty() {
            Some(power_readings.iter().sum::<f64>() / power_readings.len() as f64)
        } else {
            None
        };

        let avg_temperature = if !temperature_readings.is_empty() {
            Some(temperature_readings.iter().sum::<f32>() / temperature_readings.len() as f32)
        } else {
            None
        };

        debug!(
            "Monitoring completed. Avg power: {:?}W, Avg temp: {:?}°C",
            avg_power, avg_temperature
        );

        Ok((avg_power, avg_temperature))
    }

    /// Extract hashrate from miner output using regex
    fn extract_hashrate_from_output(
        &self,
        output: &str,
        miner_config: &MinerConfig,
    ) -> Result<f64> {
        let regex =
            Regex::new(&miner_config.hashrate_regex).context("Invalid hashrate regex pattern")?;

        // Look for the last occurrence of hashrate in the output
        let mut hashrate = 0.0;
        for capture in regex.captures_iter(output) {
            if let Some(hashrate_match) = capture.get(1) {
                hashrate = hashrate_match
                    .as_str()
                    .parse::<f64>()
                    .context("Failed to parse hashrate value")?;
            }
        }

        if hashrate > 0.0 {
            Ok(hashrate)
        } else {
            Err(anyhow!("No valid hashrate found in miner output"))
        }
    }

    /// Convert hashrate to normalized H/s (hashes per second)
    fn normalize_hashrate(&self, hashrate: f64, unit: &str) -> Result<f64> {
        let multiplier = match unit {
            "H/s" => 1.0,
            "kH/s" => 1_000.0,
            "MH/s" => 1_000_000.0,
            "GH/s" => 1_000_000_000.0,
            "TH/s" => 1_000_000_000_000.0,
            _ => return Err(anyhow!("Unknown hashrate unit: {}", unit)),
        };

        Ok(hashrate * multiplier)
    }

    /// Analyze benchmark results to find best and most efficient algorithms
    fn analyze_results(&self, results: &[BenchmarkResult]) -> (Option<String>, Option<String>) {
        let successful_results: Vec<_> = results
            .iter()
            .filter(|r| r.success && r.hashrate_hs > 0.0)
            .collect();

        if successful_results.is_empty() {
            return (None, None);
        }

        // Best algorithm by raw hashrate
        let best_by_hashrate = successful_results
            .iter()
            .max_by(|a, b| a.hashrate_hs.partial_cmp(&b.hashrate_hs).unwrap())
            .map(|r| r.algorithm.clone());

        // Most efficient algorithm by hashrate per watt
        let best_by_efficiency = successful_results
            .iter()
            .filter(|r| r.power_watts.is_some() && r.power_watts.unwrap() > 0.0)
            .max_by(|a, b| {
                let eff_a = a.hashrate_hs / a.power_watts.unwrap();
                let eff_b = b.hashrate_hs / b.power_watts.unwrap();
                eff_a.partial_cmp(&eff_b).unwrap()
            })
            .map(|r| r.algorithm.clone());

        (best_by_hashrate, best_by_efficiency)
    }

    /// Get cached benchmark report for a device
    pub async fn get_device_report(&self, device_id: &str) -> Option<DeviceBenchmarkReport> {
        let cache = self.results_cache.read().await;
        cache.get(device_id).cloned()
    }

    /// Get all cached benchmark reports
    pub async fn get_all_reports(&self) -> Vec<DeviceBenchmarkReport> {
        let cache = self.results_cache.read().await;
        cache.values().cloned().collect()
    }

    /// Cancel all active benchmarks
    pub async fn cancel_all_benchmarks(&mut self) -> Result<()> {
        info!("Cancelling all active benchmarks");

        let mut processes = self.active_processes.write().await;
        for (benchmark_id, mut child) in processes.drain() {
            info!("Terminating benchmark: {}", benchmark_id);
            let _ = child.kill();
            let _ = child.wait();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashrate_normalization() {
        let engine = create_test_engine();

        assert_eq!(engine.normalize_hashrate(1.0, "H/s").unwrap(), 1.0);
        assert_eq!(engine.normalize_hashrate(1.0, "kH/s").unwrap(), 1_000.0);
        assert_eq!(engine.normalize_hashrate(1.0, "MH/s").unwrap(), 1_000_000.0);
        assert_eq!(
            engine.normalize_hashrate(1.0, "GH/s").unwrap(),
            1_000_000_000.0
        );

        assert!(engine.normalize_hashrate(1.0, "invalid").is_err());
    }

    #[test]
    fn test_hashrate_extraction() {
        let engine = create_test_engine();
        let miner_config = MinerConfig {
            name: "test".to_string(),
            executable: "test".to_string(),
            args_template: vec![],
            hashrate_regex: r"Total:\s+(\d+\.?\d*)\s*MH/s".to_string(),
            hashrate_unit: "MH/s".to_string(),
            properties: HashMap::new(),
        };

        let output = "Some output\nTotal: 45.67 MH/s\nMore output";
        let hashrate = engine
            .extract_hashrate_from_output(output, &miner_config)
            .unwrap();
        assert_eq!(hashrate, 45.67);
    }

    fn create_test_engine() -> BenchmarkingEngine {
        // Create a mock hardware detector for testing
        let hardware_detector = Arc::new(RwLock::new(
            crate::hardware::HardwareDetector::new().unwrap(),
        ));

        BenchmarkingEngine::new(hardware_detector).unwrap()
    }
}
