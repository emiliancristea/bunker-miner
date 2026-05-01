use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Standardized telemetry data structure for fleet management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryData {
    /// Timestamp of telemetry collection
    pub timestamp: DateTime<Utc>,
    /// Current mining algorithm
    pub algorithm: String,
    /// Total hashrate across all devices (H/s)
    pub total_hashrate: f64,
    /// Total power consumption (W)
    pub total_power: f64,
    /// Average temperature across all devices (°C)
    pub avg_temperature: f64,
    /// Number of active devices
    pub device_count: u32,
    /// Accepted shares count
    pub shares_accepted: u64,
    /// Rejected shares count
    pub shares_rejected: u64,
    /// Current mining pool URL
    pub pool_url: String,
    /// Estimated daily profit in EUR
    pub profit_eur_day: Option<f64>,
    /// Per-device telemetry details
    pub device_telemetry: Vec<DeviceTelemetry>,
}

/// Per-device telemetry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTelemetry {
    /// Device identifier
    pub device_id: String,
    /// Human-readable device name
    pub device_name: String,
    /// Device hashrate (H/s)
    pub hashrate: f64,
    /// Device power consumption (W)
    pub power: f64,
    /// Device temperature (°C)
    pub temperature: f64,
    /// Fan speed (RPM)
    pub fan_speed: u32,
    /// GPU utilization percentage (0-100)
    pub utilization: u32,
    /// Device status
    pub status: String,
    /// Memory usage (MB)
    pub memory_used: u64,
    /// Memory total (MB)
    pub memory_total: u64,
    /// Clock speeds
    pub core_clock: u32,
    pub memory_clock: u32,
}

/// Telemetry collector that aggregates data from various sources
pub struct TelemetryCollector {
    /// Current telemetry data
    current_data: TelemetryData,
}

impl Default for TelemetryData {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            algorithm: "unknown".to_string(),
            total_hashrate: 0.0,
            total_power: 0.0,
            avg_temperature: 0.0,
            device_count: 0,
            shares_accepted: 0,
            shares_rejected: 0,
            pool_url: "unknown".to_string(),
            profit_eur_day: None,
            device_telemetry: Vec::new(),
        }
    }
}

impl Default for DeviceTelemetry {
    fn default() -> Self {
        Self {
            device_id: "unknown".to_string(),
            device_name: "Unknown Device".to_string(),
            hashrate: 0.0,
            power: 0.0,
            temperature: 0.0,
            fan_speed: 0,
            utilization: 0,
            status: "idle".to_string(),
            memory_used: 0,
            memory_total: 0,
            core_clock: 0,
            memory_clock: 0,
        }
    }
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new() -> Self {
        Self {
            current_data: TelemetryData::default(),
        }
    }

    /// Update telemetry from miner output data
    pub fn update_from_miner_telemetry(&mut self, miner_telemetry: &crate::miners::Telemetry) {
        self.current_data.timestamp = Utc::now();
        self.current_data.algorithm = miner_telemetry.algorithm.clone();
        self.current_data.total_hashrate = miner_telemetry.hashrate;
        self.current_data.shares_accepted = miner_telemetry.shares_accepted as u64;
        self.current_data.shares_rejected = miner_telemetry.shares_rejected as u64;
        self.current_data.pool_url = miner_telemetry.pool_url.clone();

        // Extract basic telemetry information
        self.current_data.device_count = 1; // Single device reporting for now
        self.current_data.total_power = miner_telemetry.power_watts.unwrap_or(0.0);
        self.current_data.avg_temperature = miner_telemetry.temperature_c.unwrap_or(0.0);

        // Create basic device telemetry
        self.current_data.device_telemetry = vec![DeviceTelemetry {
            device_id: "primary".to_string(),
            device_name: "Primary Mining Device".to_string(),
            hashrate: miner_telemetry.hashrate,
            power: miner_telemetry.power_watts.unwrap_or(0.0),
            temperature: miner_telemetry.temperature_c.unwrap_or(0.0),
            fan_speed: (miner_telemetry.fan_speed_percent.unwrap_or(0.0) * 50.0) as u32, // Estimate RPM
            utilization: 100, // Assume full utilization when mining
            status: if miner_telemetry.hashrate > 0.0 {
                "mining".to_string()
            } else {
                "idle".to_string()
            },
            memory_used: 0,  // Not available in current telemetry
            memory_total: 0, // Not available in current telemetry
            core_clock: 0,   // Not available in current telemetry
            memory_clock: 0, // Not available in current telemetry
        }];
    }

    /// Update profit information
    pub fn update_profit_info(&mut self, profit_eur_day: Option<f64>) {
        self.current_data.profit_eur_day = profit_eur_day;
    }

    /// Get current telemetry data
    pub fn get_current_data(&self) -> TelemetryData {
        self.current_data.clone()
    }

    /// Get telemetry summary for logging
    pub fn get_summary(&self) -> String {
        format!(
            "Algorithm: {}, Hashrate: {:.2} MH/s, Power: {:.1}W, Temp: {:.1}°C, Devices: {}, Shares: {}/{}, Profit: {}",
            self.current_data.algorithm,
            self.current_data.total_hashrate / 1_000_000.0,
            self.current_data.total_power,
            self.current_data.avg_temperature,
            self.current_data.device_count,
            self.current_data.shares_accepted,
            self.current_data.shares_rejected,
            match self.current_data.profit_eur_day {
                Some(profit) => format!("€{:.2}/day", profit),
                None => "Unknown".to_string(),
            }
        )
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}
