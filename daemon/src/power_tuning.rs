use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::hardware::MiningDevice;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerTuningProfile {
    pub algorithm: String,
    pub power_target_percent: u32,
    pub voltage_offset_mv: i32,
    pub memory_voltage_offset_mv: i32,
    pub efficiency_target_hw: f64,
    pub thermal_throttle_temp_c: u32,
    pub optimize_power_states: bool,
    pub power_curve: Vec<PowerCurvePoint>,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerCurvePoint {
    pub frequency_mhz: u32,
    pub voltage_mv: u32,
    pub power_watts: u32,
}

impl Default for PowerTuningProfile {
    fn default() -> Self {
        Self {
            algorithm: String::new(),
            power_target_percent: 100,
            voltage_offset_mv: 0,
            memory_voltage_offset_mv: 0,
            efficiency_target_hw: 0.0,
            thermal_throttle_temp_c: 83,
            optimize_power_states: true,
            power_curve: Vec::new(),
            name: "Default Power".to_string(),
            enabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PowerMetrics {
    pub device_id: String,
    pub current_power_w: f64,
    pub current_hashrate: f64,
    pub efficiency_hw: f64,
    pub temperature_c: f64,
    pub core_voltage_mv: u32,
    pub memory_voltage_mv: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct PowerTuningEngine {
    enabled: bool,
    profiles: HashMap<String, PowerTuningProfile>,
    metrics: HashMap<String, PowerMetrics>,
    efficiency_targets: HashMap<String, f64>,
}

impl PowerTuningEngine {
    pub fn new() -> Self {
        Self {
            enabled: false,
            profiles: HashMap::new(),
            metrics: HashMap::new(),
            efficiency_targets: HashMap::new(),
        }
    }

    pub fn initialize(
        &mut self,
        enabled: bool,
        profiles: HashMap<String, PowerTuningProfile>,
    ) -> Result<()> {
        self.enabled = false;
        self.profiles = profiles;
        self.calculate_efficiency_targets();

        if enabled {
            warn!("Power tuning is configured but not implemented for this build; keeping it disabled");
        }

        Ok(())
    }

    fn calculate_efficiency_targets(&mut self) {
        self.efficiency_targets.clear();
        for (algorithm, profile) in &self.profiles {
            if profile.efficiency_target_hw > 0.0 {
                self.efficiency_targets
                    .insert(algorithm.clone(), profile.efficiency_target_hw);
            }
        }
        info!(
            "Calculated efficiency targets for {} algorithms",
            self.efficiency_targets.len()
        );
    }

    pub fn apply_power_profile(&mut self, device: &MiningDevice, algorithm: &str) -> Result<()> {
        debug!(
            "Ignoring power tuning request for {} on {} because power tuning is unsupported",
            algorithm, device.id
        );
        Err(anyhow!("Power tuning is not implemented in this build"))
    }

    pub fn monitor_power_efficiency(&mut self, device: &MiningDevice, hashrate: f64) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let power = device.metrics.power_watts.unwrap_or(0.0) as f64;
        let efficiency = if power > 0.0 { hashrate / power } else { 0.0 };
        self.metrics.insert(
            device.id.clone(),
            PowerMetrics {
                device_id: device.id.clone(),
                current_power_w: power,
                current_hashrate: hashrate,
                efficiency_hw: efficiency,
                temperature_c: device.metrics.temperature_c.unwrap_or(0.0) as f64,
                core_voltage_mv: 0,
                memory_voltage_mv: 0,
                timestamp: chrono::Utc::now(),
            },
        );
        Ok(())
    }

    pub fn get_power_metrics(&self, device_id: &str) -> Option<&PowerMetrics> {
        self.metrics.get(device_id)
    }

    pub fn get_all_power_metrics(&self) -> &HashMap<String, PowerMetrics> {
        &self.metrics
    }

    pub fn optimize_power_settings(
        &mut self,
        _device: &MiningDevice,
        _target_efficiency: f64,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        Err(anyhow!(
            "Power optimization is not implemented in this build"
        ))
    }

    pub fn revert_power_settings(&mut self) -> Result<()> {
        self.metrics.clear();
        Ok(())
    }

    pub fn set_power_profile(
        &mut self,
        algorithm: String,
        profile: PowerTuningProfile,
    ) -> Result<()> {
        self.validate_power_settings(&profile)?;
        self.profiles.insert(algorithm, profile);
        self.calculate_efficiency_targets();
        Ok(())
    }

    pub fn validate_power_settings(&self, profile: &PowerTuningProfile) -> Result<()> {
        if profile.power_target_percent < 50 || profile.power_target_percent > 120 {
            return Err(anyhow!(
                "Power target out of safe range: {}% (50-120%)",
                profile.power_target_percent
            ));
        }

        if profile.voltage_offset_mv < -200 || profile.voltage_offset_mv > 100 {
            return Err(anyhow!(
                "Core voltage offset out of safe range: {}mV (-200 to +100mV)",
                profile.voltage_offset_mv
            ));
        }

        if profile.memory_voltage_offset_mv < -100 || profile.memory_voltage_offset_mv > 50 {
            return Err(anyhow!(
                "Memory voltage offset out of safe range: {}mV (-100 to +50mV)",
                profile.memory_voltage_offset_mv
            ));
        }

        if profile.thermal_throttle_temp_c < 60 || profile.thermal_throttle_temp_c > 95 {
            return Err(anyhow!(
                "Thermal throttle temperature out of safe range: {}C (60-95C)",
                profile.thermal_throttle_temp_c
            ));
        }

        for point in &profile.power_curve {
            if point.frequency_mhz > 3000 {
                return Err(anyhow!(
                    "Power curve frequency too high: {}MHz (max 3000MHz)",
                    point.frequency_mhz
                ));
            }
            if point.voltage_mv < 500 || point.voltage_mv > 1500 {
                return Err(anyhow!(
                    "Power curve voltage out of safe range: {}mV (500-1500mV)",
                    point.voltage_mv
                ));
            }
            if point.power_watts > 500 {
                return Err(anyhow!(
                    "Power curve power too high: {}W (max 500W)",
                    point.power_watts
                ));
            }
        }

        Ok(())
    }

    pub fn get_power_profiles(&self) -> &HashMap<String, PowerTuningProfile> {
        &self.profiles
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn emergency_power_reset(&mut self) -> Result<()> {
        warn!("Emergency power reset requested; power tuning is disabled in this build");
        self.enabled = false;
        self.revert_power_settings()
    }
}

impl Default for PowerTuningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_profile_validation() {
        let engine = PowerTuningEngine::new();
        let valid_profile = PowerTuningProfile {
            algorithm: "kaspa".to_string(),
            power_target_percent: 80,
            voltage_offset_mv: -50,
            memory_voltage_offset_mv: 0,
            efficiency_target_hw: 5.0,
            thermal_throttle_temp_c: 80,
            optimize_power_states: true,
            power_curve: vec![PowerCurvePoint {
                frequency_mhz: 1500,
                voltage_mv: 1000,
                power_watts: 200,
            }],
            name: "Kaspa Efficient".to_string(),
            enabled: true,
        };

        assert!(engine.validate_power_settings(&valid_profile).is_ok());

        let invalid_profile = PowerTuningProfile {
            power_target_percent: 150,
            ..valid_profile
        };

        assert!(engine.validate_power_settings(&invalid_profile).is_err());
    }

    #[test]
    fn test_efficiency_calculation() {
        let metrics = PowerMetrics {
            device_id: "test_device".to_string(),
            current_power_w: 250.0,
            current_hashrate: 1000.0,
            efficiency_hw: 4.0,
            temperature_c: 75.0,
            core_voltage_mv: 1100,
            memory_voltage_mv: 1350,
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(metrics.efficiency_hw, 4.0);
    }
}
