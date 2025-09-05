// BUNKER POOL - Main Binary
// High-performance multi-algorithm mining pool server

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use clap::{Arg, Command};

use bunker_pool::{
    config::PoolConfig,
    stratum::StratumServer,
    share_processor::ShareProcessor,
    metrics::{PoolMetrics, MetricsServer, SystemMetricsCollector},
    init_logging, NAME, VERSION,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    setup_panic_handler();
    
    // Parse command line arguments
    let matches = Command::new(NAME)
        .version(VERSION)
        .about("BUNKER POOL - High-performance multi-algorithm mining pool")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("bunker-pool.toml")
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .default_value("info")
        )
        .arg(
            Arg::new("generate-config")
                .long("generate-config")
                .help("Generate default configuration file and exit")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config").unwrap();
    let log_level = matches.get_one::<String>("log-level").unwrap();

    // Initialize logging
    init_logging(log_level)?;

    info!("Starting {} v{}", NAME, VERSION);

    // Generate config file if requested
    if matches.get_flag("generate-config") {
        let default_config = PoolConfig::default();
        default_config.to_file(config_path)?;
        info!("Generated default configuration file: {}", config_path);
        return Ok(());
    }

    // Load configuration
    let config = if std::path::Path::new(config_path).exists() {
        info!("Loading configuration from: {}", config_path);
        PoolConfig::from_file(config_path)?
    } else {
        warn!("Configuration file not found, using defaults: {}", config_path);
        PoolConfig::default()
    };

    // Validate configuration
    config.validate()?;
    info!("Configuration validated successfully");
    info!("Pool: {} ({})", config.pool.name, config.pool.algorithm.as_str());
    info!("Max connections: {}", config.stratum.max_connections);
    info!("Fee: {}%", config.pool.fee_percentage);

    // Initialize metrics
    let metrics = Arc::new(PoolMetrics::new()?);
    info!("Metrics system initialized");

    // Start metrics server if enabled
    if config.metrics.enabled {
        let metrics_server = MetricsServer::new(
            Arc::clone(&metrics),
            config.metrics.bind_address,
        );
        
        tokio::spawn(async move {
            if let Err(e) = metrics_server.start().await {
                error!("Metrics server failed: {}", e);
            }
        });
        
        info!("Metrics server started on {}", config.metrics.bind_address);
    }

    // Start system metrics collector
    let system_collector = SystemMetricsCollector::new(Arc::clone(&metrics));
    system_collector.start(config.metrics.collection_interval_seconds).await;
    info!("System metrics collector started");

    // Create share processor channel
    let (share_tx, _share_rx) = mpsc::channel(config.share_processor.max_pending_shares);

    // Create job manager
    let (job_tx, _job_rx) = mpsc::channel(1000);
    let job_manager = Arc::new(
        bunker_pool::stratum::JobManager::new(
            config.to_coin_daemon_config(),
            job_tx,
        )
    );

    // Start share processor
    let share_processor = ShareProcessor::new(
        config.to_share_processor_config(),
        Arc::clone(&job_manager),
    ).await?;

    info!("Share processor started with {} workers", config.share_processor.worker_threads);

    // Create and start Stratum server
    let stratum_config = config.to_stratum_server_config();
    let stratum_server = StratumServer::new(stratum_config, share_tx);

    info!("Starting BUNKER POOL services...");
    info!("Stratum server will bind to: {}", config.stratum.bind_address);
    info!("Redis connection: {}", config.storage.redis_url);

    // Set up graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Shutdown signal received");
    };

    // Run the server with graceful shutdown
    tokio::select! {
        result = stratum_server.start() => {
            match result {
                Ok(_) => info!("Stratum server completed successfully"),
                Err(e) => error!("Stratum server failed: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("Initiating graceful shutdown...");
        }
    }

    // Perform cleanup
    share_processor.shutdown().await;
    
    // Final metrics report
    let final_stats = stratum_server.get_stats().await;
    info!("Final statistics:");
    info!("  Total connections: {}", final_stats.total_connections);
    info!("  Total shares: {}", final_stats.total_shares);
    info!("  Valid shares: {}", final_stats.valid_shares);
    info!("  Blocks found: {}", final_stats.blocks_found);

    if final_stats.total_shares > 0 {
        let accept_rate = (final_stats.valid_shares as f64 / final_stats.total_shares as f64) * 100.0;
        info!("  Accept rate: {:.2}%", accept_rate);
    }

    info!("BUNKER POOL shutdown complete");
    Ok(())
}

/// Handle panic by logging error details
fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let message = panic_info.payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| {
                panic_info.payload()
                    .downcast_ref::<String>()
                    .cloned()
            })
            .unwrap_or_else(|| "Unknown panic".to_string());

        let location = panic_info.location()
            .map(|loc| format!(" at {}:{}", loc.file(), loc.line()))
            .unwrap_or_default();

        error!("PANIC: {}{}", message, location);
        
        // Print backtrace in debug mode
        #[cfg(debug_assertions)]
        {
            eprintln!("Backtrace:\n{:?}", std::backtrace::Backtrace::capture());
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert!(!NAME.is_empty());
    }

    #[tokio::test]
    async fn test_config_generation() {
        use tempfile::NamedTempFile;
        
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_str().unwrap();
        
        let config = PoolConfig::default();
        config.to_file(config_path).unwrap();
        
        let loaded_config = PoolConfig::from_file(config_path).unwrap();
        assert_eq!(config.pool.name, loaded_config.pool.name);
        assert_eq!(config.pool.algorithm, loaded_config.pool.algorithm);
    }
}