/*!
 * BUNKER MINER - Fleet Controller Smart Stub
 * 
 * Development stub service that implements the future Fleet Controller WebSocket API
 * with acknowledgment of connections and telemetry logging for local development and testing.
 * 
 * Features:
 * - WebSocket server for fleet management connections
 * - Connection acknowledgment and session tracking
 * - Telemetry reception and logging without processing logic
 * - Health checks and metrics endpoints
 * - Proper error handling and logging
 */

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use clap::Parser;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "fleet-controller-stub")]
#[command(about = "BUNKER MINER Fleet Controller Smart Stub")]
struct Args {
    /// WebSocket bind address
    #[arg(long, default_value = "0.0.0.0:8081")]
    ws_bind: String,
    
    /// HTTP bind address for health checks
    #[arg(long, default_value = "0.0.0.0:8082")]
    http_bind: String,
    
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
    
    /// Health check mode
    #[arg(long)]
    health_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConnection {
    pub session_id: String,
    pub miner_id: String,
    pub connected_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub telemetry_messages_received: u64,
    pub status: String,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user_agent: String,
    pub version: String,
    pub platform: String,
    pub remote_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryMessage {
    pub message_type: String,
    pub timestamp: DateTime<Utc>,
    pub miner_id: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetStats {
    pub connected_miners: u32,
    pub total_sessions: u64,
    pub active_sessions: u32,
    pub total_telemetry_received: u64,
    pub last_connection: Option<DateTime<Utc>>,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub memory_usage_mb: u64,
    pub active_connections: u32,
    pub total_messages_processed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WebSocketMessage {
    #[serde(rename = "connect")]
    Connect {
        miner_id: String,
        client_info: ClientInfo,
    },
    #[serde(rename = "telemetry")]
    Telemetry {
        miner_id: String,
        data: serde_json::Value,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat {
        miner_id: String,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "disconnect")]
    Disconnect {
        miner_id: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WebSocketResponse {
    #[serde(rename = "ack")]
    Acknowledgment {
        session_id: String,
        message: String,
    },
    #[serde(rename = "error")]
    Error {
        code: u32,
        message: String,
    },
    #[serde(rename = "pong")]
    Pong {
        timestamp: DateTime<Utc>,
    },
}

type AppState = Arc<RwLock<StubState>>;

#[derive(Debug)]
struct StubState {
    connections: HashMap<String, FleetConnection>,
    total_sessions: u64,
    total_telemetry_received: u64,
    start_time: DateTime<Utc>,
}

impl Default for StubState {
    fn default() -> Self {
        Self {
            connections: HashMap::new(),
            total_sessions: 0,
            total_telemetry_received: 0,
            start_time: Utc::now(),
        }
    }
}

// WebSocket Handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("🔌 New WebSocket connection request");
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    let session_id = Uuid::new_v4().to_string();
    info!("🔌 WebSocket connection established - Session: {}", session_id);
    
    // Send welcome message
    let welcome = WebSocketResponse::Acknowledgment {
        session_id: session_id.clone(),
        message: "Connected to BUNKER MINER Fleet Controller".to_string(),
    };
    
    if let Ok(welcome_msg) = serde_json::to_string(&welcome) {
        if socket.send(Message::Text(welcome_msg)).await.is_err() {
            warn!("Failed to send welcome message to session {}", session_id);
            return;
        }
    }

    let mut current_miner_id: Option<String> = None;

    // Handle incoming messages
    while let Some(msg_result) = socket.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                debug!("📨 Received message: {}", text);
                
                match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(ws_msg) => {
                        let response = handle_websocket_message(ws_msg, &session_id, &state, &mut current_miner_id).await;
                        
                        if let Ok(response_text) = serde_json::to_string(&response) {
                            if socket.send(Message::Text(response_text)).await.is_err() {
                                warn!("Failed to send response to session {}", session_id);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("❌ Invalid message format from {}: {}", session_id, e);
                        let error_response = WebSocketResponse::Error {
                            code: 400,
                            message: format!("Invalid message format: {}", e),
                        };
                        
                        if let Ok(error_text) = serde_json::to_string(&error_response) {
                            let _ = socket.send(Message::Text(error_text)).await;
                        }
                    }
                }
            }
            Ok(Message::Binary(_)) => {
                warn!("📨 Binary message received (not supported)");
                let error_response = WebSocketResponse::Error {
                    code: 415,
                    message: "Binary messages not supported".to_string(),
                };
                
                if let Ok(error_text) = serde_json::to_string(&error_response) {
                    let _ = socket.send(Message::Text(error_text)).await;
                }
            }
            Ok(Message::Ping(data)) => {
                debug!("🏓 Ping received from {}", session_id);
                if socket.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Pong(_)) => {
                debug!("🏓 Pong received from {}", session_id);
            }
            Ok(Message::Close(_)) => {
                info!("🔌 Client closed connection: {}", session_id);
                break;
            }
            Err(e) => {
                error!("❌ WebSocket error for session {}: {}", session_id, e);
                break;
            }
        }
    }

    // Clean up connection
    if let Some(miner_id) = current_miner_id {
        let mut state_lock = state.write().await;
        state_lock.connections.remove(&session_id);
        info!("🧹 Cleaned up session {} for miner {}", session_id, miner_id);
    }

    info!("🔌 WebSocket connection closed: {}", session_id);
}

async fn handle_websocket_message(
    message: WebSocketMessage,
    session_id: &str,
    state: &AppState,
    current_miner_id: &mut Option<String>,
) -> WebSocketResponse {
    match message {
        WebSocketMessage::Connect { miner_id, client_info } => {
            info!("🔗 Miner {} connecting with session {}", miner_id, session_id);
            
            let mut state_lock = state.write().await;
            let now = Utc::now();
            
            let connection = FleetConnection {
                session_id: session_id.to_string(),
                miner_id: miner_id.clone(),
                connected_at: now,
                last_seen: now,
                telemetry_messages_received: 0,
                status: "connected".to_string(),
                client_info,
            };
            
            state_lock.connections.insert(session_id.to_string(), connection);
            state_lock.total_sessions += 1;
            *current_miner_id = Some(miner_id.clone());
            
            WebSocketResponse::Acknowledgment {
                session_id: session_id.to_string(),
                message: format!("Miner {} successfully connected", miner_id),
            }
        }
        
        WebSocketMessage::Telemetry { miner_id, data } => {
            debug!("📊 Telemetry from {}: {:?}", miner_id, data);
            
            let mut state_lock = state.write().await;
            if let Some(connection) = state_lock.connections.get_mut(session_id) {
                connection.last_seen = Utc::now();
                connection.telemetry_messages_received += 1;
                state_lock.total_telemetry_received += 1;
            }
            
            // Log telemetry for development purposes
            info!("📊 Telemetry logged for miner {}: {} fields", miner_id, 
                  data.as_object().map(|o| o.len()).unwrap_or(0));
            
            WebSocketResponse::Acknowledgment {
                session_id: session_id.to_string(),
                message: "Telemetry received and logged".to_string(),
            }
        }
        
        WebSocketMessage::Heartbeat { miner_id, timestamp: _ } => {
            debug!("💓 Heartbeat from {}", miner_id);
            
            let mut state_lock = state.write().await;
            if let Some(connection) = state_lock.connections.get_mut(session_id) {
                connection.last_seen = Utc::now();
                connection.status = "active".to_string();
            }
            
            WebSocketResponse::Pong {
                timestamp: Utc::now(),
            }
        }
        
        WebSocketMessage::Disconnect { miner_id, reason } => {
            info!("👋 Miner {} disconnecting: {}", miner_id, reason);
            
            let mut state_lock = state.write().await;
            if let Some(connection) = state_lock.connections.get_mut(session_id) {
                connection.status = "disconnected".to_string();
                connection.last_seen = Utc::now();
            }
            
            WebSocketResponse::Acknowledgment {
                session_id: session_id.to_string(),
                message: format!("Disconnect acknowledged for {}", miner_id),
            }
        }
    }
}

// HTTP API Handlers
async fn get_fleet_stats(State(state): State<AppState>) -> Result<Json<FleetStats>, StatusCode> {
    let state_lock = state.read().await;
    let now = Utc::now();
    
    let stats = FleetStats {
        connected_miners: state_lock.connections.len() as u32,
        total_sessions: state_lock.total_sessions,
        active_sessions: state_lock.connections.values()
            .filter(|c| c.status == "connected" || c.status == "active")
            .count() as u32,
        total_telemetry_received: state_lock.total_telemetry_received,
        last_connection: state_lock.connections.values()
            .map(|c| c.connected_at)
            .max(),
        uptime_seconds: (now - state_lock.start_time).num_seconds() as u64,
    };
    
    Ok(Json(stats))
}

async fn get_connections(State(state): State<AppState>) -> Result<Json<Vec<FleetConnection>>, StatusCode> {
    let state_lock = state.read().await;
    let connections: Vec<FleetConnection> = state_lock.connections.values().cloned().collect();
    Ok(Json(connections))
}

async fn health_check(State(state): State<AppState>) -> Result<Json<HealthStatus>, StatusCode> {
    let state_lock = state.read().await;
    let now = Utc::now();
    
    // Simple memory usage estimation
    let memory_usage = (state_lock.connections.len() * 1024) as u64; // Rough estimate
    
    Ok(Json(HealthStatus {
        status: "healthy".to_string(),
        timestamp: now,
        version: "0.1.0-stub".to_string(),
        uptime_seconds: (now - state_lock.start_time).num_seconds() as u64,
        memory_usage_mb: memory_usage / 1024 / 1024,
        active_connections: state_lock.connections.len() as u32,
        total_messages_processed: state_lock.total_telemetry_received,
    }))
}

async fn websocket_info() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>BUNKER MINER Fleet Controller Stub</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .info { background: #f0f0f0; padding: 20px; border-radius: 5px; }
        code { background: #e0e0e0; padding: 2px 4px; border-radius: 3px; }
    </style>
</head>
<body>
    <h1>🚀 BUNKER MINER Fleet Controller Stub</h1>
    <div class="info">
        <h2>WebSocket Endpoints</h2>
        <ul>
            <li><strong>WebSocket:</strong> <code>ws://localhost:8081/ws</code></li>
            <li><strong>Health Check:</strong> <code>http://localhost:8082/health</code></li>
            <li><strong>Fleet Stats:</strong> <code>http://localhost:8082/api/v1/fleet/stats</code></li>
            <li><strong>Connections:</strong> <code>http://localhost:8082/api/v1/fleet/connections</code></li>
        </ul>
        
        <h2>Supported WebSocket Messages</h2>
        <ul>
            <li><strong>Connect:</strong> Register a miner with the fleet</li>
            <li><strong>Telemetry:</strong> Send mining telemetry data</li>
            <li><strong>Heartbeat:</strong> Keep connection alive</li>
            <li><strong>Disconnect:</strong> Graceful disconnection</li>
        </ul>
        
        <p><em>This is a development stub service. All messages are acknowledged and logged but not processed.</em></p>
    </div>
</body>
</html>
    "#)
}

fn create_ws_app(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/", get(websocket_info))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        )
}

fn create_http_app(state: AppState) -> Router {
    Router::new()
        // Fleet management endpoints
        .route("/api/v1/fleet/stats", get(get_fleet_stats))
        .route("/api/v1/fleet/connections", get(get_connections))
        
        // Health and monitoring
        .route("/health", get(health_check))
        .route("/api/v1/health", get(health_check))
        
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
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
        .with_env_filter(format!("fleet_controller_stub={},tower_http=debug", log_level))
        .init();

    info!("🚀 Starting BUNKER MINER Fleet Controller Stub");
    info!("Version: 0.1.0-stub");
    info!("WebSocket address: {}", args.ws_bind);
    info!("HTTP address: {}", args.http_bind);

    // Initialize shared state
    let state = Arc::new(RwLock::new(StubState::default()));

    // Create applications
    let ws_app = create_ws_app(state.clone());
    let http_app = create_http_app(state.clone());

    // Parse addresses
    let ws_addr: SocketAddr = args.ws_bind.parse()?;
    let http_addr: SocketAddr = args.http_bind.parse()?;

    info!("🌐 WebSocket server starting on ws://{}/ws", ws_addr);
    info!("🌐 HTTP API server starting on http://{}", http_addr);
    info!("📋 Available endpoints:");
    info!("  WebSocket: ws://{}/ws", ws_addr);
    info!("  GET  http://{}/health", http_addr);
    info!("  GET  http://{}/api/v1/fleet/stats", http_addr);
    info!("  GET  http://{}/api/v1/fleet/connections", http_addr);

    // Start both servers concurrently
    let ws_listener = tokio::net::TcpListener::bind(&ws_addr).await?;
    let http_listener = tokio::net::TcpListener::bind(&http_addr).await?;

    let ws_server = axum::serve(ws_listener, ws_app);
    let http_server = axum::serve(http_listener, http_app);

    // Run both servers
    tokio::try_join!(ws_server, http_server)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_state_initialization() {
        let state = StubState::default();
        
        assert!(state.connections.is_empty());
        assert_eq!(state.total_sessions, 0);
        assert_eq!(state.total_telemetry_received, 0);
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