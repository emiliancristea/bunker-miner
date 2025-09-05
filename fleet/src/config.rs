use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Database connection URL
    pub database_url: String,
    /// Redis connection URL for session management
    pub redis_url: String,
    /// JWT signing secret
    pub jwt_secret: String,
    /// JWT token expiration time in seconds
    pub jwt_expiration: i64,
    /// Environment (development, staging, production)
    pub environment: String,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to bind to
    pub port: u16,
    /// Whether to enable HTTPS
    pub tls_enabled: bool,
    /// TLS certificate file path
    pub tls_cert_path: Option<String>,
    /// TLS private key file path
    pub tls_key_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                tls_enabled: false,
                tls_cert_path: None,
                tls_key_path: None,
            },
            database_url: "postgresql://bunker:bunker@localhost:5432/bunker_fleet".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            jwt_secret: "bunker-fleet-jwt-secret-change-me-in-production".to_string(),
            jwt_expiration: 86400, // 24 hours
            environment: "development".to_string(),
        }
    }
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok(); // Load .env file if present

        let config = Self {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .context("Invalid SERVER_PORT")?,
                tls_enabled: env::var("TLS_ENABLED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .context("Invalid TLS_ENABLED")?,
                tls_cert_path: env::var("TLS_CERT_PATH").ok(),
                tls_key_path: env::var("TLS_KEY_PATH").ok(),
            },
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://bunker:bunker@localhost:5432/bunker_fleet".to_string()),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .context("JWT_SECRET environment variable is required")?,
            jwt_expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()
                .context("Invalid JWT_EXPIRATION")?,
            environment: env::var("ENVIRONMENT")
                .unwrap_or_else(|_| "development".to_string()),
        };

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate JWT secret is not the default in production
        if self.environment == "production" && self.jwt_secret.contains("change-me") {
            anyhow::bail!("JWT_SECRET must be set to a secure value in production");
        }

        // Validate TLS configuration
        if self.server.tls_enabled {
            if self.server.tls_cert_path.is_none() || self.server.tls_key_path.is_none() {
                anyhow::bail!("TLS_CERT_PATH and TLS_KEY_PATH must be set when TLS is enabled");
            }
        }

        // Validate database URL
        if !self.database_url.starts_with("postgresql://") {
            anyhow::bail!("DATABASE_URL must be a valid PostgreSQL connection string");
        }

        // Validate Redis URL
        if !self.redis_url.starts_with("redis://") {
            anyhow::bail!("REDIS_URL must be a valid Redis connection string");
        }

        Ok(())
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}