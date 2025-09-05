mod auth;
mod database;
mod models;
mod handlers;
mod websocket;
mod config;
mod error;

use anyhow::Result;
use axum::{
    extract::{Extension, State},
    http::{header, Method, StatusCode},
    middleware,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    auth::{AuthService, RequireAuth},
    config::AppConfig,
    database::DatabasePool,
    handlers::{auth as auth_handlers, fleet as fleet_handlers, api as api_handlers},
    websocket::WebSocketManager,
};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool
    pub db: DatabasePool,
    /// Authentication service
    pub auth: AuthService,
    /// WebSocket connection manager
    pub ws_manager: Arc<WebSocketManager>,
    /// Application configuration
    pub config: Arc<AppConfig>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bunker_fleet_controller=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting BUNKER MINER Fleet Controller v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Arc::new(AppConfig::load()?);
    info!("✓ Configuration loaded successfully");

    // Initialize database
    let db = DatabasePool::connect(&config.database_url).await?;
    db.run_migrations().await?;
    info!("✓ Database connected and migrations applied");

    // Initialize authentication service
    let auth = AuthService::new(config.jwt_secret.clone());
    info!("✓ Authentication service initialized");

    // Initialize WebSocket manager
    let ws_manager = Arc::new(WebSocketManager::new());
    info!("✓ WebSocket manager initialized");

    // Create application state
    let state = AppState {
        db,
        auth,
        ws_manager,
        config: config.clone(),
    };

    // Build the application router
    let app = create_app_router(state);

    // Start the server
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .expect("Invalid server address");

    info!("🌐 Fleet Controller starting on http://{}", addr);
    info!("📊 Dashboard available at: http://{}/dashboard", addr);
    info!("🔌 WebSocket API: ws://{}/api/fleet/ws", addr);
    info!("🔒 Authentication: JWT-based with secure API keys");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    info!("✅ BUNKER MINER Fleet Controller is ready!");
    info!("   API Endpoints:");
    info!("     POST /api/auth/register    - User registration");
    info!("     POST /api/auth/login       - User authentication");
    info!("     GET  /api/fleet/rigs       - List user's rigs");
    info!("     POST /api/fleet/rigs       - Register new rig");
    info!("     WS   /api/fleet/ws         - Real-time rig connection");
    info!("     GET  /dashboard            - Web dashboard");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the main application router with all endpoints
fn create_app_router(state: AppState) -> Router {
    // API routes that require authentication
    let protected_routes = Router::new()
        .route("/fleet/rigs", get(fleet_handlers::list_rigs))
        .route("/fleet/rigs", post(fleet_handlers::register_rig))
        .route("/fleet/rigs/:rig_id", get(fleet_handlers::get_rig))
        .route("/fleet/rigs/:rig_id", put(fleet_handlers::update_rig))
        .route("/fleet/rigs/:rig_id", delete(fleet_handlers::delete_rig))
        .route("/fleet/rigs/:rig_id/commands", post(fleet_handlers::send_command))
        .route("/fleet/telemetry", get(fleet_handlers::get_telemetry))
        .route("/user/profile", get(auth_handlers::get_profile))
        .route("/user/api-keys", get(api_handlers::list_api_keys))
        .route("/user/api-keys", post(api_handlers::create_api_key))
        .route("/user/api-keys/:key_id", delete(api_handlers::revoke_api_key))
        .layer(middleware::from_fn_with_state(state.clone(), RequireAuth::middleware));

    // Public API routes
    let public_routes = Router::new()
        .route("/auth/register", post(auth_handlers::register))
        .route("/auth/login", post(auth_handlers::login))
        .route("/auth/refresh", post(auth_handlers::refresh_token))
        .route("/health", get(health_check));

    // WebSocket endpoint for rig connections
    let websocket_routes = Router::new()
        .route("/fleet/ws", get(websocket::ws_handler))
        .route("/fleet/dashboard/ws", get(websocket::dashboard_ws_handler));

    // Combine API routes
    let api_routes = Router::new()
        .nest("/api", public_routes)
        .nest("/api", protected_routes)
        .nest("/api", websocket_routes);

    // Static file serving for the dashboard
    let static_routes = Router::new()
        .route("/", get(serve_dashboard))
        .route("/dashboard", get(serve_dashboard))
        .route("/dashboard/*path", get(serve_dashboard))
        .fallback_service(tower::service_fn(|_| async {
            (StatusCode::NOT_FOUND, "Not Found")
        }));

    // Main application router
    Router::new()
        .merge(api_routes)
        .merge(static_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
                        .allow_credentials(true),
                )
                .into_inner(),
        )
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "bunker-fleet-controller",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now()
    }))
}

/// Serve the dashboard SPA
async fn serve_dashboard() -> Result<axum::response::Html<&'static str>, StatusCode> {
    // In production, this would serve the built React/Vue/Svelte app
    // For now, return a placeholder HTML page
    Ok(axum::response::Html(include_str!("../static/dashboard.html")))
}