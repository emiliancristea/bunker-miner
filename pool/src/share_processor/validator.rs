// BUNKER POOL - Share Validation Engine
// Critical security component for validating mining shares

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, warn};
use sha2::{Sha256, Digest};
use blake2::{Blake2b512, Blake2s256, Blake2b};
use k12::KangarooTwelve;
use sha2::digest::FixedOutput;

use crate::stratum::protocol::{Algorithm, ShareSubmission};
use crate::stratum::job_manager::ActiveJob;

/// Share validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub is_block: bool,
    pub error: Option<ValidationError>,
    pub difficulty: f64,
    pub hash: Option<String>,
    pub target_met: bool,
}

/// Share validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    JobNotFound,
    DuplicateShare,
    StaleJob,
    InvalidNonce,
    InvalidTime,
    InvalidExtraNonce,
    LowDifficulty,
    HashCalculationError,
    InvalidBlockHeader,
    AlgorithmMismatch,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::JobNotFound => write!(f, "Job not found"),
            ValidationError::DuplicateShare => write!(f, "Duplicate share"),
            ValidationError::StaleJob => write!(f, "Stale job"),
            ValidationError::InvalidNonce => write!(f, "Invalid nonce"),
            ValidationError::InvalidTime => write!(f, "Invalid time"),
            ValidationError::InvalidExtraNonce => write!(f, "Invalid extranonce"),
            ValidationError::LowDifficulty => write!(f, "Share below target difficulty"),
            ValidationError::HashCalculationError => write!(f, "Hash calculation error"),
            ValidationError::InvalidBlockHeader => write!(f, "Invalid block header"),
            ValidationError::AlgorithmMismatch => write!(f, "Algorithm mismatch"),
        }
    }
}

/// Share validator with comprehensive security checks
pub struct ShareValidator {
    algorithm: Algorithm,
    submitted_shares: HashMap<String, SystemTime>,
    max_job_age_seconds: u64,
    min_difficulty: f64,
}

impl ShareValidator {
    pub fn new(algorithm: Algorithm) -> Self {
        Self {
            algorithm,
            submitted_shares: HashMap::new(),
            max_job_age_seconds: 600, // 10 minutes
            min_difficulty: 1.0,
        }
    }

    /// Validate a mining share with comprehensive security checks
    pub async fn validate_share(
        &mut self,
        share: &ShareSubmission,
        job: &ActiveJob,
        miner_difficulty: f64,
    ) -> ValidationResult {
        // 1. Verify job is not stale
        if let Err(error) = self.check_job_freshness(job) {
            return ValidationResult {
                is_valid: false,
                is_block: false,
                error: Some(error),
                difficulty: 0.0,
                hash: None,
                target_met: false,
            };
        }

        // 2. Check for duplicate shares
        if let Err(error) = self.check_duplicate_share(share, job) {
            return ValidationResult {
                is_valid: false,
                is_block: false,
                error: Some(error),
                difficulty: 0.0,
                hash: None,
                target_met: false,
            };
        }

        // 3. Validate input parameters
        if let Err(error) = self.validate_share_parameters(share) {
            return ValidationResult {
                is_valid: false,
                is_block: false,
                error: Some(error),
                difficulty: 0.0,
                hash: None,
                target_met: false,
            };
        }

        // 4. Build and validate block header
        let block_header = match self.build_block_header(share, job) {
            Ok(header) => header,
            Err(error) => {
                return ValidationResult {
                    is_valid: false,
                    is_block: false,
                    error: Some(error),
                    difficulty: 0.0,
                    hash: None,
                    target_met: false,
                };
            }
        };

        // 5. Calculate hash based on algorithm
        let hash = match self.calculate_hash(&block_header) {
            Ok(hash) => hash,
            Err(error) => {
                return ValidationResult {
                    is_valid: false,
                    is_block: false,
                    error: Some(error),
                    difficulty: 0.0,
                    hash: None,
                    target_met: false,
                };
            }
        };

        // 6. Check if hash meets difficulty requirements
        let actual_difficulty = self.calculate_difficulty_from_hash(&hash);
        let meets_miner_difficulty = actual_difficulty >= miner_difficulty;
        let meets_block_difficulty = actual_difficulty >= self.get_block_difficulty(&job.block_template.bits);

        // 7. Record successful share to prevent duplicates
        if meets_miner_difficulty {
            self.record_share(share, job);
        }

        ValidationResult {
            is_valid: meets_miner_difficulty,
            is_block: meets_block_difficulty,
            error: if meets_miner_difficulty { None } else { Some(ValidationError::LowDifficulty) },
            difficulty: actual_difficulty,
            hash: Some(hex::encode(&hash)),
            target_met: meets_miner_difficulty,
        }
    }

    /// Check if job is still fresh (not stale)
    fn check_job_freshness(&self, job: &ActiveJob) -> Result<(), ValidationError> {
        let job_age = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ValidationError::StaleJob)?
            .as_secs() - job.created_at.timestamp() as u64;

        if job_age > self.max_job_age_seconds {
            warn!("Rejecting stale job: {} seconds old", job_age);
            return Err(ValidationError::StaleJob);
        }

        Ok(())
    }

    /// Check for duplicate share submissions
    fn check_duplicate_share(&self, share: &ShareSubmission, job: &ActiveJob) -> Result<(), ValidationError> {
        let share_key = format!("{}:{}:{}:{}", 
            job.job.job_id, 
            share.extranonce2, 
            share.ntime, 
            share.nonce
        );

        if self.submitted_shares.contains_key(&share_key) {
            warn!("Duplicate share detected: {}", share_key);
            return Err(ValidationError::DuplicateShare);
        }

        Ok(())
    }

    /// Validate share parameters
    fn validate_share_parameters(&self, share: &ShareSubmission) -> Result<(), ValidationError> {
        // Validate extranonce2 (should be hex)
        if hex::decode(&share.extranonce2).is_err() {
            return Err(ValidationError::InvalidExtraNonce);
        }

        // Validate nonce (should be hex)
        if hex::decode(&share.nonce).is_err() {
            return Err(ValidationError::InvalidNonce);
        }

        // Validate time (should be hex timestamp)
        if hex::decode(&share.ntime).is_err() {
            return Err(ValidationError::InvalidTime);
        }

        // Additional parameter validation based on algorithm
        match self.algorithm {
            Algorithm::Sha256 => {
                // For SHA256, nonce should be 4 bytes (8 hex chars)
                if share.nonce.len() != 8 {
                    return Err(ValidationError::InvalidNonce);
                }
            }
            Algorithm::Scrypt => {
                // Scrypt has similar requirements
                if share.nonce.len() != 8 {
                    return Err(ValidationError::InvalidNonce);
                }
            }
            _ => {
                // Other algorithms may have different requirements
            }
        }

        Ok(())
    }

    /// Build block header from share and job
    fn build_block_header(&self, share: &ShareSubmission, job: &ActiveJob) -> Result<Vec<u8>, ValidationError> {
        let mut header = Vec::with_capacity(80); // Standard block header size

        // Version (4 bytes)
        let version = u32::from_str_radix(&job.job.block_version, 16)
            .map_err(|_| ValidationError::InvalidBlockHeader)?;
        header.extend_from_slice(&version.to_le_bytes());

        // Previous block hash (32 bytes)
        let prev_hash = hex::decode(&job.job.prev_hash)
            .map_err(|_| ValidationError::InvalidBlockHeader)?;
        if prev_hash.len() != 32 {
            return Err(ValidationError::InvalidBlockHeader);
        }
        // Reverse byte order for little-endian
        let mut prev_hash_le = prev_hash;
        prev_hash_le.reverse();
        header.extend_from_slice(&prev_hash_le);

        // Merkle root (32 bytes) - calculated from coinbase + transactions
        let merkle_root = self.calculate_merkle_root_with_share(share, job)?;
        header.extend_from_slice(&merkle_root);

        // Timestamp (4 bytes)
        let timestamp = u32::from_str_radix(&share.ntime, 16)
            .map_err(|_| ValidationError::InvalidTime)?;
        header.extend_from_slice(&timestamp.to_le_bytes());

        // Bits/difficulty (4 bytes)
        let bits = u32::from_str_radix(&job.job.nbits, 16)
            .map_err(|_| ValidationError::InvalidBlockHeader)?;
        header.extend_from_slice(&bits.to_le_bytes());

        // Nonce (4 bytes)
        let nonce = u32::from_str_radix(&share.nonce, 16)
            .map_err(|_| ValidationError::InvalidNonce)?;
        header.extend_from_slice(&nonce.to_le_bytes());

        if header.len() != 80 {
            error!("Block header wrong size: {} bytes", header.len());
            return Err(ValidationError::InvalidBlockHeader);
        }

        Ok(header)
    }

    /// Calculate merkle root with the submitted share
    fn calculate_merkle_root_with_share(&self, share: &ShareSubmission, job: &ActiveJob) -> Result<[u8; 32], ValidationError> {
        // Build coinbase transaction with extranonces
        let coinbase = self.build_coinbase_with_extranonces(share, job)?;
        
        // Calculate coinbase transaction hash (double SHA256)
        let coinbase_hash = {
            let hash1 = Sha256::digest(&coinbase);
            let hash2 = Sha256::digest(&hash1);
            let mut result = [0u8; 32];
            result.copy_from_slice(&hash2);
            result
        };

        // Calculate merkle root using coinbase hash and merkle branches
        let mut current_hash = coinbase_hash;
        
        for branch in &job.job.merkle_branch {
            let branch_bytes = hex::decode(branch)
                .map_err(|_| ValidationError::HashCalculationError)?;
            
            if branch_bytes.len() != 32 {
                return Err(ValidationError::HashCalculationError);
            }

            // Combine current hash with branch
            let mut combined = Vec::with_capacity(64);
            combined.extend_from_slice(&current_hash);
            combined.extend_from_slice(&branch_bytes);

            // Double SHA256
            let hash1 = Sha256::digest(&combined);
            let hash2 = Sha256::digest(&hash1);
            current_hash.copy_from_slice(&hash2);
        }

        Ok(current_hash)
    }

    /// Build coinbase transaction with extranonces
    fn build_coinbase_with_extranonces(&self, share: &ShareSubmission, job: &ActiveJob) -> Result<Vec<u8>, ValidationError> {
        let mut coinbase = Vec::new();

        // Coinbase1 (hex encoded)
        let coinbase1 = hex::decode(&job.job.coinbase1)
            .map_err(|_| ValidationError::InvalidBlockHeader)?;
        coinbase.extend_from_slice(&coinbase1);

        // Add extranonce1 (from job subscription) - we would get this from the miner session
        // For now, use a placeholder
        let extranonce1 = "01234567"; // This should come from miner session
        let extranonce1_bytes = hex::decode(extranonce1)
            .map_err(|_| ValidationError::InvalidExtraNonce)?;
        coinbase.extend_from_slice(&extranonce1_bytes);

        // Add extranonce2 (from share)
        let extranonce2_bytes = hex::decode(&share.extranonce2)
            .map_err(|_| ValidationError::InvalidExtraNonce)?;
        coinbase.extend_from_slice(&extranonce2_bytes);

        // Coinbase2 (hex encoded)
        let coinbase2 = hex::decode(&job.job.coinbase2)
            .map_err(|_| ValidationError::InvalidBlockHeader)?;
        coinbase.extend_from_slice(&coinbase2);

        Ok(coinbase)
    }

    /// Calculate hash based on algorithm
    fn calculate_hash(&self, block_header: &[u8]) -> Result<Vec<u8>, ValidationError> {
        match self.algorithm {
            Algorithm::Sha256 => {
                // Double SHA256
                let hash1 = Sha256::digest(block_header);
                let hash2 = Sha256::digest(&hash1);
                Ok(hash2.to_vec())
            }
            Algorithm::Scrypt => {
                // Scrypt requires additional parameters (N, r, p)
                // This is a simplified implementation
                let hash = Sha256::digest(block_header);
                Ok(hash.to_vec())
            }
            Algorithm::Blake2b => {
                let hash = Blake2b512::digest(block_header);
                Ok(hash[..32].to_vec()) // Take first 32 bytes
            }
            Algorithm::Keccak => {
                // Keccak-256 (simplified)
                let hash = Sha256::digest(block_header); // Using SHA256 as placeholder
                Ok(hash.to_vec())
            }
            Algorithm::KawPow => {
                // KawPow is complex and requires epoch data
                // This is a placeholder implementation
                let hash = Sha256::digest(block_header);
                Ok(hash.to_vec())
            }
            Algorithm::Khs => {
                // Karlsen hash (KHeavyHash)
                // KHS/KangarooTwelve - simplified for now
                let mut hasher = KangarooTwelve::new();
                use sha2::digest::Update;
                hasher.update(block_header);
                let hash = hasher.finalize();
                Ok(hash.to_vec())
            }
            Algorithm::X11 => {
                // X11 requires 11 different hash functions chained
                // This is a simplified implementation using Blake2s
                let hash = Blake2s256::digest(block_header);
                Ok(hash.to_vec())
            }
        }
    }

    /// Calculate difficulty from hash
    fn calculate_difficulty_from_hash(&self, hash: &[u8]) -> f64 {
        if hash.len() < 32 {
            return 0.0;
        }

        // Convert hash to big integer (reverse for little-endian)
        let mut hash_be = hash[..32].to_vec();
        hash_be.reverse();

        // Calculate difficulty as max_target / hash_target
        // This is a simplified calculation
        let mut difficulty = 1.0;

        // Count leading zeros
        let mut leading_zeros = 0;
        for byte in &hash_be {
            if *byte == 0 {
                leading_zeros += 8;
            } else {
                leading_zeros += byte.leading_zeros() as usize;
                break;
            }
        }

        // Approximate difficulty based on leading zeros
        if leading_zeros > 0 {
            difficulty = 2.0_f64.powi(leading_zeros as i32);
        }

        difficulty.max(1.0)
    }

    /// Get block difficulty from bits
    fn get_block_difficulty(&self, bits: &str) -> f64 {
        // Convert bits (compact target format) to difficulty
        // This is a simplified implementation
        match u32::from_str_radix(bits, 16) {
            Ok(bits_value) => {
                if bits_value == 0 {
                    return f64::MAX;
                }
                
                // Extract exponent and mantissa
                let exponent = (bits_value >> 24) & 0xff;
                let mantissa = bits_value & 0xffffff;

                if mantissa == 0 {
                    return f64::MAX;
                }

                // Calculate target
                let target = mantissa as f64 * 256.0_f64.powi((exponent as i32) - 3);
                
                // Max target (difficulty 1)
                let max_target = 0x00000000FFFF0000000000000000000000000000000000000000000000000000_f64;
                
                max_target / target
            }
            Err(_) => 1.0,
        }
    }

    /// Record successful share to prevent duplicates
    fn record_share(&mut self, share: &ShareSubmission, job: &ActiveJob) {
        let share_key = format!("{}:{}:{}:{}", 
            job.job.job_id, 
            share.extranonce2, 
            share.ntime, 
            share.nonce
        );

        self.submitted_shares.insert(share_key, SystemTime::now());

        // Clean up old entries periodically
        if self.submitted_shares.len() > 10000 {
            self.cleanup_old_shares();
        }
    }

    /// Clean up old share records
    fn cleanup_old_shares(&mut self) {
        let cutoff = SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(self.max_job_age_seconds))
            .unwrap_or(UNIX_EPOCH);

        self.submitted_shares.retain(|_, &mut timestamp| timestamp > cutoff);
        debug!("Cleaned up old share records, {} remaining", self.submitted_shares.len());
    }

    /// Set minimum difficulty
    pub fn set_min_difficulty(&mut self, difficulty: f64) {
        self.min_difficulty = difficulty;
    }

    /// Set maximum job age
    pub fn set_max_job_age(&mut self, seconds: u64) {
        self.max_job_age_seconds = seconds;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stratum::protocol::MiningJob;
    use crate::stratum::job_manager::{ActiveJob, BlockTemplate};
    use chrono::Utc;

    fn create_test_job() -> ActiveJob {
        let mining_job = MiningJob {
            job_id: "test_job_001".to_string(),
            prev_hash: "0".repeat(64),
            coinbase1: "01000000".to_string(),
            coinbase2: "ffffffff".to_string(),
            merkle_branch: vec![],
            block_version: "20000000".to_string(),
            nbits: "1d00ffff".to_string(),
            ntime: format!("{:08x}", Utc::now().timestamp()),
            clean_jobs: true,
        };

        let block_template = BlockTemplate {
            version: 0x20000000,
            previousblockhash: "0".repeat(64),
            transactions: vec![],
            coinbaseaux: None,
            coinbasevalue: 5000000000, // 50 BTC in satoshis
            longpollid: None,
            target: "0".repeat(64),
            mintime: None,
            mutable: None,
            noncerange: None,
            sigoplimit: None,
            sizelimit: None,
            weightlimit: None,
            curtime: Utc::now().timestamp() as u64,
            bits: "1d00ffff".to_string(),
            height: 100000,
            default_witness_commitment: None,
        };

        ActiveJob {
            job: mining_job,
            created_at: Utc::now(),
            block_template,
            coinbase_tx: "test_coinbase".to_string(),
            merkle_root: "test_merkle".to_string(),
        }
    }

    fn create_test_share() -> ShareSubmission {
        ShareSubmission {
            worker_name: "test_worker".to_string(),
            job_id: "test_job_001".to_string(),
            extranonce2: "00000000".to_string(),
            ntime: format!("{:08x}", Utc::now().timestamp()),
            nonce: "12345678".to_string(),
            version_bits: None,
        }
    }

    #[tokio::test]
    async fn test_validator_creation() {
        let validator = ShareValidator::new(Algorithm::Sha256);
        assert_eq!(validator.algorithm, Algorithm::Sha256);
        assert_eq!(validator.submitted_shares.len(), 0);
    }

    #[tokio::test]
    async fn test_share_parameter_validation() {
        let mut validator = ShareValidator::new(Algorithm::Sha256);
        let share = create_test_share();
        
        // Valid parameters should pass
        assert!(validator.validate_share_parameters(&share).is_ok());
        
        // Invalid extranonce2 should fail
        let mut invalid_share = share.clone();
        invalid_share.extranonce2 = "invalid_hex".to_string();
        assert_eq!(
            validator.validate_share_parameters(&invalid_share).unwrap_err(),
            ValidationError::InvalidExtraNonce
        );
    }

    #[tokio::test]
    async fn test_duplicate_share_detection() {
        let mut validator = ShareValidator::new(Algorithm::Sha256);
        let job = create_test_job();
        let share = create_test_share();
        
        // First submission should be OK
        assert!(validator.check_duplicate_share(&share, &job).is_ok());
        
        // Record the share
        validator.record_share(&share, &job);
        
        // Second submission should be rejected
        assert_eq!(
            validator.check_duplicate_share(&share, &job).unwrap_err(),
            ValidationError::DuplicateShare
        );
    }
}