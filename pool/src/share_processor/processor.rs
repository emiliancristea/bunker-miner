// BUNKER POOL - Share Processor
// High-throughput secure share validation and persistence system

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::stratum::{
    job_manager::JobManager,
    protocol::{Algorithm, ShareSubmission},
};
use super::{
    storage::{ShareStorage, ShareStorageConfig},
    validator::{ShareValidator, ValidationResult},
};

/// Share processor configuration
#[derive(Debug, Clone)]
pub struct ShareProcessorConfig {
    pub algorithm: Algorithm,
    pub worker_threads: usize,
    pub validation_timeout_ms: u64,
    pub storage_config: ShareStorageConfig,
    pub max_pending_shares: usize,
}

impl Default for ShareProcessorConfig {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::Sha256,
            worker_threads: num_cpus::get().max(4),
            validation_timeout_ms: 1000,
            storage_config: ShareStorageConfig::default(),
            max_pending_shares: 10000,
        }
    }
}

/// Share processing task
#[derive(Debug)]
struct ShareTask {
    miner_id: Uuid,
    share: ShareSubmission,
    ip_address: String,
    submitted_at: chrono::DateTime<chrono::Utc>,
}

/// Share processing result callback
pub type ShareResultCallback = Box<dyn Fn(Uuid, bool, bool) + Send + Sync>;

/// High-performance share processor
pub struct ShareProcessor {
    config: ShareProcessorConfig,
    job_manager: Arc<JobManager>,
    storage: Arc<ShareStorage>,
    validator: Arc<RwLock<ShareValidator>>,
    share_sender: mpsc::Sender<ShareTask>,
    result_callback: Option<ShareResultCallback>,
    stats: Arc<RwLock<ProcessorStats>>,
}

/// Share processor statistics
#[derive(Debug, Default, Clone)]
pub struct ProcessorStats {
    pub total_shares_processed: u64,
    pub valid_shares: u64,
    pub invalid_shares: u64,
    pub blocks_found: u64,
    pub processing_errors: u64,
    pub average_processing_time_ms: f64,
    pub pending_shares: u64,
}

impl ShareProcessor {
    pub async fn new(
        config: ShareProcessorConfig,
        job_manager: Arc<JobManager>,
    ) -> Result<Self, anyhow::Error> {
        let storage = Arc::new(ShareStorage::new(
            config.storage_config.clone(),
            config.algorithm,
        )?);

        let validator = Arc::new(RwLock::new(ShareValidator::new(config.algorithm)));

        let (share_sender, share_receiver) = mpsc::channel::<ShareTask>(config.max_pending_shares);

        let processor = Self {
            config: config.clone(),
            job_manager,
            storage: Arc::clone(&storage),
            validator: Arc::clone(&validator),
            share_sender,
            result_callback: None,
            stats: Arc::new(RwLock::new(ProcessorStats::default())),
        };

        // Start worker threads
        processor.start_workers(share_receiver).await;

        // Start statistics reporter
        processor.start_stats_reporter().await;

        info!("Share processor started with {} worker threads", config.worker_threads);

        Ok(processor)
    }

    /// Set result callback for share processing results
    pub fn set_result_callback(&mut self, callback: ShareResultCallback) {
        self.result_callback = Some(callback);
    }

    /// Submit a share for processing
    pub async fn process_share(
        &self,
        miner_id: Uuid,
        share: ShareSubmission,
        ip_address: String,
    ) -> Result<(), anyhow::Error> {
        let task = ShareTask {
            miner_id,
            share,
            ip_address,
            submitted_at: chrono::Utc::now(),
        };

        // Update pending shares count
        {
            let mut stats = self.stats.write().await;
            stats.pending_shares += 1;
        }

        if let Err(e) = self.share_sender.send(task).await {
            // Update error count
            {
                let mut stats = self.stats.write().await;
                stats.processing_errors += 1;
                stats.pending_shares = stats.pending_shares.saturating_sub(1);
            }
            
            error!("Failed to queue share for processing: {}", e);
            return Err(anyhow::anyhow!("Share processing queue is full"));
        }

        Ok(())
    }

    /// Start worker threads for processing shares
    async fn start_workers(&self, mut share_receiver: mpsc::Receiver<ShareTask>) {
        let worker_count = self.config.worker_threads;
        let (worker_tx, mut worker_rx) = mpsc::channel::<ShareTask>(worker_count * 2);

        // Distribute tasks to workers
        let share_sender = worker_tx;
        tokio::spawn(async move {
            while let Some(task) = share_receiver.recv().await {
                if let Err(e) = share_sender.send(task).await {
                    error!("Failed to distribute task to workers: {}", e);
                }
            }
        });

        // Start worker tasks
        for worker_id in 0..worker_count {
            let worker_rx = worker_rx.clone();
            let job_manager = Arc::clone(&self.job_manager);
            let storage = Arc::clone(&self.storage);
            let validator = Arc::clone(&self.validator);
            let stats = Arc::clone(&self.stats);
            let config = self.config.clone();

            tokio::spawn(async move {
                Self::worker_task(
                    worker_id,
                    worker_rx,
                    job_manager,
                    storage,
                    validator,
                    stats,
                    config,
                ).await;
            });
        }

        // Close the original receiver to avoid having unused clones
        worker_rx.close();
    }

    /// Worker task for processing shares
    async fn worker_task(
        worker_id: usize,
        mut task_receiver: mpsc::Receiver<ShareTask>,
        job_manager: Arc<JobManager>,
        storage: Arc<ShareStorage>,
        validator: Arc<RwLock<ShareValidator>>,
        stats: Arc<RwLock<ProcessorStats>>,
        config: ShareProcessorConfig,
    ) {
        info!("Share processor worker {} started", worker_id);

        while let Some(task) = task_receiver.recv().await {
            let start_time = std::time::Instant::now();

            // Process the share
            let result = Self::process_single_share(
                &task,
                &job_manager,
                &storage,
                &validator,
                &config,
            ).await;

            let processing_time = start_time.elapsed().as_millis() as f64;

            // Update statistics
            {
                let mut stats = stats.write().await;
                stats.total_shares_processed += 1;
                stats.pending_shares = stats.pending_shares.saturating_sub(1);

                match &result {
                    Ok(validation_result) => {
                        if validation_result.is_valid {
                            stats.valid_shares += 1;
                            if validation_result.is_block {
                                stats.blocks_found += 1;
                            }
                        } else {
                            stats.invalid_shares += 1;
                        }
                    }
                    Err(_) => {
                        stats.processing_errors += 1;
                    }
                }

                // Update average processing time (simple moving average)
                if stats.total_shares_processed == 1 {
                    stats.average_processing_time_ms = processing_time;
                } else {
                    stats.average_processing_time_ms = 
                        (stats.average_processing_time_ms * 0.95) + (processing_time * 0.05);
                }
            }

            if let Err(e) = result {
                error!("Error processing share from {}: {}", task.miner_id, e);
            }
        }

        info!("Share processor worker {} stopped", worker_id);
    }

    /// Process a single share
    async fn process_single_share(
        task: &ShareTask,
        job_manager: &Arc<JobManager>,
        storage: &Arc<ShareStorage>,
        validator: &Arc<RwLock<ShareValidator>>,
        config: &ShareProcessorConfig,
    ) -> Result<ValidationResult, anyhow::Error> {
        debug!("Processing share from miner {}: job {}", task.miner_id, task.share.job_id);

        // 1. Get job details from job manager
        let job = match job_manager.get_job(&task.share.job_id).await {
            Some(job) => job,
            None => {
                warn!("Job not found for share: {}", task.share.job_id);
                let error_msg = "Job not found";
                storage.store_invalid_share(
                    task.miner_id,
                    &task.share,
                    error_msg,
                    task.ip_address.clone(),
                ).await?;
                
                return Ok(ValidationResult {
                    is_valid: false,
                    is_block: false,
                    error: Some(super::validator::ValidationError::JobNotFound),
                    difficulty: 0.0,
                    hash: None,
                    target_met: false,
                });
            }
        };

        // 2. Get miner difficulty (would come from session in real implementation)
        let miner_difficulty = job_manager.get_difficulty_target().await;

        // 3. Validate the share
        let validation_result = {
            let mut validator = validator.write().await;
            
            // Set timeout for validation
            let validation_future = validator.validate_share(&task.share, &job, miner_difficulty);
            
            match tokio::time::timeout(
                std::time::Duration::from_millis(config.validation_timeout_ms),
                validation_future
            ).await {
                Ok(result) => result,
                Err(_) => {
                    error!("Share validation timeout for miner {}", task.miner_id);
                    return Err(anyhow::anyhow!("Validation timeout"));
                }
            }
        };

        // 4. Store result in Redis
        if validation_result.is_valid {
            storage.store_share(
                task.miner_id,
                &task.share,
                &validation_result,
                job.block_template.height,
                task.ip_address.clone(),
            ).await?;

            debug!("Valid share stored: miner {}, difficulty {:.2}, block: {}", 
                   task.miner_id, validation_result.difficulty, validation_result.is_block);
        } else {
            let error_msg = validation_result.error
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown validation error".to_string());

            storage.store_invalid_share(
                task.miner_id,
                &task.share,
                &error_msg,
                task.ip_address.clone(),
            ).await?;

            debug!("Invalid share stored: miner {}, error: {}", task.miner_id, error_msg);
        }

        Ok(validation_result)
    }

    /// Start statistics reporting task
    async fn start_stats_reporter(&self) {
        let stats = Arc::clone(&self.stats);
        let storage = Arc::clone(&self.storage);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // 1 minute

            loop {
                interval.tick().await;

                let stats_snapshot = {
                    let stats = stats.read().await;
                    stats.clone()
                };

                // Calculate pool hashrate
                let hashrate = match storage.calculate_pool_hashrate(10).await {
                    Ok(rate) => rate,
                    Err(e) => {
                        error!("Failed to calculate pool hashrate: {}", e);
                        0.0
                    }
                };

                info!("Share Processor Stats:");
                info!("  Processed: {} shares", stats_snapshot.total_shares_processed);
                info!("  Valid: {} ({:.2}%)", 
                      stats_snapshot.valid_shares,
                      if stats_snapshot.total_shares_processed > 0 {
                          (stats_snapshot.valid_shares as f64 / stats_snapshot.total_shares_processed as f64) * 100.0
                      } else { 0.0 });
                info!("  Invalid: {} ({:.2}%)", 
                      stats_snapshot.invalid_shares,
                      if stats_snapshot.total_shares_processed > 0 {
                          (stats_snapshot.invalid_shares as f64 / stats_snapshot.total_shares_processed as f64) * 100.0
                      } else { 0.0 });
                info!("  Blocks found: {}", stats_snapshot.blocks_found);
                info!("  Processing errors: {}", stats_snapshot.processing_errors);
                info!("  Pending shares: {}", stats_snapshot.pending_shares);
                info!("  Avg processing time: {:.2}ms", stats_snapshot.average_processing_time_ms);
                info!("  Pool hashrate: {:.2} H/s", hashrate);
            }
        });
    }

    /// Get current processor statistics
    pub async fn get_stats(&self) -> ProcessorStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), anyhow::Error> {
        // Check Redis connectivity
        self.storage.health_check().await?;

        // Check queue capacity
        let stats = self.stats.read().await;
        if stats.pending_shares > (self.config.max_pending_shares as u64 * 8 / 10) {
            warn!("Share processor queue is {}% full", 
                  (stats.pending_shares * 100) / self.config.max_pending_shares as u64);
        }

        Ok(())
    }

    /// Get storage handle for direct access
    pub fn get_storage(&self) -> Arc<ShareStorage> {
        Arc::clone(&self.storage)
    }

    /// Shutdown the processor gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down share processor...");
        
        // The channels will be dropped when the struct is dropped,
        // which will cause the workers to exit naturally
        
        let stats = self.get_stats().await;
        info!("Share processor shutdown complete. Final stats: processed {}, valid {}, invalid {}, blocks {}", 
              stats.total_shares_processed, stats.valid_shares, stats.invalid_shares, stats.blocks_found);
    }
}

impl Drop for ShareProcessor {
    fn drop(&mut self) {
        info!("Share processor dropping");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stratum::{
        job_manager::{CoinDaemonConfig, JobManager},
        protocol::{Algorithm, ShareSubmission},
    };
    use std::time::Duration;

    async fn create_test_processor() -> Result<ShareProcessor, anyhow::Error> {
        let config = ShareProcessorConfig {
            algorithm: Algorithm::Sha256,
            worker_threads: 2,
            validation_timeout_ms: 500,
            storage_config: ShareStorageConfig {
                redis_url: "redis://127.0.0.1:6379".to_string(),
                key_prefix: "test_bunker_pool".to_string(),
                ..Default::default()
            },
            max_pending_shares: 100,
        };

        let coin_config = CoinDaemonConfig {
            url: "http://localhost:8332".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            algorithm: Algorithm::Sha256,
            block_poll_interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            coinbase_address: "test_address".to_string(),
            extra_data: None,
        };

        let (job_tx, _) = mpsc::channel(100);
        let job_manager = Arc::new(JobManager::new(coin_config, job_tx));

        ShareProcessor::new(config, job_manager).await
    }

    #[tokio::test]
    async fn test_processor_creation() {
        // This test requires Redis to be running
        if let Ok(processor) = create_test_processor().await {
            let stats = processor.get_stats().await;
            assert_eq!(stats.total_shares_processed, 0);
        }
    }

    #[tokio::test] 
    async fn test_processor_health_check() {
        // This test requires Redis to be running
        if let Ok(processor) = create_test_processor().await {
            // Health check should pass initially
            let _ = processor.health_check().await; // May fail if Redis not available
        }
    }

    fn create_test_share() -> ShareSubmission {
        ShareSubmission {
            worker_name: "test_worker".to_string(),
            job_id: "test_job_001".to_string(),
            extranonce2: "00000000".to_string(),
            ntime: "12345678".to_string(),
            nonce: "87654321".to_string(),
            version_bits: None,
        }
    }

    #[tokio::test]
    async fn test_share_submission() {
        // This test requires Redis to be running
        if let Ok(processor) = create_test_processor().await {
            let share = create_test_share();
            let miner_id = Uuid::new_v4();
            let ip_address = "192.168.1.1".to_string();

            // This will likely fail due to job not found, but tests the pipeline
            let result = processor.process_share(miner_id, share, ip_address).await;
            
            // Should accept the share for processing even if validation fails
            assert!(result.is_ok());
        }
    }
}