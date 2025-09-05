// BUNKER POOL - Stratum Protocol Implementation
// JSON-RPC 2.0 message parsing and Stratum v1 protocol handling

use serde::{Deserialize, Serialize};
// use std::collections::HashMap; // Unused import
use uuid::Uuid;

/// Stratum JSON-RPC Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumRequest {
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<String>,
}

/// Stratum JSON-RPC Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumResponse {
    pub id: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<StratumError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<String>,
}

/// Stratum Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Stratum Notification (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumNotification {
    pub method: String,
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<String>,
}

/// Mining job sent to miners
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningJob {
    pub job_id: String,
    pub prev_hash: String,
    pub coinbase1: String,
    pub coinbase2: String,
    pub merkle_branch: Vec<String>,
    pub block_version: String,
    pub nbits: String,
    pub ntime: String,
    pub clean_jobs: bool,
}

/// Mining subscription details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningSubscription {
    pub subscription_id: String,
    pub extranonce1: String,
    pub extranonce2_size: usize,
}

/// Share submission from miner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareSubmission {
    pub worker_name: String,
    pub job_id: String,
    pub extranonce2: String,
    pub ntime: String,
    pub nonce: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_bits: Option<String>,
}

/// Mining algorithm types supported by BUNKER POOL
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Algorithm {
    #[serde(rename = "sha256")]
    Sha256,
    #[serde(rename = "scrypt")]
    Scrypt,
    #[serde(rename = "x11")]
    X11,
    #[serde(rename = "blake2b")]
    Blake2b,
    #[serde(rename = "keccak")]
    Keccak,
    #[serde(rename = "kawpow")]
    KawPow,
    #[serde(rename = "khs")]
    Khs,
}

impl Algorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Sha256 => "sha256",
            Algorithm::Scrypt => "scrypt",
            Algorithm::X11 => "x11",
            Algorithm::Blake2b => "blake2b",
            Algorithm::Keccak => "keccak",
            Algorithm::KawPow => "kawpow",
            Algorithm::Khs => "khs",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sha256" => Some(Algorithm::Sha256),
            "scrypt" => Some(Algorithm::Scrypt),
            "x11" => Some(Algorithm::X11),
            "blake2b" => Some(Algorithm::Blake2b),
            "keccak" => Some(Algorithm::Keccak),
            "kawpow" => Some(Algorithm::KawPow),
            "khs" => Some(Algorithm::Khs),
            _ => None,
        }
    }
}

/// Stratum method handlers
pub enum StratumMethod {
    Subscribe,
    Authorize,
    Submit,
    Configure,
    SuggestTarget,
    SuggestDifficulty,
    GetTransactions,
    Unknown(String),
}

impl From<&str> for StratumMethod {
    fn from(method: &str) -> Self {
        match method {
            "mining.subscribe" => StratumMethod::Subscribe,
            "mining.authorize" => StratumMethod::Authorize,
            "mining.submit" => StratumMethod::Submit,
            "mining.configure" => StratumMethod::Configure,
            "mining.suggest_target" => StratumMethod::SuggestTarget,
            "mining.suggest_difficulty" => StratumMethod::SuggestDifficulty,
            "mining.get_transactions" => StratumMethod::GetTransactions,
            method => StratumMethod::Unknown(method.to_string()),
        }
    }
}

/// Common Stratum error codes
pub mod error_codes {
    pub const OTHER_UNKNOWN: i32 = 20;
    pub const JOB_NOT_FOUND: i32 = 21;
    pub const DUPLICATE_SHARE: i32 = 22;
    pub const LOW_DIFFICULTY: i32 = 23;
    pub const UNAUTHORIZED_WORKER: i32 = 24;
    pub const NOT_SUBSCRIBED: i32 = 25;
    pub const INVALID_PARAMS: i32 = -1;
    pub const METHOD_NOT_FOUND: i32 = -2;
    pub const INVALID_REQUEST: i32 = -3;
}

impl StratumError {
    pub fn new(code: i32, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
        }
    }
    
    pub fn job_not_found() -> Self {
        Self::new(error_codes::JOB_NOT_FOUND, "Job not found")
    }
    
    pub fn duplicate_share() -> Self {
        Self::new(error_codes::DUPLICATE_SHARE, "Duplicate share")
    }
    
    pub fn low_difficulty() -> Self {
        Self::new(error_codes::LOW_DIFFICULTY, "Share below target")
    }
    
    pub fn unauthorized_worker() -> Self {
        Self::new(error_codes::UNAUTHORIZED_WORKER, "Unauthorized worker")
    }
    
    pub fn not_subscribed() -> Self {
        Self::new(error_codes::NOT_SUBSCRIBED, "Not subscribed")
    }
    
    pub fn invalid_params() -> Self {
        Self::new(error_codes::INVALID_PARAMS, "Invalid parameters")
    }
    
    pub fn method_not_found() -> Self {
        Self::new(error_codes::METHOD_NOT_FOUND, "Method not found")
    }
    
    pub fn invalid_request() -> Self {
        Self::new(error_codes::INVALID_REQUEST, "Invalid request")
    }
}

impl StratumResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
            jsonrpc: Some("2.0".to_string()),
        }
    }
    
    pub fn error(id: Option<serde_json::Value>, error: StratumError) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
            jsonrpc: Some("2.0".to_string()),
        }
    }
}

impl StratumNotification {
    pub fn new(method: &str, params: serde_json::Value) -> Self {
        Self {
            method: method.to_string(),
            params,
            jsonrpc: Some("2.0".to_string()),
        }
    }
    
    /// Create a mining.notify notification for new work
    pub fn mining_notify(job: &MiningJob) -> Self {
        let params = serde_json::json!([
            job.job_id,
            job.prev_hash,
            job.coinbase1,
            job.coinbase2,
            job.merkle_branch,
            job.block_version,
            job.nbits,
            job.ntime,
            job.clean_jobs
        ]);
        
        Self::new("mining.notify", params)
    }
    
    /// Create a mining.set_difficulty notification
    pub fn set_difficulty(difficulty: f64) -> Self {
        let params = serde_json::json!([difficulty]);
        Self::new("mining.set_difficulty", params)
    }
    
    /// Create a mining.set_target notification (for some algorithms)
    pub fn set_target(target: &str) -> Self {
        let params = serde_json::json!([target]);
        Self::new("mining.set_target", params)
    }
}

/// Parse Stratum message from JSON string
pub fn parse_stratum_message(data: &str) -> Result<StratumMessage, serde_json::Error> {
    // Try parsing as request first
    if let Ok(request) = serde_json::from_str::<StratumRequest>(data) {
        return Ok(StratumMessage::Request(request));
    }
    
    // Try parsing as response
    if let Ok(response) = serde_json::from_str::<StratumResponse>(data) {
        return Ok(StratumMessage::Response(response));
    }
    
    // Try parsing as notification
    if let Ok(notification) = serde_json::from_str::<StratumNotification>(data) {
        return Ok(StratumMessage::Notification(notification));
    }
    
    Err(serde_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidData, 
        "Unable to parse as any Stratum message type"
    )))
}

/// Unified Stratum message type
#[derive(Debug, Clone)]
pub enum StratumMessage {
    Request(StratumRequest),
    Response(StratumResponse),
    Notification(StratumNotification),
}

impl StratumMessage {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        match self {
            StratumMessage::Request(req) => serde_json::to_string(req),
            StratumMessage::Response(resp) => serde_json::to_string(resp),
            StratumMessage::Notification(notif) => serde_json::to_string(notif),
        }
    }
}

/// Miner session state
#[derive(Debug, Clone)]
pub struct MinerSession {
    pub id: Uuid,
    pub subscribed: bool,
    pub authorized: bool,
    pub worker_name: Option<String>,
    pub subscription: Option<MiningSubscription>,
    pub difficulty: f64,
    pub algorithm: Algorithm,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub shares_submitted: u64,
    pub shares_accepted: u64,
    pub shares_rejected: u64,
}

impl MinerSession {
    pub fn new(ip_address: String, algorithm: Algorithm) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            subscribed: false,
            authorized: false,
            worker_name: None,
            subscription: None,
            difficulty: 1.0, // Default difficulty
            algorithm,
            ip_address,
            user_agent: None,
            connected_at: now,
            last_activity: now,
            shares_submitted: 0,
            shares_accepted: 0,
            shares_rejected: 0,
        }
    }
    
    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now();
    }
    
    pub fn accept_rate(&self) -> f64 {
        if self.shares_submitted == 0 {
            0.0
        } else {
            self.shares_accepted as f64 / self.shares_submitted as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_mining_subscribe() {
        let json = r#"{"id": 1, "method": "mining.subscribe", "params": ["cpuminer/2.5.0"]}"#;
        let message = parse_stratum_message(json).unwrap();
        
        if let StratumMessage::Request(req) = message {
            assert_eq!(req.method, "mining.subscribe");
            assert_eq!(req.id, Some(serde_json::json!(1)));
        } else {
            panic!("Expected request message");
        }
    }
    
    #[test]
    fn test_algorithm_conversion() {
        assert_eq!(Algorithm::from_str("sha256"), Some(Algorithm::Sha256));
        assert_eq!(Algorithm::from_str("SHA256"), Some(Algorithm::Sha256));
        assert_eq!(Algorithm::from_str("invalid"), None);
        assert_eq!(Algorithm::Sha256.as_str(), "sha256");
    }
    
    #[test]
    fn test_stratum_error() {
        let error = StratumError::job_not_found();
        assert_eq!(error.code, error_codes::JOB_NOT_FOUND);
        assert_eq!(error.message, "Job not found");
    }
}