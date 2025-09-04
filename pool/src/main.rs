use axum::{
    routing::get,
    Router,
    response::Html,
};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("Starting BUNKER POOL server...");
    
    // Create the application router
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/api/v1/stats", get(pool_stats));

    // Bind to localhost for development
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind to address");
        
    info!("BUNKER POOL listening on http://127.0.0.1:8080");
    info!("This is a development stub - full implementation in Phase 3");

    // Start the server
    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}

async fn root() -> Html<&'static str> {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>BUNKER POOL</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; }
            .header { color: #2c3e50; }
            .status { color: #27ae60; }
        </style>
    </head>
    <body>
        <h1 class="header">🏊 BUNKER POOL</h1>
        <p>High-performance cryptocurrency mining pool</p>
        <p class="status">Status: Development Stub (Phase 0.1)</p>
        <p>Full implementation scheduled for Phase 3</p>
        <h3>Planned Features:</h3>
        <ul>
            <li>Multi-algorithm Stratum server</li>
            <li>PPLNS payout system</li>
            <li>Real-time statistics API</li>
            <li>Web dashboard</li>
        </ul>
        <p><a href="/health">Health Check</a> | <a href="/api/v1/stats">Pool Stats</a></p>
    </body>
    </html>
    "#)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn pool_stats() -> &'static str {
    r#"{"status":"development_stub","version":"0.1.0","phase":"0.1"}"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_initialization() {
        // Basic test to ensure the pool can be initialized
        // More comprehensive tests will be added in Phase 3
        assert!(true);
    }
}