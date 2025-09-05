use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo,
    },
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, get_service},
    Extension,
    Router,
};
use serde_json::json;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::{debug, error, info, warn};

use crate::{config::Config, grpc::TelemetryBroadcaster, miners::Telemetry};

pub struct WebDashboardServer {
    config: Config,
    telemetry_broadcaster: Arc<TelemetryBroadcaster>,
}

impl WebDashboardServer {
    pub fn new(config: Config, telemetry_broadcaster: Arc<TelemetryBroadcaster>) -> Self {
        Self {
            config,
            telemetry_broadcaster,
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let bind_addr = format!("127.0.0.1:{}", self.config.grpc.port + 100); // Use port 50151 if gRPC is on 50051
        let addr: SocketAddr = bind_addr.parse()?;

        info!("Starting web dashboard server on http://{}", addr);

        // Create app state
        let app_state = AppState {
            telemetry_broadcaster: self.telemetry_broadcaster.clone(),
            config: self.config.clone(),
        };

        // Build our application with routes
        let app = create_router(app_state);

        // Create listener
        let listener = tokio::net::TcpListener::bind(addr).await?;

        info!("Web dashboard server listening on {}", addr);
        info!("Dashboard URL: http://{}", addr);

        // Start the server
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    telemetry_broadcaster: Arc<TelemetryBroadcaster>,
    config: Config,
}

fn create_router(state: AppState) -> Router {
    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin("http://127.0.0.1:*".parse::<HeaderValue>().unwrap())
        .allow_origin("http://localhost:*".parse::<HeaderValue>().unwrap())
        .allow_methods([axum::http::Method::GET])
        .allow_headers(Any);

    Router::new()
        .route("/", get(dashboard_handler))
        .route("/ws", get(websocket_handler))
        .route("/api/status", get(status_handler))
        .nest_service("/static", get_service(ServeDir::new("web/static")))
        .layer(Extension(state))
        .layer(cors)
}

// Dashboard HTML page handler
async fn dashboard_handler() -> impl IntoResponse {
    Html(include_str!("../web/dashboard.html"))
}

// API status handler
async fn status_handler(Extension(state): Extension<AppState>) -> impl IntoResponse {
    let status = json!({
        "server": "BUNKER MINER Web Dashboard",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "profit_switching_enabled": state.config.profit_switching.enable,
        "telemetry_subscribers": state.telemetry_broadcaster.subscriber_count()
    });

    axum::Json(status)
}

// WebSocket handler for real-time telemetry
async fn websocket_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Extension(state): Extension<AppState>,
) -> Response {
    // Security check: validate Origin header to prevent CSWSH attacks
    if let Some(origin) = headers.get("origin") {
        if let Ok(origin_str) = origin.to_str() {
            // Only allow localhost origins
            if !origin_str.starts_with("http://127.0.0.1:")
                && !origin_str.starts_with("http://localhost:")
            {
                warn!(
                    "WebSocket connection rejected from invalid origin: {} (client: {})",
                    origin_str, addr
                );
                return StatusCode::FORBIDDEN.into_response();
            }
        }
    } else {
        // Require Origin header for security
        warn!(
            "WebSocket connection rejected: missing Origin header (client: {})",
            addr
        );
        return StatusCode::BAD_REQUEST.into_response();
    }

    info!("WebSocket connection established from {}", addr);

    ws.on_upgrade(move |socket| handle_websocket(socket, state, addr))
}

async fn handle_websocket(socket: WebSocket, state: AppState, addr: SocketAddr) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to telemetry broadcasts
    let mut telemetry_receiver = state.telemetry_broadcaster.subscribe();

    // Spawn task to handle incoming messages (ping/pong, close, etc.)
    let ping_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => {
                    debug!("WebSocket client {} disconnected", addr);
                    break;
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong from {}", addr);
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received ping from {}, sending pong", addr);
                    if sender.send(Message::Pong(data)).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Text(_)) | Ok(Message::Binary(_)) => {
                    // We don't handle client messages in this simple implementation
                    debug!("Received message from {}, ignoring", addr);
                }
                Err(e) => {
                    error!("WebSocket error from {}: {}", addr, e);
                    break;
                }
            }
        }
    });

    // Main telemetry broadcasting loop
    let broadcast_task = tokio::spawn(async move {
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                // Handle telemetry data
                telemetry_result = telemetry_receiver.recv() => {
                    match telemetry_result {
                        Ok(telemetry) => {
                            let telemetry_json = convert_telemetry_to_json(telemetry);

                            let message = json!({
                                "type": "telemetry",
                                "data": telemetry_json,
                                "timestamp": SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                            });

                            if let Ok(json_str) = serde_json::to_string(&message) {
                                if sender.send(Message::Text(json_str)).await.is_err() {
                                    debug!("Failed to send telemetry to {}, client disconnected", addr);
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("Telemetry broadcaster closed, ending WebSocket connection to {}", addr);
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(missed)) => {
                            warn!("WebSocket client {} lagged behind, missed {} messages", addr, missed);
                            // Continue receiving
                        }
                    }
                }

                // Send periodic ping to keep connection alive
                _ = ping_interval.tick() => {
                    if sender.send(Message::Ping(vec![])).await.is_err() {
                        debug!("Failed to send ping to {}, client disconnected", addr);
                        break;
                    }
                }
            }
        }

        // Send close message
        let _ = sender.send(Message::Close(None)).await;
        debug!("WebSocket connection to {} closed", addr);
    });

    // Wait for either task to complete
    tokio::select! {
        _ = ping_task => {
            broadcast_task.abort();
        }
        _ = broadcast_task => {
            ping_task.abort();
        }
    }

    info!("WebSocket connection to {} ended", addr);
}

fn convert_telemetry_to_json(telemetry: Telemetry) -> serde_json::Value {
    json!({
        "device_id": "device_0", // TODO: Use actual device ID
        "algorithm": telemetry.algorithm,
        "hashrate_mhs": telemetry.hashrate_hs / 1_000_000.0, // Convert H/s to MH/s
        "power_watts": telemetry.power_watts.unwrap_or(0.0),
        "temperature_celsius": telemetry.temperature_c.unwrap_or(0.0),
        "fan_speed_percent": telemetry.fan_speed_percent.unwrap_or(0.0),
        "shares_accepted": telemetry.shares_accepted,
        "shares_rejected": telemetry.shares_rejected,
        "shares_stale": telemetry.shares_stale,
        "acceptance_rate": if telemetry.shares_accepted + telemetry.shares_rejected > 0 {
            telemetry.shares_accepted as f32 / (telemetry.shares_accepted + telemetry.shares_rejected) as f32
        } else {
            0.0
        },
        "timestamp": telemetry.timestamp,
        "uptime_seconds": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - telemetry.timestamp
    })
}

use futures_util::{SinkExt, StreamExt};