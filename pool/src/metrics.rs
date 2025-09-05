// BUNKER POOL - Metrics and Monitoring
// Prometheus metrics collection for pool performance monitoring

use prometheus::{
    Counter, Gauge, Histogram, IntCounter, IntGauge, Registry, Encoder, TextEncoder
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

/// BUNKER POOL metrics collector
pub struct PoolMetrics {
    // Connection metrics
    pub active_connections: IntGauge,
    pub total_connections: IntCounter,
    pub connection_errors: IntCounter,
    
    // Share metrics
    pub total_shares: IntCounter,
    pub valid_shares: IntCounter,
    pub invalid_shares: IntCounter,
    pub blocks_found: IntCounter,
    pub share_validation_time: Histogram,
    
    // Pool metrics
    pub pool_hashrate: Gauge,
    pub current_difficulty: Gauge,
    pub active_miners: IntGauge,
    
    // System metrics
    pub memory_usage: Gauge,
    pub cpu_usage: Gauge,
    pub redis_operations: IntCounter,
    pub redis_errors: IntCounter,
    
    // Registry for Prometheus
    registry: Registry,
}

impl PoolMetrics {
    pub fn new() -> Result<Self, anyhow::Error> {
        let registry = Registry::new();

        // Connection metrics
        let active_connections = IntGauge::new(
            "bunker_pool_active_connections",
            "Number of active miner connections"
        )?;
        let total_connections = IntCounter::new(
            "bunker_pool_total_connections",
            "Total number of miner connections since startup"
        )?;
        let connection_errors = IntCounter::new(
            "bunker_pool_connection_errors",
            "Number of connection errors"
        )?;

        // Share metrics
        let total_shares = IntCounter::new(
            "bunker_pool_total_shares",
            "Total number of shares submitted"
        )?;
        let valid_shares = IntCounter::new(
            "bunker_pool_valid_shares",
            "Number of valid shares accepted"
        )?;
        let invalid_shares = IntCounter::new(
            "bunker_pool_invalid_shares",
            "Number of invalid shares rejected"
        )?;
        let blocks_found = IntCounter::new(
            "bunker_pool_blocks_found",
            "Number of blocks found by the pool"
        )?;
        let share_validation_time = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "bunker_pool_share_validation_duration_seconds",
                "Time spent validating shares"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0])
        )?;

        // Pool metrics
        let pool_hashrate = Gauge::new(
            "bunker_pool_hashrate",
            "Current pool hashrate in H/s"
        )?;
        let current_difficulty = Gauge::new(
            "bunker_pool_current_difficulty",
            "Current mining difficulty"
        )?;
        let active_miners = IntGauge::new(
            "bunker_pool_active_miners",
            "Number of unique active miners"
        )?;

        // System metrics
        let memory_usage = Gauge::new(
            "bunker_pool_memory_usage_bytes",
            "Memory usage in bytes"
        )?;
        let cpu_usage = Gauge::new(
            "bunker_pool_cpu_usage_percent",
            "CPU usage percentage"
        )?;
        let redis_operations = IntCounter::new(
            "bunker_pool_redis_operations_total",
            "Total Redis operations"
        )?;
        let redis_errors = IntCounter::new(
            "bunker_pool_redis_errors_total",
            "Total Redis errors"
        )?;

        // Register all metrics
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(total_connections.clone()))?;
        registry.register(Box::new(connection_errors.clone()))?;
        registry.register(Box::new(total_shares.clone()))?;
        registry.register(Box::new(valid_shares.clone()))?;
        registry.register(Box::new(invalid_shares.clone()))?;
        registry.register(Box::new(blocks_found.clone()))?;
        registry.register(Box::new(share_validation_time.clone()))?;
        registry.register(Box::new(pool_hashrate.clone()))?;
        registry.register(Box::new(current_difficulty.clone()))?;
        registry.register(Box::new(active_miners.clone()))?;
        registry.register(Box::new(memory_usage.clone()))?;
        registry.register(Box::new(cpu_usage.clone()))?;
        registry.register(Box::new(redis_operations.clone()))?;
        registry.register(Box::new(redis_errors.clone()))?;

        Ok(Self {
            active_connections,
            total_connections,
            connection_errors,
            total_shares,
            valid_shares,
            invalid_shares,
            blocks_found,
            share_validation_time,
            pool_hashrate,
            current_difficulty,
            active_miners,
            memory_usage,
            cpu_usage,
            redis_operations,
            redis_errors,
            registry,
        })
    }

    /// Record new connection
    pub fn record_connection(&self) {
        self.active_connections.inc();
        self.total_connections.inc();
        debug!("Connection metrics updated: {} active", self.active_connections.get());
    }

    /// Record connection closed
    pub fn record_disconnection(&self) {
        self.active_connections.dec();
        debug!("Connection closed: {} active", self.active_connections.get());
    }

    /// Record connection error
    pub fn record_connection_error(&self) {
        self.connection_errors.inc();
    }

    /// Record share submission
    pub fn record_share(&self, is_valid: bool, is_block: bool, validation_time: f64) {
        self.total_shares.inc();
        
        if is_valid {
            self.valid_shares.inc();
            if is_block {
                self.blocks_found.inc();
            }
        } else {
            self.invalid_shares.inc();
        }
        
        self.share_validation_time.observe(validation_time);
    }

    /// Update pool hashrate
    pub fn update_pool_hashrate(&self, hashrate: f64) {
        self.pool_hashrate.set(hashrate);
    }

    /// Update current difficulty
    pub fn update_difficulty(&self, difficulty: f64) {
        self.current_difficulty.set(difficulty);
    }

    /// Update active miner count
    pub fn update_active_miners(&self, count: i64) {
        self.active_miners.set(count);
    }

    /// Record Redis operation
    pub fn record_redis_operation(&self, success: bool) {
        self.redis_operations.inc();
        if !success {
            self.redis_errors.inc();
        }
    }

    /// Update system metrics
    pub fn update_system_metrics(&self, memory_bytes: f64, cpu_percent: f64) {
        self.memory_usage.set(memory_bytes);
        self.cpu_usage.set(cpu_percent);
    }

    /// Get metrics as Prometheus format string
    pub fn gather(&self) -> Result<String, anyhow::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Get registry for external use
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

/// Metrics server for Prometheus scraping
pub struct MetricsServer {
    metrics: Arc<PoolMetrics>,
    bind_address: std::net::SocketAddr,
}

impl MetricsServer {
    pub fn new(metrics: Arc<PoolMetrics>, bind_address: std::net::SocketAddr) -> Self {
        Self {
            metrics,
            bind_address,
        }
    }

    /// Start the metrics HTTP server
    pub async fn start(&self) -> Result<(), anyhow::Error> {
        use axum::{routing::get, Router};
        
        let metrics = Arc::clone(&self.metrics);
        
        let app = Router::new()
            .route("/metrics", get(move || async move {
                match metrics.gather() {
                    Ok(metrics_text) => axum::response::Response::builder()
                        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
                        .body(axum::body::Body::from(metrics_text))
                        .unwrap(),
                    Err(e) => {
                        error!("Failed to gather metrics: {}", e);
                        axum::response::Response::builder()
                            .status(500)
                            .body(axum::body::Body::from("Internal Server Error"))
                            .unwrap()
                    }
                }
            }))
            .route("/health", get(|| async {
                "OK"
            }));

        let listener = tokio::net::TcpListener::bind(self.bind_address).await?;
        
        tracing::info!("Metrics server listening on {}", self.bind_address);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

/// System metrics collector
pub struct SystemMetricsCollector {
    metrics: Arc<PoolMetrics>,
}

impl SystemMetricsCollector {
    pub fn new(metrics: Arc<PoolMetrics>) -> Self {
        Self { metrics }
    }

    /// Start collecting system metrics periodically
    pub async fn start(&self, interval_seconds: u64) {
        let metrics = Arc::clone(&self.metrics);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_seconds));
            
            loop {
                interval.tick().await;
                
                // Collect memory usage
                if let Ok(memory_info) = Self::get_memory_usage() {
                    metrics.update_system_metrics(memory_info.rss_bytes as f64, 0.0);
                }

                // Collect CPU usage (simplified)
                if let Ok(cpu_percent) = Self::get_cpu_usage().await {
                    metrics.cpu_usage.set(cpu_percent);
                }
            }
        });
    }

    /// Get memory usage information
    fn get_memory_usage() -> Result<MemoryInfo, anyhow::Error> {
        // Read from /proc/self/status on Linux
        #[cfg(target_os = "linux")]
        {
            let status = std::fs::read_to_string("/proc/self/status")?;
            let mut rss_kb = 0u64;
            
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        rss_kb = parts[1].parse().unwrap_or(0);
                        break;
                    }
                }
            }
            
            Ok(MemoryInfo {
                rss_bytes: rss_kb * 1024,
            })
        }
        
        // Simplified fallback for other platforms
        #[cfg(not(target_os = "linux"))]
        {
            Ok(MemoryInfo {
                rss_bytes: 0, // Would need platform-specific implementation
            })
        }
    }

    /// Get CPU usage (simplified implementation)
    async fn get_cpu_usage() -> Result<f64, anyhow::Error> {
        // This is a simplified implementation
        // In production, you would use a more sophisticated CPU monitoring library
        Ok(0.0)
    }
}

#[derive(Debug)]
struct MemoryInfo {
    rss_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = PoolMetrics::new().unwrap();
        
        // Test initial values
        assert_eq!(metrics.active_connections.get(), 0);
        assert_eq!(metrics.total_shares.get(), 0);
    }

    #[test]
    fn test_connection_metrics() {
        let metrics = PoolMetrics::new().unwrap();
        
        metrics.record_connection();
        assert_eq!(metrics.active_connections.get(), 1);
        assert_eq!(metrics.total_connections.get(), 1);
        
        metrics.record_disconnection();
        assert_eq!(metrics.active_connections.get(), 0);
        assert_eq!(metrics.total_connections.get(), 1);
    }

    #[test]
    fn test_share_metrics() {
        let metrics = PoolMetrics::new().unwrap();
        
        metrics.record_share(true, false, 0.005);
        assert_eq!(metrics.total_shares.get(), 1);
        assert_eq!(metrics.valid_shares.get(), 1);
        assert_eq!(metrics.invalid_shares.get(), 0);
        assert_eq!(metrics.blocks_found.get(), 0);
        
        metrics.record_share(true, true, 0.010);
        assert_eq!(metrics.total_shares.get(), 2);
        assert_eq!(metrics.valid_shares.get(), 2);
        assert_eq!(metrics.blocks_found.get(), 1);
        
        metrics.record_share(false, false, 0.002);
        assert_eq!(metrics.total_shares.get(), 3);
        assert_eq!(metrics.valid_shares.get(), 2);
        assert_eq!(metrics.invalid_shares.get(), 1);
    }

    #[test]
    fn test_metrics_gathering() {
        let metrics = PoolMetrics::new().unwrap();
        
        // Add some test data
        metrics.record_connection();
        metrics.record_share(true, false, 0.005);
        metrics.update_pool_hashrate(1000000.0);
        
        // Gather metrics
        let output = metrics.gather().unwrap();
        
        assert!(output.contains("bunker_pool_active_connections"));
        assert!(output.contains("bunker_pool_total_shares"));
        assert!(output.contains("bunker_pool_hashrate"));
    }
}