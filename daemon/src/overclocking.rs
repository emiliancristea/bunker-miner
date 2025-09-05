use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use winapi::um::winbase::GetCurrentProcess;
#[cfg(windows)]
use winapi::um::processthreadsapi::OpenProcessToken;
#[cfg(windows)]
use winapi::um::securitybaseapi::GetTokenInformation;
#[cfg(windows)]
use winapi::um::winnt::{TokenElevation, HANDLE, TOKEN_ELEVATION, TOKEN_QUERY};

use crate::hardware::{MiningDevice, DeviceType};

/// Overclocking profile configuration for specific mining algorithms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverclockProfile {
    /// Algorithm name this profile applies to
    pub algorithm: String,
    /// Core clock offset in MHz (can be negative for underclocking)
    pub core_clock_offset: i32,
    /// Memory clock offset in MHz (can be negative)
    pub memory_clock_offset: i32,
    /// Power limit in watts (0 means use default)
    pub power_limit_watts: u32,
    /// Temperature limit in Celsius (0 means use default)
    pub temperature_limit_c: u32,
    /// Fan speed percentage (0 means auto)
    pub fan_speed_percent: u32,
    /// Whether this profile is enabled
    pub enabled: bool,
    /// User-defined name for this profile
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

/// Hardware defaults captured before any overclocking is applied
#[derive(Debug, Clone)]
pub struct HardwareDefaults {
    pub device_id: String,
    pub core_clock_mhz: u32,
    pub memory_clock_mhz: u32,
    pub power_limit_watts: u32,
    pub temperature_limit_c: u32,
    pub fan_speed_percent: u32,
}

/// Current overclocking state for a device
#[derive(Debug, Clone)]
pub struct DeviceOverclockState {
    pub device_id: String,
    pub applied_profile: Option<OverclockProfile>,
    pub defaults: HardwareDefaults,
    pub is_overclocked: bool,
    pub last_applied: chrono::DateTime<chrono::Utc>,
}

/// Main overclocking engine managing all hardware overclocking operations
#[derive(Debug)]
pub struct OverclockingEngine {
    /// Whether overclocking is globally enabled (expert mode)
    enabled: bool,
    /// Whether we have the required privileges for overclocking
    has_privileges: bool,
    /// Available overclocking profiles by algorithm
    profiles: HashMap<String, OverclockProfile>,
    /// Current state for each device
    device_states: Arc<Mutex<HashMap<String, DeviceOverclockState>>>,
    /// Flag indicating if we need to revert on shutdown
    needs_revert_on_shutdown: bool,
}

impl OverclockingEngine {
    /// Create new overclocking engine (disabled by default for safety)
    pub fn new() -> Self {
        Self {
            enabled: false, // SAFETY: Always start disabled
            has_privileges: false,
            profiles: HashMap::new(),
            device_states: Arc::new(Mutex::new(HashMap::new())),
            needs_revert_on_shutdown: false,
        }
    }

    /// Initialize the overclocking engine with safety checks
    pub fn initialize(&mut self, expert_mode_enabled: bool, profiles: HashMap<String, OverclockProfile>) -> Result<()> {
        info!("Initializing Overclocking Engine...");
        
        // SECURITY: Check if expert mode is explicitly enabled
        if !expert_mode_enabled {
            warn!("Overclocking disabled - expert mode not enabled in configuration");
            self.enabled = false;
            return Ok(());
        }

        // SECURITY: Check for required privileges
        if !self.check_elevated_privileges()? {
            error!("Overclocking requires administrator/root privileges");
            return Err(anyhow!("Insufficient privileges for hardware overclocking"));
        }

        self.has_privileges = true;
        self.profiles = profiles;
        
        // Log security-critical initialization
        info!("Overclocking Engine initialized with {} profiles", self.profiles.len());
        warn!("⚠️  EXPERT MODE ACTIVE: Hardware overclocking enabled with elevated privileges");
        
        // Only enable after all security checks pass
        self.enabled = expert_mode_enabled && self.has_privileges;
        
        Ok(())
    }

    /// Check if the current process has elevated privileges required for overclocking
    fn check_elevated_privileges(&self) -> Result<bool> {
        #[cfg(windows)]
        {
            unsafe {
                let mut token: HANDLE = ptr::null_mut();
                let process = GetCurrentProcess();
                
                if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
                    return Err(anyhow!("Failed to open process token"));
                }
                
                let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
                let mut return_length: u32 = 0;
                
                let result = GetTokenInformation(
                    token,
                    TokenElevation,
                    &mut elevation as *mut _ as *mut _,
                    std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                    &mut return_length,
                );
                
                if result == 0 {
                    return Err(anyhow!("Failed to get token elevation information"));
                }
                
                Ok(elevation.TokenIsElevated != 0)
            }
        }
        
        #[cfg(unix)]
        {
            // Check if running as root
            Ok(unsafe { libc::geteuid() } == 0)
        }
    }

    /// Apply overclocking profile for specific algorithm and device
    pub fn apply_profile(&mut self, device: &MiningDevice, algorithm: &str) -> Result<()> {
        // SECURITY: Verify overclocking is enabled and we have privileges
        if !self.enabled {
            debug!("Overclocking not enabled - skipping profile application");
            return Ok(());
        }

        if !self.has_privileges {
            error!("Attempted overclocking without required privileges");
            return Err(anyhow!("Insufficient privileges for overclocking"));
        }

        // Find matching profile for algorithm
        let profile = match self.profiles.get(algorithm) {
            Some(profile) if profile.enabled => profile.clone(),
            Some(_) => {
                debug!("Profile for {} exists but is disabled", algorithm);
                return Ok(());
            }
            None => {
                debug!("No overclocking profile found for algorithm: {}", algorithm);
                return Ok(());
            }
        };

        info!("Applying overclocking profile '{}' for algorithm '{}' on device '{}'", 
              profile.name, algorithm, device.device_id);

        // Capture hardware defaults before overclocking (if not already captured)
        let defaults = self.capture_hardware_defaults(device)?;

        // Apply the overclocking settings
        self.apply_hardware_settings(device, &profile)?;

        // Update device state
        let mut states = self.device_states.lock().unwrap();
        states.insert(device.device_id.clone(), DeviceOverclockState {
            device_id: device.device_id.clone(),
            applied_profile: Some(profile),
            defaults,
            is_overclocked: true,
            last_applied: chrono::Utc::now(),
        });

        self.needs_revert_on_shutdown = true;
        
        info!("Successfully applied overclocking profile for device '{}'", device.device_id);
        Ok(())
    }

    /// Capture current hardware settings as defaults
    fn capture_hardware_defaults(&self, device: &MiningDevice) -> Result<HardwareDefaults> {
        debug!("Capturing hardware defaults for device: {}", device.device_id);

        // Check if defaults already captured
        {
            let states = self.device_states.lock().unwrap();
            if let Some(state) = states.get(&device.device_id) {
                debug!("Using previously captured defaults for device: {}", device.device_id);
                return Ok(state.defaults.clone());
            }
        }

        match device.device_type {
            DeviceType::Gpu if device.vendor.to_lowercase().contains("nvidia") => {
                self.capture_nvidia_defaults(device)
            }
            DeviceType::Gpu if device.vendor.to_lowercase().contains("amd") => {
                self.capture_amd_defaults(device)
            }
            _ => {
                Err(anyhow!("Unsupported device type for overclocking: {:?}", device.device_type))
            }
        }
    }

    /// Capture NVIDIA GPU defaults using NVML
    fn capture_nvidia_defaults(&self, device: &MiningDevice) -> Result<HardwareDefaults> {
        // Use nvml-wrapper to get current settings
        // This is a placeholder - actual NVML integration would be more complex
        Ok(HardwareDefaults {
            device_id: device.device_id.clone(),
            core_clock_mhz: 1500, // Would be read from NVML
            memory_clock_mhz: 6000, // Would be read from NVML
            power_limit_watts: 250, // Would be read from NVML
            temperature_limit_c: 83, // Would be read from NVML
            fan_speed_percent: 0, // Auto
        })
    }

    /// Capture AMD GPU defaults using ROCm-SMI
    fn capture_amd_defaults(&self, device: &MiningDevice) -> Result<HardwareDefaults> {
        // Use rocm-smi CLI or ADL to get current settings
        // This is a placeholder - actual AMD integration would use system calls
        Ok(HardwareDefaults {
            device_id: device.device_id.clone(),
            core_clock_mhz: 1200, // Would be read from ROCm-SMI
            memory_clock_mhz: 1750, // Would be read from ROCm-SMI
            power_limit_watts: 180, // Would be read from ROCm-SMI
            temperature_limit_c: 90, // Would be read from ROCm-SMI
            fan_speed_percent: 0, // Auto
        })
    }

    /// Apply hardware settings based on overclocking profile
    fn apply_hardware_settings(&self, device: &MiningDevice, profile: &OverclockProfile) -> Result<()> {
        debug!("Applying hardware settings for device: {} with profile: {}", 
               device.device_id, profile.name);

        match device.device_type {
            DeviceType::Gpu if device.vendor.to_lowercase().contains("nvidia") => {
                self.apply_nvidia_settings(device, profile)
            }
            DeviceType::Gpu if device.vendor.to_lowercase().contains("amd") => {
                self.apply_amd_settings(device, profile)
            }
            _ => {
                Err(anyhow!("Unsupported device type for overclocking: {:?}", device.device_type))
            }
        }
    }

    /// Apply overclocking settings to NVIDIA GPU
    fn apply_nvidia_settings(&self, device: &MiningDevice, profile: &OverclockProfile) -> Result<()> {
        // SAFETY: Log all changes for audit trail
        info!("Applying NVIDIA overclocking - Device: {}, Core Offset: {}MHz, Memory Offset: {}MHz, Power Limit: {}W",
              device.device_id, profile.core_clock_offset, profile.memory_clock_offset, profile.power_limit_watts);

        // TODO: Implement actual NVML calls
        // This would use nvml-wrapper to:
        // 1. Set core clock offset
        // 2. Set memory clock offset  
        // 3. Set power limit
        // 4. Set temperature limit
        // 5. Set fan speed if specified

        // Placeholder implementation
        warn!("NVIDIA overclocking - PLACEHOLDER IMPLEMENTATION");
        
        // Simulate applying settings
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        Ok(())
    }

    /// Apply overclocking settings to AMD GPU
    fn apply_amd_settings(&self, device: &MiningDevice, profile: &OverclockProfile) -> Result<()> {
        // SAFETY: Log all changes for audit trail
        info!("Applying AMD overclocking - Device: {}, Core Offset: {}MHz, Memory Offset: {}MHz, Power Limit: {}W",
              device.device_id, profile.core_clock_offset, profile.memory_clock_offset, profile.power_limit_watts);

        // TODO: Implement actual ROCm-SMI calls
        // This would use std::process::Command to call rocm-smi:
        // 1. rocm-smi --setcoreclk <device> <clock>
        // 2. rocm-smi --setmemclk <device> <clock>
        // 3. rocm-smi --setpower <device> <watts>
        // 4. rocm-smi --setfan <device> <percent>

        // Placeholder implementation
        warn!("AMD overclocking - PLACEHOLDER IMPLEMENTATION");
        
        // Simulate applying settings
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        Ok(())
    }

    /// Revert all devices to their default hardware settings
    pub fn revert_all_to_defaults(&mut self) -> Result<()> {
        if !self.needs_revert_on_shutdown {
            debug!("No overclocking applied - skipping revert");
            return Ok(());
        }

        info!("Reverting all devices to default hardware settings...");

        let states = self.device_states.lock().unwrap().clone();
        for (device_id, state) in states.iter() {
            if state.is_overclocked {
                info!("Reverting device '{}' to defaults", device_id);
                if let Err(e) = self.revert_device_to_defaults(&state.defaults) {
                    error!("Failed to revert device '{}': {}", device_id, e);
                    // Continue with other devices even if one fails
                }
            }
        }

        // Clear all overclocking state
        self.device_states.lock().unwrap().clear();
        self.needs_revert_on_shutdown = false;

        info!("All devices reverted to default settings");
        Ok(())
    }

    /// Revert specific device to default settings
    fn revert_device_to_defaults(&self, defaults: &HardwareDefaults) -> Result<()> {
        debug!("Reverting device '{}' to defaults", defaults.device_id);

        // Create a "default" profile from the captured defaults
        let default_profile = OverclockProfile {
            algorithm: "default".to_string(),
            core_clock_offset: 0, // Reset to base clock
            memory_clock_offset: 0, // Reset to base clock
            power_limit_watts: defaults.power_limit_watts,
            temperature_limit_c: defaults.temperature_limit_c,
            fan_speed_percent: 0, // Auto
            enabled: true,
            name: "Default".to_string(),
        };

        // Apply default settings (this will reset clocks to base values)
        // TODO: Implement actual hardware reset calls
        warn!("Device revert - PLACEHOLDER IMPLEMENTATION for device: {}", defaults.device_id);
        
        Ok(())
    }

    /// Get list of available overclocking profiles
    pub fn get_profiles(&self) -> &HashMap<String, OverclockProfile> {
        &self.profiles
    }

    /// Add or update an overclocking profile
    pub fn set_profile(&mut self, algorithm: String, profile: OverclockProfile) -> Result<()> {
        if !self.enabled {
            return Err(anyhow!("Overclocking not enabled"));
        }

        // SECURITY: Validate profile settings are within safe ranges
        self.validate_profile(&profile)?;

        info!("Adding/updating overclocking profile for algorithm: {}", algorithm);
        self.profiles.insert(algorithm, profile);
        
        Ok(())
    }

    /// Validate overclocking profile settings are within safe ranges
    fn validate_profile(&self, profile: &OverclockProfile) -> Result<()> {
        // SAFETY: Implement reasonable limits to prevent hardware damage
        
        if profile.core_clock_offset.abs() > 500 {
            return Err(anyhow!("Core clock offset too extreme: {}MHz (max ±500MHz)", profile.core_clock_offset));
        }

        if profile.memory_clock_offset.abs() > 1000 {
            return Err(anyhow!("Memory clock offset too extreme: {}MHz (max ±1000MHz)", profile.memory_clock_offset));
        }

        if profile.power_limit_watts > 0 && (profile.power_limit_watts < 50 || profile.power_limit_watts > 500) {
            return Err(anyhow!("Power limit out of safe range: {}W (50-500W)", profile.power_limit_watts));
        }

        if profile.temperature_limit_c > 0 && (profile.temperature_limit_c < 60 || profile.temperature_limit_c > 95) {
            return Err(anyhow!("Temperature limit out of safe range: {}°C (60-95°C)", profile.temperature_limit_c));
        }

        if profile.fan_speed_percent > 100 {
            return Err(anyhow!("Fan speed percentage invalid: {}% (max 100%)", profile.fan_speed_percent));
        }

        Ok(())
    }

    /// Get current device overclocking states
    pub fn get_device_states(&self) -> HashMap<String, DeviceOverclockState> {
        self.device_states.lock().unwrap().clone()
    }

    /// Check if overclocking is enabled and available
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.has_privileges
    }

    /// Emergency stop - immediately disable all overclocking
    pub fn emergency_stop(&mut self) -> Result<()> {
        warn!("EMERGENCY STOP: Disabling all overclocking immediately");
        
        // Disable the engine
        self.enabled = false;
        
        // Revert all devices to defaults
        self.revert_all_to_defaults()?;
        
        error!("Emergency stop completed - all overclocking disabled");
        Ok(())
    }
}

impl Drop for OverclockingEngine {
    /// Ensure hardware is reverted to defaults when the engine is dropped
    fn drop(&mut self) {
        if self.needs_revert_on_shutdown {
            warn!("OverclockingEngine dropped - reverting hardware to defaults");
            if let Err(e) = self.revert_all_to_defaults() {
                error!("Failed to revert hardware on drop: {}", e);
            }
        }
    }
}

/// RAII guard to ensure hardware settings are reverted when mining stops
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
        debug!("OverclockGuard dropped - reverting overclocking");
        if let Ok(mut engine) = self.engine.lock() {
            if let Err(e) = engine.revert_all_to_defaults() {
                error!("Failed to revert overclocking in guard: {}", e);
            }
        }
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
            core_clock_offset: 1000, // Too extreme
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