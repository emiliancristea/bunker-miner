/*!
 * BUNKER MINER - Coin Daemon Smart Stub
 *
 * Development stub service that implements a cryptocurrency daemon JSON-RPC API
 * with dummy block templates and mining data for local development and testing.
 *
 * Features:
 * - JSON-RPC 2.0 compatible API for mining operations
 * - Block template generation with realistic structure
 * - Transaction pool simulation
 * - Network information endpoints
 * - Health checks and metrics
 */

use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use chrono::{DateTime, Utc};
use clap::Parser;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, info, warn};

#[derive(Parser)]
#[command(name = "coin-daemon-stub")]
#[command(about = "BUNKER MINER Coin Daemon Smart Stub")]
struct Args {
    /// Bind address
    #[arg(long, default_value = "0.0.0.0:18081")]
    bind: String,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Health check mode
    #[arg(long)]
    health_check: bool,

    /// Simulate network difficulty
    #[arg(long, default_value = "300000000000")]
    difficulty: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTemplate {
    pub blocktemplate_blob: String,
    pub blockhashing_blob: String,
    pub difficulty: u64,
    pub expected_reward: u64,
    pub height: u64,
    pub prev_hash: String,
    pub reserved_offset: u32,
    pub seed_hash: String,
    pub next_seed_hash: String,
    pub status: String,
    pub untrusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub status: String,
    pub height: u64,
    pub target_height: u64,
    pub difficulty: u64,
    pub target: u64,
    pub tx_count: u32,
    pub tx_pool_size: u32,
    pub alt_blocks_count: u32,
    pub outgoing_connections_count: u32,
    pub incoming_connections_count: u32,
    pub rpc_connections_count: u32,
    pub white_peerlist_size: u32,
    pub grey_peerlist_size: u32,
    pub mainnet: bool,
    pub testnet: bool,
    pub stagenet: bool,
    pub nettype: String,
    pub top_block_hash: String,
    pub cumulative_difficulty: u64,
    pub cumulative_difficulty_top64: u64,
    pub block_size_limit: u32,
    pub block_size_median: u32,
    pub block_weight_limit: u32,
    pub block_weight_median: u32,
    pub start_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitBlockResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub blob_size: u32,
    pub weight: u32,
    pub fee: u64,
}

type AppState = Arc<RwLock<StubState>>;

#[derive(Debug)]
struct StubState {
    height: u64,
    difficulty: u64,
    tx_pool: Vec<Transaction>,
    blocks_submitted: u64,
    rpc_requests: u64,
    start_time: DateTime<Utc>,
    rng: StdRng,
}

impl StubState {
    fn new(difficulty: u64) -> Self {
        let mut rng = StdRng::from_entropy();
        let mut tx_pool = Vec::new();

        // Generate some mock transactions
        for i in 0..5 {
            tx_pool.push(Transaction {
                id: format!("{:064x}", rng.gen::<u64>()),
                blob_size: 1000 + (i * 200),
                weight: 1500 + (i * 300),
                fee: 10000 + (i as u64 * 5000),
            });
        }

        Self {
            height: 2800000,
            difficulty,
            tx_pool,
            blocks_submitted: 0,
            rpc_requests: 0,
            start_time: Utc::now(),
            rng,
        }
    }

    fn generate_hash(&mut self) -> String {
        let random_bytes: [u8; 32] = self.rng.gen();
        hex::encode(random_bytes)
    }

    fn generate_block_template(&mut self) -> BlockTemplate {
        let prev_hash = self.generate_hash();
        let seed_hash = self.generate_hash();
        let next_seed_hash = self.generate_hash();

        // Generate block template blob (simplified)
        let template_data = format!(
            "{}{}{}{}",
            self.height, prev_hash, seed_hash, self.difficulty
        );

        let mut hasher = Sha256::new();
        hasher.update(template_data.as_bytes());
        let template_hash = hex::encode(hasher.finalize());

        BlockTemplate {
            blocktemplate_blob: format!("0x{}", template_hash),
            blockhashing_blob: format!("0x{}", &template_hash[..64]),
            difficulty: self.difficulty,
            expected_reward: 600000000000, // 0.6 XMR in atomic units
            height: self.height,
            prev_hash,
            reserved_offset: 47,
            seed_hash,
            next_seed_hash,
            status: "OK".to_string(),
            untrusted: false,
        }
    }

    fn get_network_info(&self) -> NetworkInfo {
        NetworkInfo {
            status: "OK".to_string(),
            height: self.height,
            target_height: self.height + 2,
            difficulty: self.difficulty,
            target: 120, // 2-minute block target
            tx_count: self.tx_pool.len() as u32,
            tx_pool_size: self.tx_pool.len() as u32,
            alt_blocks_count: 0,
            outgoing_connections_count: 8,
            incoming_connections_count: 12,
            rpc_connections_count: 3,
            white_peerlist_size: 150,
            grey_peerlist_size: 300,
            mainnet: false,
            testnet: true,
            stagenet: false,
            nettype: "testnet".to_string(),
            top_block_hash: format!("{:064x}", rand::thread_rng().gen::<u64>()),
            cumulative_difficulty: self.difficulty * self.height,
            cumulative_difficulty_top64: 0,
            block_size_limit: 600000,
            block_size_median: 300000,
            block_weight_limit: 600000,
            block_weight_median: 300000,
            start_time: self.start_time.timestamp() as u64,
        }
    }
}

async fn handle_jsonrpc(
    State(state): State<AppState>,
    Json(request): Json<JsonRpcRequest>,
) -> Result<Json<JsonRpcResponse>, StatusCode> {
    debug!(
        "📨 JSON-RPC Request: {} - {}",
        request.method,
        serde_json::to_string(&request.params).unwrap_or_default()
    );

    let mut state_lock = state.write().await;
    state_lock.rpc_requests += 1;

    let response = match request.method.as_str() {
        "get_block_template" => {
            info!(
                "🔨 Generating block template for height {}",
                state_lock.height
            );
            let template = state_lock.generate_block_template();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::to_value(template).unwrap()),
                error: None,
            }
        }

        "submit_block" => {
            let block_blob = request
                .params
                .as_ref()
                .and_then(|p| p.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            info!(
                "✅ Block submitted: {}",
                &block_blob[..std::cmp::min(16, block_blob.len())]
            );

            state_lock.blocks_submitted += 1;
            state_lock.height += 1;

            // Simulate difficulty adjustment
            if state_lock.height % 10 == 0 {
                let adjustment = 1.0 + (rand::thread_rng().gen::<f64>() - 0.5) * 0.1;
                state_lock.difficulty = (state_lock.difficulty as f64 * adjustment) as u64;
                info!("⚖️ Difficulty adjusted to: {}", state_lock.difficulty);
            }

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(
                    serde_json::to_value(SubmitBlockResponse {
                        status: "OK".to_string(),
                    })
                    .unwrap(),
                ),
                error: None,
            }
        }

        "get_info" => {
            info!("ℹ️ Network info requested");
            let network_info = state_lock.get_network_info();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::to_value(network_info).unwrap()),
                error: None,
            }
        }

        "get_height" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::json!({
                "height": state_lock.height,
                "status": "OK"
            })),
            error: None,
        },

        "get_last_block_header" => {
            let block_hash = state_lock.generate_hash();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::json!({
                    "block_header": {
                        "block_size": 300000,
                        "block_weight": 300000,
                        "cumulative_difficulty": state_lock.difficulty * state_lock.height,
                        "cumulative_difficulty_top64": 0,
                        "depth": 0,
                        "difficulty": state_lock.difficulty,
                        "hash": block_hash,
                        "height": state_lock.height - 1,
                        "major_version": 14,
                        "minor_version": 14,
                        "nonce": rand::thread_rng().gen::<u32>(),
                        "num_txes": state_lock.tx_pool.len(),
                        "orphan_status": false,
                        "prev_hash": state_lock.generate_hash(),
                        "reward": 600000000000u64,
                        "timestamp": Utc::now().timestamp() as u64
                    },
                    "status": "OK",
                    "untrusted": false
                })),
                error: None,
            }
        }

        "get_connections" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::json!({
                "connections": [
                    {
                        "address": "127.0.0.1:18080",
                        "avg_download": 5120,
                        "avg_upload": 1024,
                        "connection_id": "abc123",
                        "current_download": 1024,
                        "current_upload": 512,
                        "height": state_lock.height,
                        "host": "127.0.0.1",
                        "incoming": false,
                        "ip": "127.0.0.1",
                        "live_time": 3600,
                        "local_ip": false,
                        "localhost": true,
                        "peer_id": "1234567890abcdef",
                        "port": "18080",
                        "recv_count": 1000,
                        "recv_idle_time": 10,
                        "send_count": 500,
                        "send_idle_time": 5,
                        "state": "normal",
                        "support_flags": 1
                    }
                ],
                "status": "OK"
            })),
            error: None,
        },

        _ => {
            warn!("❓ Unknown JSON-RPC method: {}", request.method);
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            }
        }
    };

    debug!(
        "📤 JSON-RPC Response: {}",
        serde_json::to_string(&response).unwrap_or_default()
    );

    Ok(Json(response))
}

async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let state_lock = state.read().await;
    let now = Utc::now();

    Ok(Json(serde_json::json!({
        "status": "healthy",
        "timestamp": now,
        "version": "0.1.0-stub",
        "uptime_seconds": (now - state_lock.start_time).num_seconds(),
        "height": state_lock.height,
        "difficulty": state_lock.difficulty,
        "tx_pool_size": state_lock.tx_pool.len(),
        "blocks_submitted": state_lock.blocks_submitted,
        "rpc_requests": state_lock.rpc_requests
    })))
}

async fn daemon_info(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let state_lock = state.read().await;

    Ok(Json(serde_json::json!({
        "daemon_info": {
            "name": "BUNKER MINER Coin Daemon Stub",
            "version": "0.1.0-stub",
            "description": "Development stub for cryptocurrency daemon",
            "supported_methods": [
                "get_block_template",
                "submit_block",
                "get_info",
                "get_height",
                "get_last_block_header",
                "get_connections"
            ]
        },
        "network": {
            "height": state_lock.height,
            "difficulty": state_lock.difficulty,
            "testnet": true
        },
        "endpoints": {
            "jsonrpc": "/json_rpc",
            "health": "/health",
            "info": "/info"
        }
    })))
}

fn create_app(state: AppState) -> Router {
    Router::new()
        // JSON-RPC endpoint (main daemon API)
        .route("/json_rpc", post(handle_jsonrpc))
        // Health and info endpoints
        .route("/health", axum::routing::get(health_check))
        .route("/info", axum::routing::get(daemon_info))
        // Root endpoint with daemon info
        .route("/", axum::routing::get(daemon_info))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Handle health check mode
    if args.health_check {
        std::process::exit(0);
    }

    // Initialize logging
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("coin_daemon_stub={},tower_http=debug", log_level))
        .init();

    info!("💰 Starting BUNKER MINER Coin Daemon Stub");
    info!("Version: 0.1.0-stub");
    info!("Bind address: {}", args.bind);
    info!("Simulated difficulty: {}", args.difficulty);

    // Initialize state
    let state = Arc::new(RwLock::new(StubState::new(args.difficulty)));

    // Log initial state
    {
        let state_lock = state.read().await;
        info!("Mock daemon initialized:");
        info!("  Current height: {}", state_lock.height);
        info!("  Network difficulty: {}", state_lock.difficulty);
        info!("  Transaction pool size: {}", state_lock.tx_pool.len());
    }

    // Create and bind server
    let app = create_app(state);
    let addr: SocketAddr = args.bind.parse()?;

    info!("🚀 Coin Daemon Stub listening on http://{}", addr);
    info!("📋 Available endpoints:");
    info!("  POST /json_rpc              - Main JSON-RPC endpoint");
    info!("  GET  /health                 - Health check");
    info!("  GET  /info                   - Daemon information");
    info!("  GET  /                       - Root info page");
    info!("");
    info!("📡 Supported JSON-RPC methods:");
    info!("  get_block_template           - Get mining block template");
    info!("  submit_block                 - Submit mined block");
    info!("  get_info                     - Get network information");
    info!("  get_height                   - Get current blockchain height");
    info!("  get_last_block_header        - Get latest block header");
    info!("  get_connections              - Get peer connections");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_state_initialization() {
        let state = StubState::new(300000000000);

        assert_eq!(state.height, 2800000);
        assert_eq!(state.difficulty, 300000000000);
        assert!(!state.tx_pool.is_empty());
        assert_eq!(state.blocks_submitted, 0);
        assert_eq!(state.rpc_requests, 0);
    }

    #[test]
    fn test_block_template_generation() {
        let mut state = StubState::new(300000000000);
        let template = state.generate_block_template();

        assert_eq!(template.height, 2800000);
        assert_eq!(template.difficulty, 300000000000);
        assert_eq!(template.status, "OK");
        assert!(!template.prev_hash.is_empty());
        assert!(!template.blocktemplate_blob.is_empty());
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = Arc::new(RwLock::new(StubState::new(300000000000)));
        let result = health_check(State(state)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response["status"], "healthy");
        assert_eq!(response["version"], "0.1.0-stub");
    }
}
