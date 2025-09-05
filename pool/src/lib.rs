// BUNKER POOL - High-Performance Mining Pool Library
// Multi-algorithm Stratum server and share processing system

pub mod stratum;
pub mod share_processor;
pub mod config;
pub mod metrics;

pub use stratum::{StratumServer, Algorithm, MinerConnection};
pub use share_processor::{ShareProcessor, ShareStorage, ShareValidator};

/// BUNKER POOL version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Initialize logging and tracing
pub fn init_logging(level: &str) -> Result<(), anyhow::Error> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}