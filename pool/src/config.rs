// BUNKER POOL - Configuration Management
// Centralized configuration for all pool services

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

use crate::stratum::protocol::Algorithm;
use crate::stratum::connection::ConnectionConfig;
use crate::stratum::job_manager::CoinDaemonConfig;
use crate::share_processor::processor::ShareProcessorConfig;
use crate::share_processor::storage::ShareStorageConfig;

/// Main BUNKER POOL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub pool: PoolSettings,
    pub stratum: StratumConfig,
    pub share_processor: ProcessorConfig,
    pub storage: StorageConfig,
    pub coin_daemon: CoinConfig,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
}

/// Pool-wide settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSettings {
    pub name: String,
    pub algorithm: Algorithm,
    pub fee_percentage: f64,
    pub minimum_payout: f64,
    pub payout_address: String,
    pub donation_percentage: f64,
}

/// Stratum server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumConfig {
    pub bind_address: SocketAddr,
    pub max_connections: usize,
    pub default_difficulty: f64,
    pub difficulty_adjustment: DifficultyConfig,
    pub connection_timeout_seconds: u64,
    pub max_message_size: usize,
}

/// Difficulty adjustment settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyConfig {
    pub target_time_seconds: u64,
    pub adjustment_window: usize,
    pub min_difficulty: f64,
    pub max_difficulty: f64,
    pub max_adjustment_factor: f64,
}

/// Share processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    pub worker_threads: usize,
    pub validation_timeout_ms: u64,
    pub max_pending_shares: usize,
    pub duplicate_check_window_minutes: u64,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub redis_url: String,
    pub key_prefix: String,
    pub round_expiry_days: u64,
    pub stats_expiry_days: u64,
    pub max_retry_attempts: u32,
}

/// Coin daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinConfig {
    pub rpc_url: String,
    pub rpc_username: String,
    pub rpc_password: String,
    pub poll_interval_seconds: u64,
    pub timeout_seconds: u64,
    pub coinbase_address: String,
    pub extra_data: Option<String>,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub bind_address: SocketAddr,
    pub collection_interval_seconds: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
    pub json_format: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            pool: PoolSettings {
                name: "BUNKER POOL".to_string(),
                algorithm: Algorithm::Sha256,
                fee_percentage: 1.0,
                minimum_payout: 0.01,
                payout_address: "bc1qbunkerpooltestaddress".to_string(),
                donation_percentage: 0.5,
            },
            stratum: StratumConfig {
                bind_address: "0.0.0.0:3333".parse().unwrap(),
                max_connections: 10000,
                default_difficulty: 1.0,
                difficulty_adjustment: DifficultyConfig {
                    target_time_seconds: 15,
                    adjustment_window: 10,
                    min_difficulty: 0.01,
                    max_difficulty: 1000000.0,
                    max_adjustment_factor: 4.0,
                },
                connection_timeout_seconds: 300,
                max_message_size: 8192,
            },
            share_processor: ProcessorConfig {
                worker_threads: num_cpus::get().max(4),
                validation_timeout_ms: 1000,
                max_pending_shares: 50000,
                duplicate_check_window_minutes: 10,
            },
            storage: StorageConfig {
                redis_url: "redis://localhost:6379".to_string(),
                key_prefix: "bunker_pool".to_string(),
                round_expiry_days: 7,
                stats_expiry_days: 30,
                max_retry_attempts: 3,
            },
            coin_daemon: CoinConfig {
                rpc_url: "http://localhost:8332".to_string(),
                rpc_username: "bunker_user".to_string(),
                rpc_password: "secure_password_123".to_string(),
                poll_interval_seconds: 10,
                timeout_seconds: 30,
                coinbase_address: "bc1qbunkerpooltestaddress".to_string(),
                extra_data: Some("BUNKER POOL".to_string()),
            },
            metrics: MetricsConfig {
                enabled: true,
                bind_address: "0.0.0.0:9090".parse().unwrap(),
                collection_interval_seconds: 60,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_path: None,
                json_format: false,
            },
        }
    }
}

impl PoolConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let config: PoolConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<(), anyhow::Error> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to Stratum server config
    pub fn to_stratum_server_config(&self) -> crate::stratum::server::StratumServerConfig {
        crate::stratum::server::StratumServerConfig {
            bind_address: self.stratum.bind_address,
            algorithm: self.pool.algorithm,
            max_connections: self.stratum.max_connections,
            connection_config: ConnectionConfig {
                read_timeout: Duration::from_secs(30),
                write_timeout: Duration::from_secs(10),
                ping_interval: Duration::from_secs(60),
                max_idle_time: Duration::from_secs(self.stratum.connection_timeout_seconds),
                max_message_size: self.stratum.max_message_size,
                default_difficulty: self.stratum.default_difficulty,
            },
            coin_daemon: self.to_coin_daemon_config(),
        }
    }

    /// Convert to share processor config
    pub fn to_share_processor_config(&self) -> ShareProcessorConfig {
        ShareProcessorConfig {
            algorithm: self.pool.algorithm,
            worker_threads: self.share_processor.worker_threads,
            validation_timeout_ms: self.share_processor.validation_timeout_ms,
            storage_config: self.to_share_storage_config(),
            max_pending_shares: self.share_processor.max_pending_shares,
        }
    }

    /// Convert to coin daemon config
    pub fn to_coin_daemon_config(&self) -> CoinDaemonConfig {
        CoinDaemonConfig {
            url: self.coin_daemon.rpc_url.clone(),
            username: self.coin_daemon.rpc_username.clone(),
            password: self.coin_daemon.rpc_password.clone(),
            algorithm: self.pool.algorithm,
            block_poll_interval: Duration::from_secs(self.coin_daemon.poll_interval_seconds),
            timeout: Duration::from_secs(self.coin_daemon.timeout_seconds),
            coinbase_address: self.coin_daemon.coinbase_address.clone(),
            extra_data: self.coin_daemon.extra_data.clone(),
        }
    }

    /// Convert to share storage config
    pub fn to_share_storage_config(&self) -> ShareStorageConfig {
        ShareStorageConfig {
            redis_url: self.storage.redis_url.clone(),
            key_prefix: self.storage.key_prefix.clone(),
            round_expiry_seconds: self.storage.round_expiry_days * 24 * 60 * 60,
            stats_expiry_seconds: self.storage.stats_expiry_days * 24 * 60 * 60,
            max_retry_attempts: self.storage.max_retry_attempts,
            connection_timeout_ms: 5000,
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.pool.fee_percentage < 0.0 || self.pool.fee_percentage > 50.0 {
            return Err(anyhow::anyhow!("Invalid pool fee percentage: {}", self.pool.fee_percentage));
        }

        if self.pool.minimum_payout <= 0.0 {
            return Err(anyhow::anyhow!("Minimum payout must be positive"));
        }

        if self.stratum.max_connections == 0 || self.stratum.max_connections > 100000 {
            return Err(anyhow::anyhow!("Invalid max connections: {}", self.stratum.max_connections));
        }

        if self.stratum.default_difficulty <= 0.0 {
            return Err(anyhow::anyhow!("Default difficulty must be positive"));
        }

        if self.share_processor.worker_threads == 0 || self.share_processor.worker_threads > 256 {
            return Err(anyhow::anyhow!("Invalid worker thread count: {}", self.share_processor.worker_threads));
        }

        if self.storage.redis_url.is_empty() {
            return Err(anyhow::anyhow!("Redis URL cannot be empty"));
        }

        if self.coin_daemon.rpc_url.is_empty() {
            return Err(anyhow::anyhow!("Coin daemon RPC URL cannot be empty"));
        }

        if self.coin_daemon.coinbase_address.is_empty() {
            return Err(anyhow::anyhow!("Coinbase address cannot be empty"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PoolConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.pool.algorithm, Algorithm::Sha256);
        assert_eq!(config.stratum.max_connections, 10000);
    }

    #[test]
    fn test_config_validation() {
        let mut config = PoolConfig::default();
        
        // Test invalid fee percentage
        config.pool.fee_percentage = 60.0;
        assert!(config.validate().is_err());
        
        // Test invalid minimum payout
        config.pool.fee_percentage = 1.0;
        config.pool.minimum_payout = -1.0;
        assert!(config.validate().is_err());
        
        // Test valid config
        config.pool.minimum_payout = 0.01;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_algorithm_serialization() {
        let config = PoolConfig {
            pool: PoolSettings {
                algorithm: Algorithm::Blake2b,
                ..Default::default()
            },
            ..Default::default()
        };

        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("blake2b"));
        
        let deserialized: PoolConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.pool.algorithm, Algorithm::Blake2b);
    }
}