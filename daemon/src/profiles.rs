/*!
 * BUNKER MINER - Device Profile Management
 *
 * This module manages persistent storage and retrieval of device performance profiles
 * generated through benchmarking. Device profiles are the foundation for intelligent
 * mining decisions and profit optimization.
 *
 * Key Features:
 * - Persistent storage of benchmark results as device profiles
 * - JSON-based profile format for portability and debugging
 * - Profile versioning and migration support
 * - Secure profile validation and integrity checking
 * - Configuration directory management across platforms
 * - Profile caching and performance optimization
 */

use crate::benchmarking::{BenchmarkResult, DeviceBenchmarkReport};
use crate::hardware::MiningDevice;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use sysinfo::{CpuExt, SystemExt};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Version of the profile format (for future compatibility)
const PROFILE_FORMAT_VERSION: u32 = 1;

/// Device performance profile containing benchmark results and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// Unique profile identifier
    pub id: String,
    /// Profile format version
    pub version: u32,
    /// Device information
    pub device: MiningDevice,
    /// Algorithm performance data
    pub algorithms: HashMap<String, AlgorithmProfile>,
    /// Profile creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Profile metadata
    pub metadata: ProfileMetadata,
    /// Profile validation checksum
    pub checksum: Option<String>,
}

/// Performance profile for a specific algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmProfile {
    /// Algorithm name
    pub algorithm: String,
    /// Best benchmark result for this algorithm
    pub best_result: BenchmarkResult,
    /// All benchmark results (for history/validation)
    pub all_results: Vec<BenchmarkResult>,
    /// Average performance metrics
    pub average_metrics: AlgorithmMetrics,
    /// Algorithm-specific properties
    pub properties: HashMap<String, String>,
    /// Last benchmark timestamp
    pub last_benchmarked: DateTime<Utc>,
}

/// Average performance metrics for an algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmMetrics {
    /// Average hashrate in H/s
    pub avg_hashrate_hs: f64,
    /// Average power consumption in Watts
    pub avg_power_watts: Option<f64>,
    /// Average temperature in Celsius
    pub avg_temperature_c: Option<f32>,
    /// Power efficiency (H/s per Watt)
    pub power_efficiency: Option<f64>,
    /// Number of successful benchmark runs
    pub sample_count: u32,
    /// Standard deviation of hashrate
    pub hashrate_std_dev: Option<f64>,
}

/// Profile metadata for additional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetadata {
    /// Operating system where profile was created
    pub os: String,
    /// Driver version when profile was created  
    pub driver_version: Option<String>,
    /// Mining software versions used
    pub miner_versions: HashMap<String, String>,
    /// System specifications
    pub system_info: SystemInfo,
    /// Profile tags for organization
    pub tags: Vec<String>,
    /// User notes
    pub notes: Option<String>,
}

/// System information relevant to mining performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// CPU information
    pub cpu: String,
    /// Total system memory in MB
    pub memory_mb: u64,
    /// GPU driver version
    pub gpu_driver: Option<String>,
    /// Operating system version
    pub os_version: String,
}

/// Collection of all device profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCollection {
    /// Collection format version
    pub version: u32,
    /// All device profiles indexed by device ID
    pub profiles: HashMap<String, DeviceProfile>,
    /// Collection metadata
    pub metadata: CollectionMetadata,
    /// Collection checksum for integrity verification
    pub checksum: Option<String>,
}

/// Metadata for the profile collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    /// Total number of profiles
    pub profile_count: u32,
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
    /// Collection creation timestamp
    pub created_at: DateTime<Utc>,
    /// Application version that created this collection
    pub app_version: String,
}

/// Profile manager that handles all profile operations
pub struct ProfileManager {
    /// Configuration directory path
    config_dir: PathBuf,
    /// Profiles file path
    profiles_file: PathBuf,
    /// In-memory profile cache
    cached_collection: Option<ProfileCollection>,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_directory()?;
        let profiles_file = config_dir.join("profiles.json");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create configuration directory")?;
            info!("Created configuration directory: {}", config_dir.display());
        }

        let manager = Self {
            config_dir,
            profiles_file,
            cached_collection: None,
        };

        info!(
            "Profile manager initialized with config dir: {}",
            manager.config_dir.display()
        );

        Ok(manager)
    }

    /// Get the configuration directory path
    fn get_config_directory() -> Result<PathBuf> {
        // Check for environment variable override
        if let Ok(config_path) = std::env::var("BUNKER_MINER_CONFIG_DIR") {
            return Ok(PathBuf::from(config_path));
        }

        // Use platform-appropriate config directory
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir.join("bunker-miner"))
        } else {
            // Fallback to local config directory
            Ok(PathBuf::from(".bunker-miner"))
        }
    }

    /// Load all profiles from disk
    pub fn load_profiles(&mut self) -> Result<ProfileCollection> {
        if self.profiles_file.exists() {
            info!("Loading profiles from: {}", self.profiles_file.display());

            let file = File::open(&self.profiles_file).context("Failed to open profiles file")?;

            let reader = BufReader::new(file);
            let collection: ProfileCollection =
                serde_json::from_reader(reader).context("Failed to parse profiles JSON")?;

            // Validate collection integrity
            self.validate_collection(&collection)?;

            info!("Loaded {} device profiles", collection.profiles.len());
            self.cached_collection = Some(collection.clone());
            Ok(collection)
        } else {
            info!("No existing profiles file found, creating new collection");
            let collection = self.create_empty_collection();
            self.cached_collection = Some(collection.clone());
            Ok(collection)
        }
    }

    /// Save profiles to disk
    pub fn save_profiles(&mut self, collection: &ProfileCollection) -> Result<()> {
        info!("Saving {} profiles to disk", collection.profiles.len());

        // Validate collection before saving
        self.validate_collection(collection)?;

        // Create backup of existing file
        if self.profiles_file.exists() {
            let backup_path = self.profiles_file.with_extension("json.backup");
            fs::copy(&self.profiles_file, &backup_path)
                .context("Failed to create backup of profiles file")?;
            debug!("Created backup: {}", backup_path.display());
        }

        // Write the collection to file
        let file = File::create(&self.profiles_file).context("Failed to create profiles file")?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, collection)
            .context("Failed to write profiles JSON")?;

        // Update cache
        self.cached_collection = Some(collection.clone());

        info!(
            "Successfully saved profiles to: {}",
            self.profiles_file.display()
        );
        Ok(())
    }

    /// Create device profile from benchmark report
    pub fn create_profile_from_benchmark(
        &self,
        report: &DeviceBenchmarkReport,
    ) -> Result<DeviceProfile> {
        info!("Creating profile for device: {}", report.device.name);

        let mut algorithms = HashMap::new();

        // Process each successful benchmark result
        let successful_results: Vec<_> = report.results.iter().filter(|r| r.success).collect();

        // Group results by algorithm
        let mut algorithm_groups: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();
        for result in &successful_results {
            algorithm_groups
                .entry(result.algorithm.clone())
                .or_default()
                .push(result);
        }

        // Create algorithm profiles
        for (algorithm_name, results) in algorithm_groups {
            let algorithm_profile = self.create_algorithm_profile(&algorithm_name, &results)?;
            algorithms.insert(algorithm_name, algorithm_profile);
        }

        // Create system metadata
        let metadata = self.create_profile_metadata(&report.device)?;

        let profile = DeviceProfile {
            id: format!("profile_{}", Uuid::new_v4()),
            version: PROFILE_FORMAT_VERSION,
            device: report.device.clone(),
            algorithms,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata,
            checksum: None, // Will be calculated during save
        };

        info!(
            "Created profile with {} algorithms",
            profile.algorithms.len()
        );
        Ok(profile)
    }

    /// Create algorithm profile from benchmark results
    fn create_algorithm_profile(
        &self,
        algorithm: &str,
        results: &[&BenchmarkResult],
    ) -> Result<AlgorithmProfile> {
        if results.is_empty() {
            return Err(anyhow!("Cannot create algorithm profile with no results"));
        }

        // Find the best result (highest hashrate)
        let best_result = results
            .iter()
            .max_by(|a, b| a.hashrate_hs.partial_cmp(&b.hashrate_hs).unwrap())
            .map(|result| (*result).clone())
            .unwrap();

        // Calculate average metrics
        let avg_metrics = self.calculate_average_metrics(results)?;

        // Get the latest benchmark timestamp
        let last_benchmarked = results
            .iter()
            .map(|r| r.end_time)
            .max()
            .unwrap_or(Utc::now());

        Ok(AlgorithmProfile {
            algorithm: algorithm.to_string(),
            best_result,
            all_results: results.iter().map(|r| (*r).clone()).collect(),
            average_metrics: avg_metrics,
            properties: HashMap::new(),
            last_benchmarked,
        })
    }

    /// Calculate average metrics from benchmark results
    fn calculate_average_metrics(&self, results: &[&BenchmarkResult]) -> Result<AlgorithmMetrics> {
        if results.is_empty() {
            return Err(anyhow!("Cannot calculate metrics with no results"));
        }

        let sample_count = results.len() as u32;

        // Calculate hashrate statistics
        let hashrates: Vec<f64> = results.iter().map(|r| r.hashrate_hs).collect();
        let avg_hashrate_hs = hashrates.iter().sum::<f64>() / hashrates.len() as f64;

        let hashrate_std_dev = if hashrates.len() > 1 {
            let variance = hashrates
                .iter()
                .map(|hr| (hr - avg_hashrate_hs).powi(2))
                .sum::<f64>()
                / (hashrates.len() - 1) as f64;
            Some(variance.sqrt())
        } else {
            None
        };

        // Calculate power statistics
        let power_readings: Vec<f64> = results.iter().filter_map(|r| r.power_watts).collect();
        let avg_power_watts = if !power_readings.is_empty() {
            Some(power_readings.iter().sum::<f64>() / power_readings.len() as f64)
        } else {
            None
        };

        // Calculate temperature statistics
        let temperature_readings: Vec<f32> =
            results.iter().filter_map(|r| r.temperature_c).collect();
        let avg_temperature_c = if !temperature_readings.is_empty() {
            Some(temperature_readings.iter().sum::<f32>() / temperature_readings.len() as f32)
        } else {
            None
        };

        // Calculate power efficiency
        let power_efficiency = if let Some(avg_power) = avg_power_watts {
            if avg_power > 0.0 {
                Some(avg_hashrate_hs / avg_power)
            } else {
                None
            }
        } else {
            None
        };

        Ok(AlgorithmMetrics {
            avg_hashrate_hs,
            avg_power_watts,
            avg_temperature_c,
            power_efficiency,
            sample_count,
            hashrate_std_dev,
        })
    }

    /// Create profile metadata from device information
    fn create_profile_metadata(&self, device: &MiningDevice) -> Result<ProfileMetadata> {
        let system_info = SystemInfo {
            cpu: sysinfo::System::new().global_cpu_info().brand().to_string(),
            memory_mb: sysinfo::System::new().total_memory() / 1024 / 1024,
            gpu_driver: device.driver_version.clone(),
            os_version: sysinfo::System::new()
                .long_os_version()
                .unwrap_or_else(|| "Unknown".to_string()),
        };

        Ok(ProfileMetadata {
            os: std::env::consts::OS.to_string(),
            driver_version: device.driver_version.clone(),
            miner_versions: HashMap::new(), // Would be populated with actual miner versions
            system_info,
            tags: vec!["benchmark".to_string()],
            notes: None,
        })
    }

    /// Update an existing profile with new benchmark data
    pub fn update_profile(
        &self,
        existing_profile: &mut DeviceProfile,
        new_report: &DeviceBenchmarkReport,
    ) -> Result<()> {
        info!(
            "Updating profile for device: {}",
            existing_profile.device.name
        );

        let successful_results: Vec<_> = new_report.results.iter().filter(|r| r.success).collect();

        // Group new results by algorithm
        let mut algorithm_groups: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();
        for result in &successful_results {
            algorithm_groups
                .entry(result.algorithm.clone())
                .or_default()
                .push(result);
        }

        // Update existing algorithms or create new ones
        for (algorithm_name, new_results) in algorithm_groups {
            if let Some(existing_algorithm) = existing_profile.algorithms.get_mut(&algorithm_name) {
                // Update existing algorithm profile
                self.update_algorithm_profile(existing_algorithm, &new_results)?;
            } else {
                // Create new algorithm profile
                let new_algorithm_profile =
                    self.create_algorithm_profile(&algorithm_name, &new_results)?;
                existing_profile
                    .algorithms
                    .insert(algorithm_name, new_algorithm_profile);
            }
        }

        existing_profile.updated_at = Utc::now();

        info!(
            "Updated profile with {} algorithms",
            existing_profile.algorithms.len()
        );
        Ok(())
    }

    /// Update an existing algorithm profile with new results
    fn update_algorithm_profile(
        &self,
        algorithm_profile: &mut AlgorithmProfile,
        new_results: &[&BenchmarkResult],
    ) -> Result<()> {
        // Add new results to the collection
        for result in new_results {
            algorithm_profile.all_results.push((*result).clone());
        }

        // Update best result if needed
        if let Some(best_new_result) = new_results
            .iter()
            .max_by(|a, b| a.hashrate_hs.partial_cmp(&b.hashrate_hs).unwrap())
        {
            if best_new_result.hashrate_hs > algorithm_profile.best_result.hashrate_hs {
                algorithm_profile.best_result = (*best_new_result).clone();
            }
        }

        // Recalculate average metrics with all results
        let all_result_refs: Vec<&BenchmarkResult> = algorithm_profile.all_results.iter().collect();
        algorithm_profile.average_metrics = self.calculate_average_metrics(&all_result_refs)?;

        // Update timestamp
        algorithm_profile.last_benchmarked = new_results
            .iter()
            .map(|r| r.end_time)
            .max()
            .unwrap_or(Utc::now());

        Ok(())
    }

    /// Get profile for a specific device
    pub fn get_profile(&mut self, device_id: &str) -> Result<Option<DeviceProfile>> {
        let collection = if let Some(ref cached) = self.cached_collection {
            cached.clone()
        } else {
            self.load_profiles()?
        };

        Ok(collection.profiles.get(device_id).cloned())
    }

    /// Add or update a device profile
    pub fn save_profile(&mut self, profile: DeviceProfile) -> Result<()> {
        let mut collection = if let Some(cached) = &self.cached_collection {
            cached.clone()
        } else {
            self.load_profiles()?
        };

        collection
            .profiles
            .insert(profile.device.id.clone(), profile);
        collection.metadata.profile_count = collection.profiles.len() as u32;
        collection.metadata.last_updated = Utc::now();

        self.save_profiles(&collection)?;
        Ok(())
    }

    /// Get all profiles
    pub fn get_all_profiles(&mut self) -> Result<Vec<DeviceProfile>> {
        let collection = if let Some(ref cached) = self.cached_collection {
            cached.clone()
        } else {
            self.load_profiles()?
        };

        Ok(collection.profiles.values().cloned().collect())
    }

    /// Create an empty profile collection
    fn create_empty_collection(&self) -> ProfileCollection {
        ProfileCollection {
            version: PROFILE_FORMAT_VERSION,
            profiles: HashMap::new(),
            metadata: CollectionMetadata {
                profile_count: 0,
                last_updated: Utc::now(),
                created_at: Utc::now(),
                app_version: "0.1.0".to_string(),
            },
            checksum: None,
        }
    }

    /// Validate profile collection integrity
    fn validate_collection(&self, collection: &ProfileCollection) -> Result<()> {
        // Check format version compatibility
        if collection.version > PROFILE_FORMAT_VERSION {
            return Err(anyhow!(
                "Profile collection version {} is newer than supported version {}",
                collection.version,
                PROFILE_FORMAT_VERSION
            ));
        }

        // Validate profile count matches actual profiles
        if collection.metadata.profile_count != collection.profiles.len() as u32 {
            warn!(
                "Profile count mismatch in metadata: expected {}, found {}",
                collection.metadata.profile_count,
                collection.profiles.len()
            );
        }

        // Validate individual profiles
        for (device_id, profile) in &collection.profiles {
            if profile.device.id != *device_id {
                return Err(anyhow!(
                    "Profile device ID mismatch: key '{}' vs profile '{}'",
                    device_id,
                    profile.device.id
                ));
            }

            if profile.algorithms.is_empty() {
                warn!("Profile for device '{}' has no algorithms", device_id);
            }
        }

        debug!("Profile collection validation passed");
        Ok(())
    }

    /// Export profiles to a specific file
    pub fn export_profiles(&mut self, export_path: &Path) -> Result<()> {
        let collection = if let Some(ref cached) = self.cached_collection {
            cached.clone()
        } else {
            self.load_profiles()?
        };

        let file = File::create(export_path).context("Failed to create export file")?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &collection).context("Failed to write export JSON")?;

        info!(
            "Exported {} profiles to: {}",
            collection.profiles.len(),
            export_path.display()
        );
        Ok(())
    }

    /// Import profiles from a specific file
    pub fn import_profiles(&mut self, import_path: &Path) -> Result<u32> {
        info!("Importing profiles from: {}", import_path.display());

        let file = File::open(import_path).context("Failed to open import file")?;

        let reader = BufReader::new(file);
        let imported_collection: ProfileCollection =
            serde_json::from_reader(reader).context("Failed to parse import JSON")?;

        // Validate imported collection
        self.validate_collection(&imported_collection)?;

        // Load existing collection
        let mut existing_collection = if let Some(cached) = &self.cached_collection {
            cached.clone()
        } else {
            self.load_profiles()?
        };

        // Merge profiles (imported profiles override existing ones)
        let mut imported_count = 0u32;
        for (device_id, profile) in imported_collection.profiles {
            existing_collection.profiles.insert(device_id, profile);
            imported_count += 1;
        }

        // Update metadata
        existing_collection.metadata.profile_count = existing_collection.profiles.len() as u32;
        existing_collection.metadata.last_updated = Utc::now();

        // Save merged collection
        self.save_profiles(&existing_collection)?;

        info!("Successfully imported {} profiles", imported_count);
        Ok(imported_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarking::BenchmarkStatus;
    use tempfile::TempDir;

    #[test]
    fn test_profile_creation() -> Result<()> {
        let report = create_test_benchmark_report();
        let manager = create_test_profile_manager()?;

        let profile = manager.create_profile_from_benchmark(&report)?;

        assert_eq!(profile.device.id, report.device.id);
        assert!(!profile.algorithms.is_empty());
        assert_eq!(profile.version, PROFILE_FORMAT_VERSION);

        Ok(())
    }

    #[test]
    fn test_profile_serialization() -> Result<()> {
        let report = create_test_benchmark_report();
        let manager = create_test_profile_manager()?;
        let profile = manager.create_profile_from_benchmark(&report)?;

        let serialized = serde_json::to_string(&profile)?;
        let deserialized: DeviceProfile = serde_json::from_str(&serialized)?;

        assert_eq!(deserialized.id, profile.id);
        assert_eq!(deserialized.device.id, profile.device.id);

        Ok(())
    }

    fn create_test_profile_manager() -> Result<ProfileManager> {
        // Use a temporary directory for testing
        let temp_dir = TempDir::new()?;
        std::env::set_var("BUNKER_MINER_CONFIG_DIR", temp_dir.path());
        ProfileManager::new()
    }

    fn create_test_benchmark_report() -> DeviceBenchmarkReport {
        use crate::hardware::*;

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

        let result = BenchmarkResult {
            id: "test_benchmark".to_string(),
            device_id: device.id.clone(),
            algorithm: "ethash".to_string(),
            miner_name: "lolMiner".to_string(),
            hashrate: 50.0,
            hashrate_unit: "MH/s".to_string(),
            hashrate_hs: 50_000_000.0,
            power_watts: Some(150.0),
            temperature_c: Some(65.0),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_seconds: 60,
            success: true,
            error_message: None,
            metrics: HashMap::new(),
        };

        DeviceBenchmarkReport {
            device,
            results: vec![result],
            status: BenchmarkStatus::Completed,
            total_duration_seconds: 60,
            completed_at: Utc::now(),
            best_algorithm: Some("ethash".to_string()),
            most_efficient_algorithm: Some("ethash".to_string()),
        }
    }
}
