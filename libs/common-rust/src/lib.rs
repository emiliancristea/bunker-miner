//! BUNKER MINER Common Library
//! 
//! Shared types, utilities, and constants used across BUNKER MINER components.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Mining algorithms supported by BUNKER MINER
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Algorithm {
    #[serde(rename = "kaspa")]
    Kaspa,
    #[serde(rename = "ethash")]
    Ethash,
    #[serde(rename = "etchash")]
    EtcHash,
    #[serde(rename = "randomx")]
    RandomX,
    #[serde(rename = "kawpow")]
    Kawpow,
    #[serde(rename = "flux")]
    Flux,
    #[serde(rename = "octopus")]
    Octopus,
}

/// Hardware types that can be used for mining
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareType {
    #[serde(rename = "nvidia_gpu")]
    NvidiaGpu,
    #[serde(rename = "amd_gpu")]
    AmdGpu,
    #[serde(rename = "cpu")]
    Cpu,
}

/// Device information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub name: String,
    pub hardware_type: HardwareType,
    pub vram_mb: Option<u32>,
    pub core_count: Option<u32>,
    pub driver_version: Option<String>,
}

/// Real-time telemetry data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Telemetry {
    pub device_id: String,
    pub algorithm: Algorithm,
    pub hashrate_mhs: f64,
    pub power_watts: u32,
    pub temperature_celsius: u32,
    pub fan_speed_percent: u32,
    pub shares: ShareStats,
    pub timestamp: DateTime<Utc>,
}

/// Share statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareStats {
    pub accepted: u64,
    pub rejected: u64,
    pub stale: u64,
}

/// Profitability information for an algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitabilityInfo {
    pub algorithm: Algorithm,
    pub coin: String,
    pub revenue_eur_day: f64,
    pub cost_eur_day: f64,
    pub profit_eur_day: f64,
}

/// Configuration for a mining operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    pub algorithm: Algorithm,
    pub pool_url: String,
    pub wallet_address: String,
    pub worker_name: Option<String>,
    pub devices: Vec<String>, // Device IDs to use
}

/// Common error types
#[derive(thiserror::Error, Debug)]
pub enum BunkerError {
    #[error("Hardware detection failed: {0}")]
    HardwareDetection(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Mining process error: {0}")]
    MiningProcess(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Security error: {0}")]
    Security(String),
}

/// Result type for BUNKER MINER operations
pub type BunkerResult<T> = Result<T, BunkerError>;

/// Constants used throughout the application
pub mod constants {
    /// Default gRPC port for daemon API
    pub const DEFAULT_GRPC_PORT: u16 = 50051;
    
    /// Default web dashboard port
    pub const DEFAULT_WEB_PORT: u16 = 8080;
    
    /// Configuration file name
    pub const CONFIG_FILE: &str = "config.toml";
    
    /// Profiles file name
    pub const PROFILES_FILE: &str = "profiles.json";
    
    /// Maximum number of concurrent mining processes
    pub const MAX_MINING_PROCESSES: usize = 16;
    
    /// Default profit switching threshold (5%)
    pub const DEFAULT_PROFIT_THRESHOLD: f64 = 0.05;
    
    /// Default minimum dwell time (5 minutes)
    pub const DEFAULT_MIN_DWELL_TIME_SECS: u64 = 300;
}

impl Default for ShareStats {
    fn default() -> Self {
        Self {
            accepted: 0,
            rejected: 0,
            stale: 0,
        }
    }
}

impl DeviceInfo {
    /// Create a new device info instance
    pub fn new(device_id: String, name: String, hardware_type: HardwareType) -> Self {
        Self {
            device_id,
            name,
            hardware_type,
            vram_mb: None,
            core_count: None,
            driver_version: None,
        }
    }
}

impl Telemetry {
    /// Create a new telemetry instance
    pub fn new(device_id: String, algorithm: Algorithm) -> Self {
        Self {
            device_id,
            algorithm,
            hashrate_mhs: 0.0,
            power_watts: 0,
            temperature_celsius: 0,
            fan_speed_percent: 0,
            shares: ShareStats::default(),
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let device = DeviceInfo::new(
            "gpu0".to_string(),
            "RTX 4090".to_string(),
            HardwareType::NvidiaGpu,
        );
        
        assert_eq!(device.device_id, "gpu0");
        assert_eq!(device.name, "RTX 4090");
        assert_eq!(device.hardware_type, HardwareType::NvidiaGpu);
    }

    #[test]
    fn test_telemetry_creation() {
        let telemetry = Telemetry::new("gpu0".to_string(), Algorithm::Kaspa);
        
        assert_eq!(telemetry.device_id, "gpu0");
        assert_eq!(telemetry.algorithm, Algorithm::Kaspa);
        assert_eq!(telemetry.hashrate_mhs, 0.0);
    }

    #[test]
    fn test_algorithm_serialization() {
        let algorithm = Algorithm::Kaspa;
        let serialized = serde_json::to_string(&algorithm).unwrap();
        let deserialized: Algorithm = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(algorithm, deserialized);
    }
}