/*!
 * BUNKER MINER - Pool API Smart Stub
 *
 * Development stub service that implements the future BUNKER POOL API
 * with hardcoded, schema-valid data for local development and testing.
 *
 * Features:
 * - REST API endpoints for pool statistics and miner management
 * - Realistic mock data for development and testing
 * - Health checks and metrics endpoints
 * - Proper error handling and logging
 */

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use sysinfo::SystemExt;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "pool-api-stub")]
#[command(about = "BUNKER MINER Pool API Smart Stub")]
struct Args {
    /// Bind address
    #[arg(long, default_value = "0.0.0.0:8080")]
    bind: String,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,

    /// Health check mode
    #[arg(long)]
    health_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub pool_hashrate_hs: f64,
    pub connected_miners: u32,
    pub active_workers: u32,
    pub blocks_found: u32,
    pub last_block_time: DateTime<Utc>,
    pub difficulty: f64,
    pub fee_percentage: f64,
    pub minimum_payout: f64,
    pub payment_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerStats {
    pub miner_id: String,
    pub worker_name: String,
    pub hashrate_hs: f64,
    pub shares_submitted: u64,
    pub shares_accepted: u64,
    pub last_seen: DateTime<Utc>,
    pub connected_since: DateTime<Utc>,
    pub status: String,
    pub difficulty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutInfo {
    pub address: String,
    pub pending_balance: f64,
    pub total_paid: f64,
    pub last_payment: Option<DateTime<Utc>>,
    pub payment_threshold: f64,
    pub auto_payout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub timestamp: DateTime<Utc>,
    pub difficulty: f64,
    pub reward: f64,
    pub finder: String,
    pub confirmations: u32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub memory_usage_mb: u64,
    pub connected_miners: u32,
}

type AppState = Arc<RwLock<StubState>>;

#[derive(Debug)]
struct StubState {
    pool_stats: PoolStats,
    miners: HashMap<String, MinerStats>,
    payouts: HashMap<String, PayoutInfo>,
    recent_blocks: Vec<BlockInfo>,
    start_time: DateTime<Utc>,
}

impl Default for StubState {
    fn default() -> Self {
        let now = Utc::now();

        // Generate mock miners
        let mut miners = HashMap::new();
        let miner_names = [
            "alice_miner",
            "bob_farm",
            "crypto_enthusiast",
            "mining_rig_01",
        ];

        for (i, name) in miner_names.iter().enumerate() {
            let miner_id = format!("miner_{:04}", i + 1);
            miners.insert(
                miner_id.clone(),
                MinerStats {
                    miner_id: miner_id.clone(),
                    worker_name: name.to_string(),
                    hashrate_hs: 1000.0 + (i as f64 * 500.0),
                    shares_submitted: 1000 + (i as u64 * 250),
                    shares_accepted: 950 + (i as u64 * 240),
                    last_seen: now - chrono::Duration::minutes(i as i64 * 2),
                    connected_since: now - chrono::Duration::hours(i as i64 + 1),
                    status: if i == 0 {
                        "mining"
                    } else if i == 1 {
                        "connected"
                    } else {
                        "idle"
                    }
                    .to_string(),
                    difficulty: 1000.0 * (i as f64 + 1.0),
                },
            );
        }

        // Generate mock payouts
        let mut payouts = HashMap::new();
        let addresses = [
            "48abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12",
            "48fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321fedcba09",
        ];

        for (i, addr) in addresses.iter().enumerate() {
            payouts.insert(
                addr.to_string(),
                PayoutInfo {
                    address: addr.to_string(),
                    pending_balance: 0.5 + (i as f64 * 0.3),
                    total_paid: 10.0 + (i as f64 * 5.0),
                    last_payment: Some(now - chrono::Duration::days(i as i64 + 1)),
                    payment_threshold: 1.0,
                    auto_payout: true,
                },
            );
        }

        // Generate mock recent blocks
        let mut recent_blocks = Vec::new();
        for i in 0..5 {
            recent_blocks.push(BlockInfo {
                height: 2800000 + i,
                hash: format!("{:064x}", i * 123456789),
                timestamp: now - chrono::Duration::hours(i as i64 * 2),
                difficulty: 300000000000.0 + (i as f64 * 1000000000.0),
                reward: 0.6,
                finder: format!("miner_{:04}", (i % 4) + 1),
                confirmations: 60 - (i as u32 * 10),
                status: if i < 2 { "confirmed" } else { "pending" }.to_string(),
            });
        }

        let total_hashrate = miners.values().map(|m| m.hashrate_hs).sum::<f64>();

        Self {
            pool_stats: PoolStats {
                pool_hashrate_hs: total_hashrate,
                connected_miners: miners.len() as u32,
                active_workers: miners.values().filter(|m| m.status == "mining").count() as u32,
                blocks_found: recent_blocks.len() as u32,
                last_block_time: recent_blocks.first().map(|b| b.timestamp).unwrap_or(now),
                difficulty: 300000000000.0,
                fee_percentage: 1.0,
                minimum_payout: 0.1,
                payment_threshold: 1.0,
            },
            miners,
            payouts,
            recent_blocks,
            start_time: now,
        }
    }
}

// API Handlers
async fn get_pool_stats(State(state): State<AppState>) -> Result<Json<PoolStats>, StatusCode> {
    let state = state.read().await;
    Ok(Json(state.pool_stats.clone()))
}

async fn get_miners(State(state): State<AppState>) -> Result<Json<Vec<MinerStats>>, StatusCode> {
    let state = state.read().await;
    let miners: Vec<MinerStats> = state.miners.values().cloned().collect();
    Ok(Json(miners))
}

async fn get_miner(
    Path(miner_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<MinerStats>, StatusCode> {
    let state = state.read().await;
    match state.miners.get(&miner_id) {
        Some(miner) => Ok(Json(miner.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_payouts(State(state): State<AppState>) -> Result<Json<Vec<PayoutInfo>>, StatusCode> {
    let state = state.read().await;
    let payouts: Vec<PayoutInfo> = state.payouts.values().cloned().collect();
    Ok(Json(payouts))
}

async fn get_payout(
    Path(address): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<PayoutInfo>, StatusCode> {
    let state = state.read().await;
    match state.payouts.get(&address) {
        Some(payout) => Ok(Json(payout.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_blocks(State(state): State<AppState>) -> Result<Json<Vec<BlockInfo>>, StatusCode> {
    let state = state.read().await;
    Ok(Json(state.recent_blocks.clone()))
}

async fn health_check(State(state): State<AppState>) -> Result<Json<HealthStatus>, StatusCode> {
    let state = state.read().await;
    let system = sysinfo::System::new_all();

    let uptime = (Utc::now() - state.start_time).num_seconds() as u64;
    let memory_usage = system.used_memory() / 1024 / 1024;

    Ok(Json(HealthStatus {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        version: "0.1.0-stub".to_string(),
        uptime_seconds: uptime,
        memory_usage_mb: memory_usage,
        connected_miners: state.miners.len() as u32,
    }))
}

// Mock endpoint for triggering a "payout"
#[derive(Deserialize)]
struct PayoutRequest {
    address: String,
    amount: f64,
}

async fn trigger_payout(
    State(state): State<AppState>,
    Json(request): Json<PayoutRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut state = state.write().await;

    if let Some(payout) = state.payouts.get_mut(&request.address) {
        if request.amount <= payout.pending_balance {
            payout.pending_balance -= request.amount;
            payout.total_paid += request.amount;
            payout.last_payment = Some(Utc::now());

            info!(
                "Mock payout triggered: {} XMR to {}",
                request.amount, request.address
            );

            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Payout processed",
                "transaction_id": Uuid::new_v4().to_string(),
                "amount": request.amount,
                "address": request.address
            })))
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

fn create_app(state: AppState) -> Router {
    Router::new()
        // Pool statistics
        .route("/api/v1/pool/stats", get(get_pool_stats))
        // Miner management
        .route("/api/v1/miners", get(get_miners))
        .route("/api/v1/miners/:miner_id", get(get_miner))
        // Payout management
        .route("/api/v1/payouts", get(get_payouts))
        .route("/api/v1/payouts/trigger", post(trigger_payout))
        .route("/api/v1/payouts/:address", get(get_payout))
        // Block information
        .route("/api/v1/blocks", get(get_blocks))
        // Health and monitoring
        .route("/health", get(health_check))
        .route("/api/v1/health", get(health_check))
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
        .with_env_filter(format!("pool_api_stub={},tower_http=debug", log_level))
        .init();

    info!("🏊 Starting BUNKER MINER Pool API Stub");
    info!("Version: 0.1.0-stub");
    info!("Bind address: {}", args.bind);

    // Initialize state
    let state = Arc::new(RwLock::new(StubState::default()));

    // Log initial state
    {
        let state_lock = state.read().await;
        info!("Mock pool initialized:");
        info!(
            "  Total hashrate: {:.2} H/s",
            state_lock.pool_stats.pool_hashrate_hs
        );
        info!(
            "  Connected miners: {}",
            state_lock.pool_stats.connected_miners
        );
        info!("  Recent blocks: {}", state_lock.recent_blocks.len());
    }

    // Create and bind server
    let app = create_app(state);
    let addr: SocketAddr = args.bind.parse()?;

    info!("🚀 Pool API Stub listening on http://{}", addr);
    info!("📊 Available endpoints:");
    info!("  GET  /health                    - Health check");
    info!("  GET  /api/v1/pool/stats         - Pool statistics");
    info!("  GET  /api/v1/miners             - All miners");
    info!("  GET  /api/v1/miners/:id         - Specific miner");
    info!("  GET  /api/v1/payouts            - All payouts");
    info!("  GET  /api/v1/payouts/:address   - Specific payout");
    info!("  POST /api/v1/payouts/trigger    - Trigger payout");
    info!("  GET  /api/v1/blocks             - Recent blocks");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_state_initialization() {
        let state = StubState::default();

        assert!(!state.miners.is_empty());
        assert!(!state.payouts.is_empty());
        assert!(!state.recent_blocks.is_empty());
        assert!(state.pool_stats.pool_hashrate_hs > 0.0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = Arc::new(RwLock::new(StubState::default()));
        let result = health_check(State(state)).await;

        assert!(result.is_ok());
        let health = result.unwrap().0;
        assert_eq!(health.status, "healthy");
        assert_eq!(health.version, "0.1.0-stub");
    }
}
