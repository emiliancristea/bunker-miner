/*!
 * BUNKER MINER - Secure Storage PoC
 * 
 * This PoC validates a secure and user-friendly method for encrypting local
 * configuration files using the `age` encryption library, ensuring sensitive
 * data like wallet addresses and API keys are protected at rest.
 * 
 * Success Criteria:
 * - Successfully encrypt and decrypt configuration files
 * - Handle incorrect passwords gracefully with clear error messages  
 * - Provide secure key derivation from user passwords
 * - Demonstrate file integrity verification
 * - Show performance characteristics for typical config file sizes
 */

use age::{
    secrecy::{ExposeSecret, Secret},
    Decryptor, Encryptor,
};
use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;
use tracing::{error, info, warn, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    pub version: String,
    pub miner_settings: MinerSettings,
    pub pool_settings: PoolSettings,
    pub security_settings: SecuritySettings,
    pub ui_settings: UiSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerSettings {
    pub auto_start: bool,
    pub hardware_monitoring: bool,
    pub cpu_threads: Option<u32>,
    pub gpu_enabled: bool,
    pub algorithm: String,
    pub intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSettings {
    pub primary_pool: PoolConfig,
    pub backup_pools: Vec<PoolConfig>,
    pub wallet_address: String, // This needs to be encrypted!
    pub worker_name: String,
    pub auto_failover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub name: String,
    pub url: String,
    pub port: u16,
    pub username: String,
    pub password: String, // This needs to be encrypted!
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub api_key: Option<String>, // This needs to be encrypted!
    pub enable_remote_access: bool,
    pub allowed_ips: Vec<String>,
    pub require_auth: bool,
    pub session_timeout_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub theme: String,
    pub language: String,
    pub auto_refresh_seconds: u32,
    pub show_advanced_settings: bool,
}

pub struct SecureConfigManager {
    config_path: String,
    encrypted_path: String,
}

impl SecureConfigManager {
    pub fn new(config_path: String) -> Self {
        let encrypted_path = format!("{}.encrypted", config_path);
        
        Self {
            config_path,
            encrypted_path,
        }
    }

    pub fn encrypt_config_file(&self, password: &str) -> Result<()> {
        info!("Encrypting configuration file: {}", self.config_path);
        let start_time = Instant::now();

        // Read the plaintext config file
        let plaintext = fs::read(&self.config_path)
            .context("Failed to read config file")?;

        info!("Config file size: {} bytes", plaintext.len());

        // Create encryptor with password
        let encryptor = Encryptor::with_user_passphrase(Secret::new(password.to_owned()));

        // Encrypt the data
        let mut encrypted = Vec::new();
        let mut writer = encryptor.wrap_output(&mut encrypted)
            .context("Failed to create encryption writer")?;
        
        writer.write_all(&plaintext)
            .context("Failed to write plaintext to encryptor")?;
        
        writer.finish()
            .context("Failed to finalize encryption")?;

        // Write encrypted data to file
        fs::write(&self.encrypted_path, &encrypted)
            .context("Failed to write encrypted file")?;

        let encryption_time = start_time.elapsed();
        info!("✅ Config encrypted successfully in {:?}", encryption_time);
        info!("Encrypted file size: {} bytes", encrypted.len());
        info!("Size increase: {:.1}%", 
              ((encrypted.len() as f64 / plaintext.len() as f64) - 1.0) * 100.0);

        Ok(())
    }

    pub fn decrypt_config_file(&self, password: &str) -> Result<MinerConfig> {
        info!("Decrypting configuration file: {}", self.encrypted_path);
        let start_time = Instant::now();

        // Read the encrypted file
        let encrypted = fs::read(&self.encrypted_path)
            .context("Failed to read encrypted config file")?;

        info!("Encrypted file size: {} bytes", encrypted.len());

        // Create decryptor
        let decryptor = match Decryptor::new(&encrypted[..])? {
            Decryptor::Passphrase(d) => d,
            _ => return Err(anyhow::anyhow!("Unexpected decryptor type")),
        };

        // Decrypt the data
        let mut decrypted = Vec::new();
        let mut reader = decryptor.decrypt(&Secret::new(password.to_owned()), None)
            .context("Failed to decrypt - incorrect password?")?;
        
        reader.read_to_end(&mut decrypted)
            .context("Failed to read decrypted data")?;

        let decryption_time = start_time.elapsed();
        info!("✅ Config decrypted successfully in {:?}", decryption_time);
        info!("Decrypted data size: {} bytes", decrypted.len());

        // Parse the decrypted TOML
        let config_str = String::from_utf8(decrypted)
            .context("Decrypted data is not valid UTF-8")?;

        let config: MinerConfig = toml::from_str(&config_str)
            .context("Failed to parse decrypted config as TOML")?;

        info!("✅ Config parsed successfully");
        Ok(config)
    }

    pub fn verify_password(&self, password: &str) -> Result<bool> {
        debug!("Verifying password for encrypted config");
        
        match self.decrypt_config_file(password) {
            Ok(_) => Ok(true),
            Err(e) => {
                let error_str = e.to_string().to_lowercase();
                if error_str.contains("decrypt") || error_str.contains("password") {
                    debug!("Password verification failed - incorrect password");
                    Ok(false)
                } else {
                    // Other error (file not found, corruption, etc.)
                    Err(e)
                }
            }
        }
    }

    pub fn create_sample_config(&self) -> Result<MinerConfig> {
        let config = MinerConfig {
            version: "1.0.0".to_string(),
            miner_settings: MinerSettings {
                auto_start: true,
                hardware_monitoring: true,
                cpu_threads: Some(4),
                gpu_enabled: true,
                algorithm: "randomx".to_string(),
                intensity: 0.8,
            },
            pool_settings: PoolSettings {
                primary_pool: PoolConfig {
                    name: "Primary Pool".to_string(),
                    url: "pool.supportxmr.com".to_string(),
                    port: 443,
                    username: "wallet_address_placeholder".to_string(),
                    password: "worker1".to_string(),
                },
                backup_pools: vec![
                    PoolConfig {
                        name: "Backup Pool".to_string(),
                        url: "xmr.pool.minergate.com".to_string(),
                        port: 45700,
                        username: "backup_wallet_placeholder".to_string(),
                        password: "worker1_backup".to_string(),
                    }
                ],
                wallet_address: "48abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string(),
                worker_name: "bunker-miner-001".to_string(),
                auto_failover: true,
            },
            security_settings: SecuritySettings {
                api_key: Some("super-secret-api-key-12345".to_string()),
                enable_remote_access: false,
                allowed_ips: vec!["127.0.0.1".to_string()],
                require_auth: true,
                session_timeout_minutes: 60,
            },
            ui_settings: UiSettings {
                theme: "dark".to_string(),
                language: "en".to_string(),
                auto_refresh_seconds: 5,
                show_advanced_settings: false,
            },
        };

        Ok(config)
    }

    pub fn save_config_as_toml(&self, config: &MinerConfig) -> Result<()> {
        let toml_string = toml::to_string_pretty(config)
            .context("Failed to serialize config to TOML")?;

        fs::write(&self.config_path, toml_string)
            .context("Failed to write config file")?;

        info!("Configuration saved to: {}", self.config_path);
        Ok(())
    }

    pub fn config_exists(&self) -> bool {
        Path::new(&self.config_path).exists()
    }

    pub fn encrypted_config_exists(&self) -> bool {
        Path::new(&self.encrypted_path).exists()
    }

    pub fn remove_plaintext_config(&self) -> Result<()> {
        if self.config_exists() {
            fs::remove_file(&self.config_path)
                .context("Failed to remove plaintext config file")?;
            info!("Removed plaintext config file for security");
        }
        Ok(())
    }
}

async fn run_encryption_test(config_path: &str, password: &str) -> Result<()> {
    info!("=== Starting Secure Storage PoC Test ===");
    
    let manager = SecureConfigManager::new(config_path.to_string());

    // Step 1: Create and save sample configuration
    info!("Step 1: Creating sample configuration...");
    let sample_config = manager.create_sample_config()?;
    manager.save_config_as_toml(&sample_config)?;

    // Step 2: Encrypt the configuration
    info!("Step 2: Encrypting configuration...");
    manager.encrypt_config_file(password)?;

    // Step 3: Verify we can decrypt with correct password
    info!("Step 3: Testing decryption with correct password...");
    let decrypted_config = manager.decrypt_config_file(password)?;
    
    // Verify data integrity
    assert_eq!(sample_config.version, decrypted_config.version);
    assert_eq!(sample_config.pool_settings.wallet_address, decrypted_config.pool_settings.wallet_address);
    assert_eq!(sample_config.security_settings.api_key, decrypted_config.security_settings.api_key);
    
    info!("✅ Data integrity verified - all sensitive fields match");

    // Step 4: Test incorrect password handling
    info!("Step 4: Testing incorrect password handling...");
    let wrong_password = "wrong_password";
    match manager.verify_password(wrong_password) {
        Ok(false) => info!("✅ Incorrect password correctly rejected"),
        Ok(true) => error!("❌ Incorrect password was accepted - this is a bug!"),
        Err(e) => error!("❌ Unexpected error with incorrect password: {}", e),
    }

    // Step 5: Performance benchmark
    info!("Step 5: Running performance benchmark...");
    let iterations = 10;
    let mut total_encrypt_time = std::time::Duration::new(0, 0);
    let mut total_decrypt_time = std::time::Duration::new(0, 0);

    for i in 1..=iterations {
        debug!("Benchmark iteration {}/{}", i, iterations);
        
        // Encrypt
        let start = Instant::now();
        manager.encrypt_config_file(password)?;
        total_encrypt_time += start.elapsed();
        
        // Decrypt
        let start = Instant::now();
        let _config = manager.decrypt_config_file(password)?;
        total_decrypt_time += start.elapsed();
    }

    let avg_encrypt_time = total_encrypt_time / iterations;
    let avg_decrypt_time = total_decrypt_time / iterations;

    info!("📊 Performance Results:");
    info!("  Average encryption time: {:?}", avg_encrypt_time);
    info!("  Average decryption time: {:?}", avg_decrypt_time);
    info!("  Total benchmark time: {:?}", total_encrypt_time + total_decrypt_time);

    // Step 6: Clean up plaintext for security demo
    info!("Step 6: Removing plaintext config file for security...");
    manager.remove_plaintext_config()?;

    // Step 7: Verify we can still access config from encrypted file only
    info!("Step 7: Verifying access from encrypted file only...");
    let final_config = manager.decrypt_config_file(password)?;
    info!("✅ Successfully accessed config from encrypted file");
    info!("  Wallet address (first 12 chars): {}...", 
          &final_config.pool_settings.wallet_address[..12]);

    info!("✅ Secure Storage PoC completed successfully!");
    Ok(())
}

async fn run_interactive_test() -> Result<()> {
    use std::io::{self, Write};

    println!("=== Interactive Secure Storage Test ===");
    println!("This test will create a sample config and let you encrypt/decrypt it.");

    // Get password from user
    print!("Enter encryption password: ");
    io::stdout().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim();

    if password.is_empty() {
        return Err(anyhow::anyhow!("Password cannot be empty"));
    }

    let config_path = "test_interactive_config.toml";
    let manager = SecureConfigManager::new(config_path.to_string());

    // Create sample config
    let config = manager.create_sample_config()?;
    manager.save_config_as_toml(&config)?;

    println!("✅ Sample config created at: {}", config_path);
    println!("📄 Config contains sensitive data like:");
    println!("  - Wallet address: {}", config.pool_settings.wallet_address);
    println!("  - API key: {:?}", config.security_settings.api_key);

    // Encrypt
    print!("Press Enter to encrypt the file...");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    manager.encrypt_config_file(password)?;
    println!("✅ File encrypted successfully!");

    // Test decryption
    print!("Press Enter to test decryption...");
    io::stdout().flush()?;
    input.clear();
    io::stdin().read_line(&mut input)?;

    match manager.decrypt_config_file(password) {
        Ok(decrypted) => {
            println!("✅ Decryption successful!");
            println!("📄 Recovered sensitive data:");
            println!("  - Wallet address: {}", decrypted.pool_settings.wallet_address);
            println!("  - API key: {:?}", decrypted.security_settings.api_key);
        }
        Err(e) => {
            println!("❌ Decryption failed: {}", e);
        }
    }

    // Test wrong password
    print!("Now enter a wrong password to test error handling: ");
    io::stdout().flush()?;
    input.clear();
    io::stdin().read_line(&mut input)?;
    let wrong_password = input.trim();

    match manager.verify_password(wrong_password) {
        Ok(true) => println!("❌ Wrong password was accepted - this shouldn't happen!"),
        Ok(false) => println!("✅ Wrong password correctly rejected"),
        Err(e) => println!("❌ Error during password verification: {}", e),
    }

    // Cleanup
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_file(format!("{}.encrypted", config_path));

    println!("✅ Interactive test completed!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("BUNKER MINER - Secure Storage PoC");

    let matches = Command::new("secure-storage")
        .about("BUNKER MINER Secure Storage Proof of Concept")
        .arg(
            Arg::new("config-path")
                .long("config")
                .help("Path to configuration file")
                .default_value("test_config.toml")
                .value_name("PATH")
        )
        .arg(
            Arg::new("password")
                .long("password")
                .help("Encryption password (for automated testing)")
                .value_name("PASSWORD")
        )
        .arg(
            Arg::new("interactive")
                .long("interactive")
                .help("Run interactive test mode")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    if matches.get_flag("interactive") {
        run_interactive_test().await?;
    } else {
        let config_path = matches.get_one::<String>("config-path").unwrap();
        let password = matches.get_one::<String>("password")
            .map(|s| s.as_str())
            .unwrap_or("test_password_123");

        run_encryption_test(config_path, password).await?;

        // Cleanup test files
        let _ = fs::remove_file(config_path);
        let _ = fs::remove_file(format!("{}.encrypted", config_path));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_config_encryption_decryption() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_str().unwrap().to_string();
        let password = "test_password_123";

        let manager = SecureConfigManager::new(config_path);
        let original_config = manager.create_sample_config().unwrap();
        
        // Save and encrypt
        manager.save_config_as_toml(&original_config).unwrap();
        manager.encrypt_config_file(password).unwrap();

        // Decrypt and verify
        let decrypted_config = manager.decrypt_config_file(password).unwrap();
        
        assert_eq!(original_config.version, decrypted_config.version);
        assert_eq!(original_config.pool_settings.wallet_address, decrypted_config.pool_settings.wallet_address);
        assert_eq!(original_config.security_settings.api_key, decrypted_config.security_settings.api_key);
    }

    #[tokio::test]
    async fn test_wrong_password_handling() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_str().unwrap().to_string();
        let correct_password = "correct_password";
        let wrong_password = "wrong_password";

        let manager = SecureConfigManager::new(config_path);
        let config = manager.create_sample_config().unwrap();
        
        manager.save_config_as_toml(&config).unwrap();
        manager.encrypt_config_file(correct_password).unwrap();

        // Correct password should work
        assert!(manager.verify_password(correct_password).unwrap());
        
        // Wrong password should be rejected
        assert!(!manager.verify_password(wrong_password).unwrap());
    }

    #[test]
    fn test_sample_config_creation() {
        let manager = SecureConfigManager::new("test.toml".to_string());
        let config = manager.create_sample_config().unwrap();
        
        assert_eq!(config.version, "1.0.0");
        assert!(!config.pool_settings.wallet_address.is_empty());
        assert!(config.security_settings.api_key.is_some());
        assert_eq!(config.miner_settings.algorithm, "randomx");
    }
}