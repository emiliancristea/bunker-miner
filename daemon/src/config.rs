use age::{Decryptor, Encryptor};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use dirs::config_dir;
use rpassword;
use secrecy::{ExposeSecret, Secret};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mining: MiningConfig,
    pub wallets: HashMap<String, WalletConfig>,
    pub pools: HashMap<String, PoolConfig>,
    pub security: SecurityConfig,
    pub grpc: GrpcConfig,
    pub profit_switching: ProfitSwitchingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    pub active_coin: String,
    pub active_wallet: String,
    pub active_pool: String,
    pub auto_switch: bool,
    pub profit_switch_interval_minutes: u32,
    pub backup_pools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub coin: String,
    pub address: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub coin: String,
    pub url: String,
    pub port: u16,
    pub username: Option<String>,
    pub worker_name: Option<String>,
    pub ssl: bool,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub auto_restart: bool,
    pub max_restart_attempts: u32,
    pub restart_delay_seconds: u64,
    pub max_restart_delay_seconds: u64,
    pub telemetry_collection: bool,
    pub remote_access_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitSwitchingConfig {
    pub enable: bool,
    pub electricity_eur_per_kwh: f64,
    pub profit_delta_threshold: f64,
    pub min_dwell_time_minutes: u64,
    pub update_interval_minutes: Option<u64>,
    pub pool_fee_percent: Option<f64>,
    pub enabled_algorithms: Vec<String>,
    pub disabled_algorithms: Vec<String>,
    #[cfg(feature = "proxy")]
    pub proxy_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut wallets = HashMap::new();
        wallets.insert("ethereum_main".to_string(), WalletConfig {
            coin: "ethereum".to_string(),
            address: "0x0000000000000000000000000000000000000000".to_string(),
            label: Some("Main Ethereum Wallet".to_string()),
        });
        
        wallets.insert("monero_main".to_string(), WalletConfig {
            coin: "monero".to_string(),
            address: "4444444444444444444444444444444444444444444444444444444444444444".to_string(),
            label: Some("Main Monero Wallet".to_string()),
        });

        let mut pools = HashMap::new();
        
        // BUNKER POOL - Our proprietary mining pool (highest priority)
        pools.insert("bunker_pool_btc".to_string(), PoolConfig {
            coin: "bitcoin".to_string(),
            url: "stratum+tcp://pool.bunkerminer.com".to_string(),
            port: 3333,
            username: None,
            worker_name: Some("bunker-miner".to_string()),
            ssl: true,
            priority: 10, // Highest priority
        });
        
        pools.insert("bunker_pool_eth".to_string(), PoolConfig {
            coin: "ethereum".to_string(),
            url: "stratum+tcp://pool.bunkerminer.com".to_string(),
            port: 3334,
            username: None,
            worker_name: Some("bunker-miner".to_string()),
            ssl: true,
            priority: 10, // Highest priority
        });
        
        pools.insert("bunker_pool_xmr".to_string(), PoolConfig {
            coin: "monero".to_string(),
            url: "stratum+tcp://pool.bunkerminer.com".to_string(),
            port: 3335,
            username: None,
            worker_name: Some("bunker-miner".to_string()),
            ssl: true,
            priority: 10, // Highest priority
        });
        
        // External pools as backups (lower priority)
        pools.insert("ethermine".to_string(), PoolConfig {
            coin: "ethereum".to_string(),
            url: "stratum1+tcp://eth-us-east1.nanopool.org".to_string(),
            port: 9999,
            username: None,
            worker_name: Some("bunker-miner".to_string()),
            ssl: false,
            priority: 1, // Lower priority backup
        });
        
        pools.insert("minexmr".to_string(), PoolConfig {
            coin: "monero".to_string(),
            url: "pool.minexmr.com".to_string(),
            port: 4444,
            username: None,
            worker_name: Some("bunker-miner".to_string()),
            ssl: false,
            priority: 1, // Lower priority backup
        });

        Config {
            mining: MiningConfig {
                active_coin: "ethereum".to_string(),
                active_wallet: "ethereum_main".to_string(),
                active_pool: "bunker_pool_eth".to_string(), // Default to our pool
                auto_switch: true, // Enable profit switching by default
                profit_switch_interval_minutes: 10,
                backup_pools: vec!["bunker_pool_eth".to_string(), "ethermine".to_string()],
            },
            wallets,
            pools,
            security: SecurityConfig {
                auto_restart: true,
                max_restart_attempts: 10,
                restart_delay_seconds: 5,
                max_restart_delay_seconds: 300,
                telemetry_collection: true,
                remote_access_enabled: false,
            },
            grpc: GrpcConfig {
                enabled: true,
                bind_address: "127.0.0.1".to_string(),
                port: 50051,
                tls_enabled: false,
                tls_cert_path: None,
                tls_key_path: None,
                max_connections: 100,
                connection_timeout_seconds: 30,
                request_timeout_seconds: 60,
            },
            profit_switching: ProfitSwitchingConfig {
                enable: true, // Enable by default to leverage our pool
                electricity_eur_per_kwh: 0.15,
                profit_delta_threshold: 3.0, // Lower threshold for more responsive switching
                min_dwell_time_minutes: 10,
                update_interval_minutes: Some(5),
                pool_fee_percent: Some(0.5), // Effective 50% lower fee for BUNKER POOL
                enabled_algorithms: vec!["RandomX".to_string(), "Ethash".to_string(), "SHA256".to_string(), "Scrypt".to_string()],
                disabled_algorithms: vec![],
                #[cfg(feature = "proxy")]
                proxy_url: None,
            },
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    password_hint: Option<String>,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config_dir = config_dir()
            .ok_or_else(|| anyhow!("Could not determine config directory"))?
            .join("bunker-miner");
        
        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;
        
        let config_path = config_dir.join("config.toml.encrypted");
        
        Ok(ConfigManager {
            config_path,
            password_hint: None,
        })
    }
    
    pub fn config_exists(&self) -> bool {
        self.config_path.exists()
    }
    
    pub async fn load_config(&mut self) -> Result<Config> {
        if !self.config_exists() {
            return Ok(self.create_initial_config().await?);
        }
        
        println!("Configuration file found: {}", self.config_path.display());
        
        // Read encrypted file
        let encrypted_data = fs::read(&self.config_path)
            .context("Failed to read encrypted config file")?;
        
        // Get password from user
        let password = self.prompt_password("Enter configuration password: ")?;
        
        // Decrypt the data
        let decrypted_data = self.decrypt_data(&encrypted_data, &password)
            .context("Failed to decrypt configuration - incorrect password or corrupted file")?;
        
        // Parse TOML
        let config: Config = toml::from_str(&decrypted_data)
            .context("Failed to parse configuration file")?;
        
        // Validate configuration
        self.validate_config(&config)?;
        
        println!("✓ Configuration loaded and validated successfully");
        
        Ok(config)
    }
    
    pub async fn save_config(&self, config: &Config, password: Option<&Secret<String>>) -> Result<()> {
        // Validate configuration before saving
        self.validate_config(config)?;
        
        // Serialize to TOML
        let toml_data = toml::to_string_pretty(config)
            .context("Failed to serialize configuration to TOML")?;
        
        // Get or prompt for password
        let password = match password {
            Some(pwd) => pwd.clone(),
            None => {
                let pwd = self.prompt_password("Enter password to encrypt configuration: ")?;
                Secret::new(pwd)
            }
        };
        
        // Encrypt the data
        let encrypted_data = self.encrypt_data(&toml_data, &password)
            .context("Failed to encrypt configuration data")?;
        
        // Write to file
        fs::write(&self.config_path, encrypted_data)
            .context("Failed to write encrypted config file")?;
        
        println!("✓ Configuration saved and encrypted successfully");
        println!("  Location: {}", self.config_path.display());
        
        Ok(())
    }
    
    async fn create_initial_config(&mut self) -> Result<Config> {
        println!("🔧 BUNKER MINER - Initial Configuration Setup");
        println!("============================================");
        println!();
        println!("No configuration file found. Creating initial setup...");
        println!();
        println!("⚠️  IMPORTANT: Your configuration contains sensitive wallet addresses");
        println!("   and will be encrypted with a password you choose.");
        println!("   If you forget this password, your configuration will be unrecoverable!");
        println!();
        
        let password = self.prompt_new_password()?;
        
        let config = Config::default();
        
        println!();
        println!("📝 Creating default configuration template...");
        println!("   You can edit the encrypted file later with the 'config edit' command");
        println!();
        
        self.save_config(&config, Some(&password)).await?;
        
        println!();
        println!("✅ Initial configuration created successfully!");
        println!();
        println!("⚠️  REMEMBER: Update your wallet addresses before mining!");
        println!("   Default addresses are placeholders and will not work.");
        println!();
        
        Ok(config)
    }
    
    fn prompt_password(&self, prompt: &str) -> Result<Secret<String>> {
        print!("{}", prompt);
        std::io::stdout().flush()
            .context("Failed to flush stdout")?;
        
        let password = rpassword::read_password()
            .context("Failed to read password")?;
        
        if password.is_empty() {
            return Err(anyhow!("Password cannot be empty"));
        }
        
        Ok(Secret::new(password))
    }
    
    fn prompt_new_password(&self) -> Result<Secret<String>> {
        loop {
            let password1 = self.prompt_password("Choose a password for configuration encryption: ")?;
            
            if password1.expose_secret().len() < 8 {
                println!("❌ Password must be at least 8 characters long");
                continue;
            }
            
            let password2 = self.prompt_password("Confirm password: ")?;
            
            if password1.expose_secret() == password2.expose_secret() {
                return Ok(password1);
            } else {
                println!("❌ Passwords do not match. Please try again.");
            }
        }
    }
    
    fn encrypt_data(&self, data: &str, password: &Secret<String>) -> Result<Vec<u8>> {
        let encryptor = Encryptor::with_user_passphrase(password.expose_secret().clone());
        
        let mut encrypted = vec![];
        {
            let mut writer = encryptor.wrap_output(&mut encrypted)
                .context("Failed to create encryption writer")?;
            writer.write_all(data.as_bytes())
                .context("Failed to write data to encryptor")?;
            writer.finish()
                .context("Failed to finalize encryption")?;
        }
        
        Ok(encrypted)
    }
    
    fn decrypt_data(&self, encrypted_data: &[u8], password: &Secret<String>) -> Result<String> {
        let decryptor = match Decryptor::new(encrypted_data)
            .context("Failed to create decryptor")? {
            Decryptor::Passphrase(d) => d,
            _ => return Err(anyhow!("Unsupported decryption method")),
        };
        
        let mut decrypted = vec![];
        {
            let mut reader = decryptor.decrypt(password.expose_secret(), None)
                .context("Failed to decrypt - incorrect password")?;
            
            use std::io::Read;
            reader.read_to_end(&mut decrypted)
                .context("Failed to read decrypted data")?;
        }
        
        String::from_utf8(decrypted)
            .context("Decrypted data is not valid UTF-8")
    }
    
    fn validate_config(&self, config: &Config) -> Result<()> {
        // Validate active references exist
        if !config.wallets.contains_key(&config.mining.active_wallet) {
            return Err(anyhow!("Active wallet '{}' not found in wallets configuration", 
                             config.mining.active_wallet));
        }
        
        if !config.pools.contains_key(&config.mining.active_pool) {
            return Err(anyhow!("Active pool '{}' not found in pools configuration", 
                             config.mining.active_pool));
        }
        
        // Validate wallet addresses are not defaults
        let active_wallet = &config.wallets[&config.mining.active_wallet];
        if active_wallet.address.starts_with("0x0000000000000000000") || 
           active_wallet.address.starts_with("4444444444444444444") {
            return Err(anyhow!("Please update your wallet address - default placeholder addresses cannot be used for mining"));
        }
        
        // Validate coin consistency
        if active_wallet.coin != config.mining.active_coin {
            return Err(anyhow!("Active wallet coin '{}' does not match active mining coin '{}'", 
                             active_wallet.coin, config.mining.active_coin));
        }
        
        let active_pool = &config.pools[&config.mining.active_pool];
        if active_pool.coin != config.mining.active_coin {
            return Err(anyhow!("Active pool coin '{}' does not match active mining coin '{}'", 
                             active_pool.coin, config.mining.active_coin));
        }
        
        // Validate security settings
        if config.security.max_restart_attempts == 0 {
            return Err(anyhow!("max_restart_attempts must be greater than 0"));
        }
        
        if config.security.restart_delay_seconds == 0 {
            return Err(anyhow!("restart_delay_seconds must be greater than 0"));
        }
        
        if config.security.max_restart_delay_seconds < config.security.restart_delay_seconds {
            return Err(anyhow!("max_restart_delay_seconds must be >= restart_delay_seconds"));
        }
        
        // Validate gRPC settings
        if config.grpc.port == 0 {
            return Err(anyhow!("gRPC port must be greater than 0"));
        }
        
        if config.grpc.max_connections == 0 {
            return Err(anyhow!("gRPC max_connections must be greater than 0"));
        }
        
        if config.grpc.connection_timeout_seconds == 0 {
            return Err(anyhow!("gRPC connection_timeout_seconds must be greater than 0"));
        }
        
        if config.grpc.request_timeout_seconds == 0 {
            return Err(anyhow!("gRPC request_timeout_seconds must be greater than 0"));
        }
        
        // Validate TLS configuration for remote access
        if config.grpc.bind_address != "127.0.0.1" && config.grpc.bind_address != "localhost" {
            if !config.grpc.tls_enabled {
                return Err(anyhow!("TLS must be enabled for non-localhost gRPC binding"));
            }
            
            if config.grpc.tls_cert_path.is_none() || config.grpc.tls_key_path.is_none() {
                return Err(anyhow!("TLS cert and key paths must be specified for remote gRPC access"));
            }
        }
        
        Ok(())
    }
    
    pub fn get_config_path(&self) -> &Path {
        &self.config_path
    }
}

impl Config {
    pub fn get_active_wallet(&self) -> Result<&WalletConfig> {
        self.wallets.get(&self.mining.active_wallet)
            .ok_or_else(|| anyhow!("Active wallet '{}' not found", self.mining.active_wallet))
    }
    
    pub fn get_active_pool(&self) -> Result<&PoolConfig> {
        self.pools.get(&self.mining.active_pool)
            .ok_or_else(|| anyhow!("Active pool '{}' not found", self.mining.active_pool))
    }
    
    pub fn get_backup_pools(&self) -> Vec<&PoolConfig> {
        self.mining.backup_pools.iter()
            .filter_map(|pool_name| self.pools.get(pool_name))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_config_encryption_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml.encrypted");
        
        let mut config_manager = ConfigManager {
            config_path,
            password_hint: None,
        };
        
        let original_config = Config::default();
        let password = Secret::new("test_password_123".to_string());
        
        // Save encrypted config
        config_manager.save_config(&original_config, Some(&password)).await.unwrap();
        
        // Verify file exists and is not plaintext
        assert!(config_manager.config_exists());
        let file_contents = fs::read_to_string(&config_manager.config_path).unwrap();
        assert!(!file_contents.contains("ethereum"));  // Should not contain plaintext
        
        // Load and decrypt config
        // Note: This would require interactive password input in real usage
    }
    
    #[test]
    fn test_config_validation() {
        let config_manager = ConfigManager {
            config_path: PathBuf::new(),
            password_hint: None,
        };
        
        let valid_config = Config::default();
        assert!(config_manager.validate_config(&valid_config).is_err()); // Should fail due to default addresses
        
        // Create a valid config with real addresses
        let mut valid_config = Config::default();
        valid_config.wallets.get_mut("ethereum_main").unwrap().address = 
            "0x742d35Cc6635C0532925a3b8D400cdFb7021f39f".to_string();
        
        assert!(config_manager.validate_config(&valid_config).is_ok());
    }
    
    #[test]
    fn test_config_getters() {
        let mut config = Config::default();
        config.wallets.get_mut("ethereum_main").unwrap().address = 
            "0x742d35Cc6635C0532925a3b8D400cdFb7021f39f".to_string();
        
        let active_wallet = config.get_active_wallet().unwrap();
        assert_eq!(active_wallet.coin, "ethereum");
        assert_eq!(active_wallet.address, "0x742d35Cc6635C0532925a3b8D400cdFb7021f39f");
        
        let active_pool = config.get_active_pool().unwrap();
        assert_eq!(active_pool.coin, "ethereum");
        assert_eq!(active_pool.port, 9999);
    }
}