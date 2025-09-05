// BUNKER POOL - Job Manager
// Manages mining jobs by polling coin daemon RPC and distributing work to miners

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use super::protocol::{Algorithm, MiningJob};

// Cryptography imports
use sha2::Digest;

/// Coin daemon configuration
#[derive(Debug, Clone)]
pub struct CoinDaemonConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    pub algorithm: Algorithm,
    pub block_poll_interval: Duration,
    pub timeout: Duration,
    pub coinbase_address: String,
    pub extra_data: Option<String>,
}

/// Block template from coin daemon
#[derive(Debug, Clone, Deserialize)]
pub struct BlockTemplate {
    pub version: u32,
    pub previousblockhash: String,
    pub transactions: Vec<Transaction>,
    pub coinbaseaux: Option<HashMap<String, String>>,
    pub coinbasevalue: u64,
    pub longpollid: Option<String>,
    pub target: String,
    pub mintime: Option<u64>,
    pub mutable: Option<Vec<String>>,
    pub noncerange: Option<String>,
    pub sigoplimit: Option<u32>,
    pub sizelimit: Option<u32>,
    pub weightlimit: Option<u32>,
    pub curtime: u64,
    pub bits: String,
    pub height: u64,
    pub default_witness_commitment: Option<String>,
}

/// Transaction in block template
#[derive(Debug, Clone, Deserialize)]
pub struct Transaction {
    pub data: String,
    pub txid: Option<String>,
    pub hash: Option<String>,
    pub depends: Option<Vec<u32>>,
    pub fee: Option<u64>,
    pub sigops: Option<u32>,
    pub weight: Option<u32>,
}

/// RPC request to coin daemon
#[derive(Debug, Serialize)]
struct RpcRequest {
    method: String,
    params: serde_json::Value,
    id: u64,
}

/// RPC response from coin daemon
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
    id: u64,
}

/// RPC error
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

/// Active mining job
#[derive(Debug, Clone)]
pub struct ActiveJob {
    pub job: MiningJob,
    pub created_at: DateTime<Utc>,
    pub block_template: BlockTemplate,
    pub coinbase_tx: String,
    pub merkle_root: String,
}

/// Job manager for distributing work to miners
pub struct JobManager {
    config: CoinDaemonConfig,
    client: Client,
    current_job: Arc<RwLock<Option<ActiveJob>>>,
    job_history: Arc<RwLock<HashMap<String, ActiveJob>>>,
    job_broadcaster: mpsc::Sender<MiningJob>,
    difficulty_target: Arc<RwLock<f64>>,
    next_job_id: Arc<RwLock<u64>>,
}

impl JobManager {
    pub fn new(
        config: CoinDaemonConfig,
        job_broadcaster: mpsc::Sender<MiningJob>,
    ) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            current_job: Arc::new(RwLock::new(None)),
            job_history: Arc::new(RwLock::new(HashMap::new())),
            job_broadcaster,
            difficulty_target: Arc::new(RwLock::new(1.0)),
            next_job_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Start the job manager
    pub async fn start(&self) -> Result<(), anyhow::Error> {
        info!("Starting job manager for algorithm: {:?}", self.config.algorithm);

        // Initial job fetch
        if let Err(e) = self.fetch_and_broadcast_job(true).await {
            error!("Failed to fetch initial job: {}", e);
            return Err(e);
        }

        // Start polling loop
        let mut interval = interval(self.config.block_poll_interval);
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.fetch_and_broadcast_job(false).await {
                        error!("Failed to fetch job: {}", e);
                        // Continue running even if one fetch fails
                    }
                }
            }
        }
    }

    /// Fetch new job from coin daemon and broadcast if changed
    async fn fetch_and_broadcast_job(&self, force: bool) -> Result<(), anyhow::Error> {
        let block_template = self.get_block_template().await?;
        
        // Check if this is a new block
        let is_new_block = {
            let current_job = self.current_job.read().await;
            match current_job.as_ref() {
                Some(job) => job.block_template.previousblockhash != block_template.previousblockhash 
                           || job.block_template.height != block_template.height,
                None => true,
            }
        };

        if !is_new_block && !force {
            debug!("No new block, skipping job update");
            return Ok(());
        }

        // Create new mining job
        let mining_job = self.create_mining_job(&block_template).await?;
        let active_job = ActiveJob {
            job: mining_job.clone(),
            created_at: Utc::now(),
            block_template: block_template.clone(),
            coinbase_tx: self.build_coinbase_transaction(&block_template),
            merkle_root: self.calculate_merkle_root(&block_template),
        };

        // Update current job
        {
            let mut current_job = self.current_job.write().await;
            *current_job = Some(active_job.clone());
        }

        // Add to job history
        {
            let mut job_history = self.job_history.write().await;
            job_history.insert(mining_job.job_id.clone(), active_job);
            
            // Clean up old jobs (keep last 10)
            if job_history.len() > 10 {
                let oldest_jobs: Vec<String> = job_history
                    .iter()
                    .map(|(id, job)| (id.clone(), job.created_at))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .sorted_by_key(|(_, created_at)| *created_at)
                    .take(job_history.len() - 10)
                    .map(|(id, _)| id)
                    .collect();
                    
                for job_id in oldest_jobs {
                    job_history.remove(&job_id);
                }
            }
        }

        info!("New mining job created: {} (height: {})", 
              mining_job.job_id, block_template.height);

        // Broadcast job to all miners
        if let Err(e) = self.job_broadcaster.send(mining_job).await {
            error!("Failed to broadcast job: {}", e);
        }

        Ok(())
    }

    /// Get block template from coin daemon
    async fn get_block_template(&self) -> Result<BlockTemplate, anyhow::Error> {
        let request = RpcRequest {
            method: "getblocktemplate".to_string(),
            params: serde_json::json!([{
                "mode": "template",
                "capabilities": ["coinbasetxn", "workid", "coinbase/append"],
                "rules": ["segwit"]
            }]),
            id: 1,
        };

        let response = timeout(
            self.config.timeout,
            self.client
                .post(&self.config.url)
                .basic_auth(&self.config.username, Some(&self.config.password))
                .json(&request)
                .send()
        ).await??;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }

        let rpc_response: RpcResponse<BlockTemplate> = response.json().await?;

        match rpc_response.result {
            Some(template) => Ok(template),
            None => {
                let error_msg = rpc_response.error
                    .map(|e| format!("RPC error {}: {}", e.code, e.message))
                    .unwrap_or_else(|| "Unknown RPC error".to_string());
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    /// Create mining job from block template
    async fn create_mining_job(&self, template: &BlockTemplate) -> Result<MiningJob, anyhow::Error> {
        let job_id = {
            let mut next_id = self.next_job_id.write().await;
            let id = *next_id;
            *next_id += 1;
            format!("{:08x}", id)
        };

        // Build coinbase transaction
        let (coinbase1, coinbase2) = self.build_coinbase_split(&template);

        // Calculate merkle branches
        let merkle_branch = self.calculate_merkle_branches(&template);

        let job = MiningJob {
            job_id,
            prev_hash: template.previousblockhash.clone(),
            coinbase1,
            coinbase2,
            merkle_branch,
            block_version: format!("{:08x}", template.version),
            nbits: template.bits.clone(),
            ntime: format!("{:08x}", template.curtime),
            clean_jobs: true, // Always clean jobs for new blocks
        };

        Ok(job)
    }

    /// Build coinbase transaction split for Stratum
    fn build_coinbase_split(&self, template: &BlockTemplate) -> (String, String) {
        // This is a simplified coinbase construction
        // In production, this would be more sophisticated
        
        let mut coinbase = Vec::new();
        
        // Version (4 bytes)
        coinbase.extend_from_slice(&1u32.to_le_bytes());
        
        // Input count (1 byte)
        coinbase.push(1);
        
        // Previous output hash (32 bytes of zeros)
        coinbase.extend_from_slice(&[0u8; 32]);
        
        // Previous output index (4 bytes of 0xffffffff)
        coinbase.extend_from_slice(&[0xff; 4]);
        
        // Script length (will be filled in)
        let script_sig_start = coinbase.len();
        coinbase.push(0); // Placeholder for script length
        
        // Block height (BIP 34)
        let height_bytes = self.encode_block_height(template.height);
        coinbase.extend_from_slice(&height_bytes);
        
        // Extranonce1 placeholder (4 bytes)
        let extranonce1_pos = coinbase.len();
        coinbase.extend_from_slice(b"EN1P"); // Placeholder for extranonce1
        
        // This is where coinbase1 ends and coinbase2 begins
        let split_pos = coinbase.len();
        
        // Extranonce2 placeholder (4 bytes)
        coinbase.extend_from_slice(b"EN2P"); // Placeholder for extranonce2
        
        // Extra data
        if let Some(extra_data) = &self.config.extra_data {
            coinbase.extend_from_slice(extra_data.as_bytes());
        }
        
        // Update script length
        let script_len = coinbase.len() - script_sig_start - 1;
        coinbase[script_sig_start] = script_len as u8;
        
        // Sequence number (4 bytes)
        coinbase.extend_from_slice(&[0xff; 4]);
        
        // Output count (1 byte)
        coinbase.push(1);
        
        // Output value (8 bytes)
        coinbase.extend_from_slice(&template.coinbasevalue.to_le_bytes());
        
        // Output script length and script (P2PKH to coinbase address)
        let output_script = self.build_coinbase_output_script();
        coinbase.push(output_script.len() as u8);
        coinbase.extend_from_slice(&output_script);
        
        // Lock time (4 bytes)
        coinbase.extend_from_slice(&0u32.to_le_bytes());
        
        // Convert to hex strings
        let coinbase1 = hex::encode(&coinbase[..split_pos]);
        let coinbase2 = hex::encode(&coinbase[split_pos..]);
        
        (coinbase1, coinbase2)
    }

    /// Encode block height for coinbase (BIP 34)
    fn encode_block_height(&self, height: u64) -> Vec<u8> {
        if height == 0 {
            return vec![0];
        }
        
        let mut bytes = Vec::new();
        let mut h = height;
        
        while h > 0 {
            bytes.push((h & 0xff) as u8);
            h >>= 8;
        }
        
        // Add length prefix
        let mut result = vec![bytes.len() as u8];
        result.extend(bytes);
        result
    }

    /// Build coinbase output script (simplified)
    fn build_coinbase_output_script(&self) -> Vec<u8> {
        // For simplicity, we'll use a fixed P2PKH script
        // In production, decode the actual coinbase address
        vec![
            0x76, // OP_DUP
            0xa9, // OP_HASH160
            0x14, // Push 20 bytes
            // 20-byte hash160 of public key (placeholder)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x88, // OP_EQUALVERIFY
            0xac, // OP_CHECKSIG
        ]
    }

    /// Calculate merkle branches for Stratum
    fn calculate_merkle_branches(&self, template: &BlockTemplate) -> Vec<String> {
        let mut txids = Vec::new();
        
        // Add coinbase transaction (empty hash as placeholder)
        txids.push([0u8; 32]);
        
        // Add all transaction IDs
        for tx in &template.transactions {
            if let Some(txid) = &tx.txid {
                if let Ok(bytes) = hex::decode(txid) {
                    if bytes.len() == 32 {
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(&bytes);
                        txids.push(hash);
                    }
                }
            }
        }
        
        // Calculate merkle tree branches
        let mut branches = Vec::new();
        let mut level = txids;
        
        while level.len() > 1 {
            // For coinbase (index 0), we need the sibling at each level
            if level.len() > 1 {
                branches.push(hex::encode(&level[1]));
            }
            
            // Calculate next level
            let mut next_level = Vec::new();
            for i in (0..level.len()).step_by(2) {
                let left = &level[i];
                let right = if i + 1 < level.len() {
                    &level[i + 1]
                } else {
                    &level[i] // Duplicate last hash if odd number
                };
                
                // Double SHA256
                let combined = [left.as_slice(), right.as_slice()].concat();
                let hash1 = sha2::Sha256::digest(&combined);
                let hash2 = sha2::Sha256::digest(&hash1);
                
                let mut result = [0u8; 32];
                result.copy_from_slice(&hash2);
                next_level.push(result);
            }
            level = next_level;
        }
        
        branches
    }

    /// Calculate merkle root from transactions
    fn calculate_merkle_root(&self, template: &BlockTemplate) -> String {
        // This is a simplified merkle root calculation
        // In production, use proper merkle tree implementation
        if template.transactions.is_empty() {
            return "0".repeat(64);
        }
        
        format!("{:064x}", template.transactions.len())
    }

    /// Build coinbase transaction (full transaction)
    fn build_coinbase_transaction(&self, template: &BlockTemplate) -> String {
        // This would build the complete coinbase transaction
        // For now, return a placeholder
        format!("coinbase_tx_{}", template.height)
    }

    /// Get job by ID for share validation
    pub async fn get_job(&self, job_id: &str) -> Option<ActiveJob> {
        let job_history = self.job_history.read().await;
        job_history.get(job_id).cloned()
    }

    /// Get current difficulty target
    pub async fn get_difficulty_target(&self) -> f64 {
        *self.difficulty_target.read().await
    }
}

// Helper trait for sorting
trait IteratorExt<T> {
    fn sorted_by_key<K, F>(self, f: F) -> std::vec::IntoIter<T>
    where
        F: FnMut(&T) -> K,
        K: Ord;
}

impl<I, T> IteratorExt<T> for I
where
    I: Iterator<Item = T>,
{
    fn sorted_by_key<K, F>(self, f: F) -> std::vec::IntoIter<T>
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        let mut items: Vec<T> = self.collect();
        items.sort_by_key(f);
        items.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encode_block_height() {
        let config = CoinDaemonConfig {
            url: "http://localhost".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            algorithm: Algorithm::Sha256,
            block_poll_interval: Duration::from_secs(10),
            timeout: Duration::from_secs(30),
            coinbase_address: "test".to_string(),
            extra_data: None,
        };
        
        let (tx, _rx) = mpsc::channel(1);
        let job_manager = JobManager::new(config, tx);
        
        assert_eq!(job_manager.encode_block_height(0), vec![0]);
        assert_eq!(job_manager.encode_block_height(255), vec![1, 255]);
        assert_eq!(job_manager.encode_block_height(256), vec![2, 0, 1]);
    }
}