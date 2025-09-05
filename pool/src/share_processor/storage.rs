// BUNKER POOL - Share Storage System
// High-performance Redis-based share persistence for PPLNS calculations

use std::collections::HashMap;
use redis::{AsyncCommands, Client, RedisResult};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::stratum::protocol::{Algorithm, ShareSubmission};
use super::validator::ValidationResult;

/// Configuration for Redis share storage
#[derive(Debug, Clone)]
pub struct ShareStorageConfig {
    pub redis_url: String,
    pub key_prefix: String,
    pub round_expiry_seconds: u64,
    pub stats_expiry_seconds: u64,
    pub max_retry_attempts: u32,
    pub connection_timeout_ms: u64,
}

impl Default for ShareStorageConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            key_prefix: "bunker_pool".to_string(),
            round_expiry_seconds: 86400 * 7, // 7 days
            stats_expiry_seconds: 86400 * 30, // 30 days
            max_retry_attempts: 3,
            connection_timeout_ms: 5000,
        }
    }
}

/// Share record stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareRecord {
    pub worker_name: String,
    pub job_id: String,
    pub difficulty: f64,
    pub algorithm: Algorithm,
    pub is_block: bool,
    pub block_height: Option<u64>,
    pub hash: String,
    pub submitted_at: DateTime<Utc>,
    pub miner_id: Uuid,
    pub ip_address: String,
}

/// Miner statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerStats {
    pub worker_name: String,
    pub total_shares: u64,
    pub valid_shares: u64,
    pub invalid_shares: u64,
    pub blocks_found: u64,
    pub total_difficulty: f64,
    pub first_share_time: DateTime<Utc>,
    pub last_share_time: DateTime<Utc>,
    pub average_difficulty: f64,
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total_shares: u64,
    pub valid_shares: u64,
    pub invalid_shares: u64,
    pub blocks_found: u64,
    pub total_difficulty: f64,
    pub unique_miners: u64,
    pub current_round_shares: u64,
    pub last_block_time: Option<DateTime<Utc>>,
    pub pool_hashrate: f64,
}

/// High-performance Redis-based share storage
pub struct ShareStorage {
    config: ShareStorageConfig,
    client: Client,
    algorithm: Algorithm,
}

impl ShareStorage {
    pub fn new(config: ShareStorageConfig, algorithm: Algorithm) -> Result<Self, anyhow::Error> {
        let client = Client::open(config.redis_url.as_str())?;
        
        Ok(Self {
            config,
            client,
            algorithm,
        })
    }

    /// Store a valid share in Redis
    pub async fn store_share(
        &self,
        miner_id: Uuid,
        share: &ShareSubmission,
        validation_result: &ValidationResult,
        block_height: u64,
        ip_address: String,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.get_connection().await?;
        
        let share_record = ShareRecord {
            worker_name: share.worker_name.clone(),
            job_id: share.job_id.clone(),
            difficulty: validation_result.difficulty,
            algorithm: self.algorithm,
            is_block: validation_result.is_block,
            block_height: if validation_result.is_block { Some(block_height) } else { None },
            hash: validation_result.hash.clone().unwrap_or_default(),
            submitted_at: Utc::now(),
            miner_id,
            ip_address,
        };

        // Use Redis pipeline for atomic operations
        let mut pipe = redis::pipe();

        // 1. Add share to current round (PPLNS calculation)
        let round_key = format!("{}:round:{}:shares", self.config.key_prefix, block_height);
        pipe.zadd(&round_key, &share.worker_name, validation_result.difficulty);
        pipe.expire(&round_key, self.config.round_expiry_seconds as i64);

        // 2. Add share to miner's history
        let miner_key = format!("{}:miner:{}:shares", self.config.key_prefix, share.worker_name);
        let share_data = serde_json::to_string(&share_record)?;
        pipe.lpush(&miner_key, &share_data);
        pipe.ltrim(&miner_key, 0, 999); // Keep last 1000 shares
        pipe.expire(&miner_key, self.config.stats_expiry_seconds as i64);

        // 3. Update miner statistics
        self.update_miner_stats(&mut pipe, &share.worker_name, validation_result).await;

        // 4. Update pool statistics
        self.update_pool_stats(&mut pipe, validation_result, block_height).await;

        // 5. If this is a block, record it specially
        if validation_result.is_block {
            let block_key = format!("{}:blocks", self.config.key_prefix);
            let block_data = serde_json::json!({
                "height": block_height,
                "finder": share.worker_name,
                "hash": validation_result.hash,
                "difficulty": validation_result.difficulty,
                "time": Utc::now(),
                "algorithm": self.algorithm
            });
            pipe.lpush(&block_key, serde_json::to_string(&block_data)?);
            pipe.ltrim(&block_key, 0, 99); // Keep last 100 blocks

            info!("Block found! Height: {}, Finder: {}, Hash: {}", 
                  block_height, share.worker_name, validation_result.hash.as_deref().unwrap_or("unknown"));
        }

        // Execute pipeline
        let _: () = pipe.query_async(&mut conn).await?;

        debug!("Stored share for {}: difficulty {:.2}", 
               share.worker_name, validation_result.difficulty);

        Ok(())
    }

    /// Store an invalid share for statistics
    pub async fn store_invalid_share(
        &self,
        miner_id: Uuid,
        share: &ShareSubmission,
        error: &str,
        ip_address: String,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.get_connection().await?;

        let invalid_share = serde_json::json!({
            "worker_name": share.worker_name,
            "job_id": share.job_id,
            "error": error,
            "time": Utc::now(),
            "miner_id": miner_id,
            "ip_address": ip_address
        });

        let mut pipe = redis::pipe();

        // Store invalid share
        let invalid_key = format!("{}:invalid_shares", self.config.key_prefix);
        pipe.lpush(&invalid_key, serde_json::to_string(&invalid_share)?);
        pipe.ltrim(&invalid_key, 0, 999); // Keep last 1000 invalid shares
        pipe.expire(&invalid_key, self.config.stats_expiry_seconds as i64);

        // Update miner stats (invalid share count)
        let miner_stats_key = format!("{}:miner:{}:stats", self.config.key_prefix, share.worker_name);
        pipe.hincrby(&miner_stats_key, "invalid_shares", 1);
        pipe.expire(&miner_stats_key, self.config.stats_expiry_seconds as i64);

        // Update pool stats
        let pool_stats_key = format!("{}:pool:stats", self.config.key_prefix);
        pipe.hincrby(&pool_stats_key, "invalid_shares", 1);

        let _: () = pipe.query_async(&mut conn).await?;

        warn!("Invalid share from {}: {}", share.worker_name, error);

        Ok(())
    }

    /// Get shares for current round (PPLNS calculation)
    pub async fn get_round_shares(&self, block_height: u64) -> Result<HashMap<String, f64>, anyhow::Error> {
        let mut conn = self.get_connection().await?;
        let round_key = format!("{}:round:{}:shares", self.config.key_prefix, block_height);
        
        let shares: Vec<(String, f64)> = conn.zrevrange_withscores(&round_key, 0, -1).await?;
        
        Ok(shares.into_iter().collect())
    }

    /// Get miner statistics
    pub async fn get_miner_stats(&self, worker_name: &str) -> Result<Option<MinerStats>, anyhow::Error> {
        let mut conn = self.get_connection().await?;
        let stats_key = format!("{}:miner:{}:stats", self.config.key_prefix, worker_name);
        
        let stats: HashMap<String, String> = conn.hgetall(&stats_key).await?;
        
        if stats.is_empty() {
            return Ok(None);
        }

        let miner_stats = MinerStats {
            worker_name: worker_name.to_string(),
            total_shares: stats.get("total_shares").and_then(|s| s.parse().ok()).unwrap_or(0),
            valid_shares: stats.get("valid_shares").and_then(|s| s.parse().ok()).unwrap_or(0),
            invalid_shares: stats.get("invalid_shares").and_then(|s| s.parse().ok()).unwrap_or(0),
            blocks_found: stats.get("blocks_found").and_then(|s| s.parse().ok()).unwrap_or(0),
            total_difficulty: stats.get("total_difficulty").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            first_share_time: stats.get("first_share_time")
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            last_share_time: stats.get("last_share_time")
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            average_difficulty: stats.get("average_difficulty").and_then(|s| s.parse().ok()).unwrap_or(0.0),
        };

        Ok(Some(miner_stats))
    }

    /// Get pool statistics
    pub async fn get_pool_stats(&self) -> Result<PoolStats, anyhow::Error> {
        let mut conn = self.get_connection().await?;
        let stats_key = format!("{}:pool:stats", self.config.key_prefix);
        
        let stats: HashMap<String, String> = conn.hgetall(&stats_key).await?;

        let pool_stats = PoolStats {
            total_shares: stats.get("total_shares").and_then(|s| s.parse().ok()).unwrap_or(0),
            valid_shares: stats.get("valid_shares").and_then(|s| s.parse().ok()).unwrap_or(0),
            invalid_shares: stats.get("invalid_shares").and_then(|s| s.parse().ok()).unwrap_or(0),
            blocks_found: stats.get("blocks_found").and_then(|s| s.parse().ok()).unwrap_or(0),
            total_difficulty: stats.get("total_difficulty").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            unique_miners: stats.get("unique_miners").and_then(|s| s.parse().ok()).unwrap_or(0),
            current_round_shares: stats.get("current_round_shares").and_then(|s| s.parse().ok()).unwrap_or(0),
            last_block_time: stats.get("last_block_time")
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            pool_hashrate: stats.get("pool_hashrate").and_then(|s| s.parse().ok()).unwrap_or(0.0),
        };

        Ok(pool_stats)
    }

    /// Get recent blocks found by the pool
    pub async fn get_recent_blocks(&self, limit: i64) -> Result<Vec<serde_json::Value>, anyhow::Error> {
        let mut conn = self.get_connection().await?;
        let blocks_key = format!("{}:blocks", self.config.key_prefix);
        
        let blocks: Vec<String> = conn.lrange(&blocks_key, 0, limit - 1).await?;
        
        let mut result = Vec::new();
        for block_str in blocks {
            if let Ok(block_data) = serde_json::from_str::<serde_json::Value>(&block_str) {
                result.push(block_data);
            }
        }

        Ok(result)
    }

    /// Calculate pool hashrate based on recent shares
    pub async fn calculate_pool_hashrate(&self, window_minutes: i64) -> Result<f64, anyhow::Error> {
        let mut conn = self.get_connection().await?;
        
        let cutoff_time = Utc::now() - chrono::Duration::minutes(window_minutes);
        let cutoff_timestamp = cutoff_time.timestamp();

        // Get all miner keys
        let pattern = format!("{}:miner:*:shares", self.config.key_prefix);
        let keys: Vec<String> = conn.keys(&pattern).await?;

        let mut total_difficulty = 0.0;
        let mut share_count = 0;

        for key in keys {
            // Get recent shares for this miner
            let shares: Vec<String> = conn.lrange(&key, 0, -1).await?;
            
            for share_str in shares {
                if let Ok(share) = serde_json::from_str::<ShareRecord>(&share_str) {
                    if share.submitted_at.timestamp() >= cutoff_timestamp {
                        total_difficulty += share.difficulty;
                        share_count += 1;
                    }
                }
            }
        }

        // Calculate hashrate: (total_difficulty * 2^32) / time_window_seconds
        let time_window_seconds = (window_minutes * 60) as f64;
        let hashrate = if time_window_seconds > 0.0 {
            (total_difficulty * 4294967296.0) / time_window_seconds // 2^32
        } else {
            0.0
        };

        debug!("Pool hashrate calculation: {} shares, {:.2} total difficulty, {:.2} H/s", 
               share_count, total_difficulty, hashrate);

        Ok(hashrate)
    }

    /// Update miner statistics in pipeline
    async fn update_miner_stats(
        &self,
        pipe: &mut redis::Pipeline,
        worker_name: &str,
        validation_result: &ValidationResult,
    ) {
        let stats_key = format!("{}:miner:{}:stats", self.config.key_prefix, worker_name);
        
        pipe.hincrby(&stats_key, "total_shares", 1);
        pipe.hincrby(&stats_key, "valid_shares", 1);
        pipe.hincrbyfloat(&stats_key, "total_difficulty", validation_result.difficulty);
        
        if validation_result.is_block {
            pipe.hincrby(&stats_key, "blocks_found", 1);
        }

        // Set timestamps
        let now = Utc::now().to_rfc3339();
        pipe.hsetnx(&stats_key, "first_share_time", &now);
        pipe.hset(&stats_key, "last_share_time", &now);
        
        // Calculate and update average difficulty
        // This would be more accurate with a separate calculation
        let avg_difficulty = validation_result.difficulty; // Simplified
        pipe.hset(&stats_key, "average_difficulty", avg_difficulty);
        
        pipe.expire(&stats_key, self.config.stats_expiry_seconds as i64);
    }

    /// Update pool statistics in pipeline
    async fn update_pool_stats(
        &self,
        pipe: &mut redis::Pipeline,
        validation_result: &ValidationResult,
        block_height: u64,
    ) {
        let stats_key = format!("{}:pool:stats", self.config.key_prefix);
        
        pipe.hincrby(&stats_key, "total_shares", 1);
        pipe.hincrby(&stats_key, "valid_shares", 1);
        pipe.hincrbyfloat(&stats_key, "total_difficulty", validation_result.difficulty);
        pipe.hincrby(&stats_key, "current_round_shares", 1);
        
        if validation_result.is_block {
            pipe.hincrby(&stats_key, "blocks_found", 1);
            pipe.hset(&stats_key, "last_block_time", Utc::now().to_rfc3339());
            pipe.hset(&stats_key, "current_round_shares", 0); // Reset round shares
        }

        // Update current block height
        pipe.hset(&stats_key, "current_height", block_height);
    }

    /// Get Redis connection with retry logic
    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, anyhow::Error> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.config.max_retry_attempts {
            match self.client.get_multiplexed_async_connection().await {
                Ok(conn) => return Ok(conn),
                Err(e) => {
                    last_error = Some(e);
                    attempts += 1;
                    if attempts < self.config.max_retry_attempts {
                        tokio::time::sleep(std::time::Duration::from_millis(100 * attempts as u64)).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Failed to connect to Redis after {} attempts: {:?}", 
                            self.config.max_retry_attempts, last_error))
    }

    /// Health check - ping Redis
    pub async fn health_check(&self) -> Result<(), anyhow::Error> {
        let mut conn = self.get_connection().await?;
        let _: String = conn.ping().await?;
        Ok(())
    }

    /// Get database info
    pub async fn get_db_info(&self) -> Result<HashMap<String, String>, anyhow::Error> {
        let mut conn = self.get_connection().await?;
        let info: String = conn.info().await?;
        
        let mut result = HashMap::new();
        for line in info.lines() {
            if let Some(pos) = line.find(':') {
                let key = line[..pos].to_string();
                let value = line[pos + 1..].to_string();
                result.insert(key, value);
            }
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stratum::protocol::Algorithm;

    fn create_test_config() -> ShareStorageConfig {
        ShareStorageConfig {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            key_prefix: "test_bunker_pool".to_string(),
            round_expiry_seconds: 3600, // 1 hour for tests
            stats_expiry_seconds: 7200, // 2 hours for tests
            max_retry_attempts: 2,
            connection_timeout_ms: 1000,
        }
    }

    #[tokio::test]
    async fn test_storage_creation() {
        let config = create_test_config();
        // This test requires Redis to be running
        if let Ok(storage) = ShareStorage::new(config, Algorithm::Sha256) {
            assert_eq!(storage.algorithm, Algorithm::Sha256);
        }
    }

    #[test]
    fn test_share_record_serialization() {
        let share_record = ShareRecord {
            worker_name: "test_worker".to_string(),
            job_id: "job_123".to_string(),
            difficulty: 100.0,
            algorithm: Algorithm::Sha256,
            is_block: false,
            block_height: None,
            hash: "abcdef".to_string(),
            submitted_at: Utc::now(),
            miner_id: Uuid::new_v4(),
            ip_address: "192.168.1.1".to_string(),
        };

        let json = serde_json::to_string(&share_record).unwrap();
        let deserialized: ShareRecord = serde_json::from_str(&json).unwrap();
        
        assert_eq!(share_record.worker_name, deserialized.worker_name);
        assert_eq!(share_record.difficulty, deserialized.difficulty);
    }
}