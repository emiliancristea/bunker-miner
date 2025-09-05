use anyhow::{Context, Result};
use sqlx::{
    postgres::{PgPool, PgPoolOptions},
    migrate::Migrator,
};
use std::time::Duration;
use tracing::{info, error};

/// Database connection pool wrapper
#[derive(Clone, Debug)]
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// Create a new database connection pool
    pub async fn connect(database_url: &str) -> Result<Self> {
        info!("🔌 Connecting to PostgreSQL database...");
        
        let pool = PgPoolOptions::new()
            .max_connections(50)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL database")?;

        // Test the connection
        sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .context("Failed to verify database connection")?;

        info!("✓ Database connection established successfully");

        Ok(Self { pool })
    }

    /// Get a reference to the underlying connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("🔧 Running database migrations...");
        
        // Create the migrator from the migrations directory
        static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
        
        MIGRATOR
            .run(&self.pool)
            .await
            .context("Failed to run database migrations")?;

        info!("✓ Database migrations completed successfully");
        Ok(())
    }

    /// Check database health
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }

    /// Get database connection statistics
    pub fn connection_stats(&self) -> ConnectionStats {
        ConnectionStats {
            size: self.pool.size(),
            idle: self.pool.num_idle(),
        }
    }
}

/// Database connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub size: u32,
    pub idle: u32,
}

impl std::fmt::Display for ConnectionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pool size: {}, Idle: {}", self.size, self.idle)
    }
}