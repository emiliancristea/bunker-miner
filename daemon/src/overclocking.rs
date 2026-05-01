use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};

use crate::hardware::MiningDevice;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverclockProfile {
    pub algorithm: String,
    pub core_clock_offset: i32,
    pub memory_clock_offset: i32,
    pub power_limit_watts: u32,
    pub temperature_limit_c: u32,
    pub fan_speed_percent: u32,
    pub enabled: bool,
    pub name: String,
}

impl Default for OverclockProfile {
    fn default() -> Self {
        Self {
            algorithm: String::new(),
            core_clock_offset: 0,
            memory_clock_offset: 0,
            power_limit_watts: 0,
            temperature_limit_c: 0,
            fan_speed_percent: 0,
            enabled: false,
            name: "Default".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HardwareDefaults {
    pub device_id: String,
    pub core_clock_mhz: u32,
    pub memory_clock_mhz: u32,
    pub power_limit_watts: u32,
    pub temperature_limit_c: u32,
    pub fan_speed_percent: u32,
}

#[derive(Debug, Clone)]
pub struct DeviceOverclockState {
    pub device_id: String,
    pub applied_profile: Option<OverclockProfile>,
    pub defaults: HardwareDefaults,
    pub is_overclocked: bool,
    pub last_applied: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct OverclockingEngine {
    enabled: bool,
    profiles: HashMap<String, OverclockProfile>,
    device_states: Arc<Mutex<HashMap<String, DeviceOverclockState>>>,
}

impl OverclockingEngine {
    pub fn new() -> Self {
        Self {
            enabled: false,
            profiles: HashMap::new(),
            device_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn initialize(
        &mut self,
        expert_mode_enabled: bool,
        profiles: HashMap<String, OverclockProfile>,
    ) -> Result<()> {
        self.enabled = false;
        self.profiles = profiles;

        if expert_mode_enabled {
            warn!("Overclocking is configured but not implemented for this build; keeping it disabled");
        }

        Ok(())
    }

    pub fn apply_profile(&mut self, device: &MiningDevice, algorithm: &str) -> Result<()> {
        debug!(
            "Ignoring overclock request for {} on {} because overclocking is unsupported",
            algorithm, device.id
        );
        Err(anyhow!("Overclocking is not implemented in this build"))
    }

    pub fn revert_all_to_defaults(&mut self) -> Result<()> {
        self.device_states.lock().unwrap().clear();
        Ok(())
    }

    pub fn get_profiles(&self) -> &HashMap<String, OverclockProfile> {
        &self.profiles
    }

    pub fn set_profile(&mut self, algorithm: String, profile: OverclockProfile) -> Result<()> {
        self.validate_profile(&profile)?;
        self.profiles.insert(algorithm, profile);
        Ok(())
    }

    pub fn validate_profile(&self, profile: &OverclockProfile) -> Result<()> {
        if profile.core_clock_offset.abs() > 500 {
            return Err(anyhow!(
                "Core clock offset too extreme: {}MHz (max +/-500MHz)",
                profile.core_clock_offset
            ));
        }

        if profile.memory_clock_offset.abs() > 1000 {
            return Err(anyhow!(
                "Memory clock offset too extreme: {}MHz (max +/-1000MHz)",
                profile.memory_clock_offset
            ));
        }

        if profile.power_limit_watts > 0
            && (profile.power_limit_watts < 50 || profile.power_limit_watts > 500)
        {
            return Err(anyhow!(
                "Power limit out of safe range: {}W (50-500W)",
                profile.power_limit_watts
            ));
        }

        if profile.temperature_limit_c > 0
            && (profile.temperature_limit_c < 60 || profile.temperature_limit_c > 95)
        {
            return Err(anyhow!(
                "Temperature limit out of safe range: {}C (60-95C)",
                profile.temperature_limit_c
            ));
        }

        if profile.fan_speed_percent > 100 {
            return Err(anyhow!(
                "Fan speed percentage invalid: {}% (max 100%)",
                profile.fan_speed_percent
            ));
        }

        Ok(())
    }

    pub fn get_device_states(&self) -> HashMap<String, DeviceOverclockState> {
        self.device_states.lock().unwrap().clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn emergency_stop(&mut self) -> Result<()> {
        warn!("Emergency overclock stop requested; overclocking is disabled in this build");
        self.enabled = false;
        self.revert_all_to_defaults()
    }
}

pub struct OverclockGuard {
    engine: Arc<Mutex<OverclockingEngine>>,
}

impl OverclockGuard {
    pub fn new(engine: Arc<Mutex<OverclockingEngine>>) -> Self {
        Self { engine }
    }
}

impl Drop for OverclockGuard {
    fn drop(&mut self) {
        if let Ok(mut engine) = self.engine.lock() {
            let _ = engine.revert_all_to_defaults();
        }
    }
}

impl Default for OverclockingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_validation() {
        let engine = OverclockingEngine::new();
        let valid_profile = OverclockProfile {
            algorithm: "kaspa".to_string(),
            core_clock_offset: 100,
            memory_clock_offset: 500,
            power_limit_watts: 250,
            temperature_limit_c: 80,
            fan_speed_percent: 75,
            enabled: true,
            name: "Kaspa Optimized".to_string(),
        };

        assert!(engine.validate_profile(&valid_profile).is_ok());

        let invalid_profile = OverclockProfile {
            core_clock_offset: 1000,
            ..valid_profile
        };

        assert!(engine.validate_profile(&invalid_profile).is_err());
    }

    #[test]
    fn test_engine_defaults_to_disabled() {
        let engine = OverclockingEngine::new();
        assert!(!engine.is_enabled());
    }
}
