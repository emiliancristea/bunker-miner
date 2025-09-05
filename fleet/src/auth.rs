use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json, RequestPartsExt,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{Claims, User},
    AppState,
};

/// Authentication service for JWT generation and validation
#[derive(Debug, Clone)]
pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    jwt_expiration: i64,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(jwt_secret: String) -> Self {
        let encoding_key = EncodingKey::from_secret(jwt_secret.as_ref());
        let decoding_key = DecodingKey::from_secret(jwt_secret.as_ref());

        Self {
            encoding_key,
            decoding_key,
            jwt_expiration: 86400, // 24 hours
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.jwt_expiration);

        let claims = Claims {
            sub: user.user_id.to_string(),
            email: user.email.clone(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: "bunker-fleet-controller".to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .context("Failed to generate JWT token")
    }

    /// Validate a JWT token and return the claims
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["bunker-fleet-controller"]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .context("Invalid JWT token")?;

        Ok(token_data.claims)
    }

    /// Hash a password using Argon2
    pub fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .context("Failed to hash password")?
            .to_string();

        Ok(password_hash)
    }

    /// Verify a password against its hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .context("Failed to parse password hash")?;

        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    /// Generate a secure API key
    pub fn generate_api_key(&self) -> Result<(String, String)> {
        // Generate a 32-byte random key
        let key_bytes: [u8; 32] = ring::rand::generate(&ring::rand::SystemRandom::new())
            .context("Failed to generate random bytes")?
            .expose();

        // Format as hex string with "bk_" prefix
        let api_key = format!("bk_{}", hex::encode(key_bytes));
        
        // Hash the API key for storage
        let key_hash = self.hash_api_key(&api_key)?;

        Ok((api_key, key_hash))
    }

    /// Hash an API key using Argon2
    pub fn hash_api_key(&self, api_key: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let key_hash = argon2
            .hash_password(api_key.as_bytes(), &salt)
            .context("Failed to hash API key")?
            .to_string();

        Ok(key_hash)
    }

    /// Verify an API key against its hash
    pub fn verify_api_key(&self, api_key: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .context("Failed to parse API key hash")?;

        let argon2 = Argon2::default();
        Ok(argon2.verify_password(api_key.as_bytes(), &parsed_hash).is_ok())
    }
}

/// JWT claims extractor for authenticated requests
#[derive(Debug)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        
        // Extract the Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

        // Check if it's a Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("Invalid authorization format".to_string()))?;

        // Validate the token
        let claims = app_state
            .auth
            .validate_token(token)
            .map_err(|e| AppError::Unauthorized(format!("Invalid token: {}", e)))?;

        // Parse user ID
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))?;

        Ok(AuthenticatedUser {
            user_id,
            email: claims.email,
        })
    }
}

/// Authentication middleware
pub struct RequireAuth;

impl RequireAuth {
    pub async fn middleware(
        State(state): State<AppState>,
        mut req: axum::extract::Request,
        next: Next,
    ) -> Response {
        // Extract the Authorization header
        let auth_header = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok());

        let token = match auth_header {
            Some(header) => match header.strip_prefix("Bearer ") {
                Some(token) => token,
                None => {
                    warn!("Invalid authorization format");
                    return (StatusCode::UNAUTHORIZED, "Invalid authorization format").into_response();
                }
            },
            None => {
                warn!("Missing authorization header");
                return (StatusCode::UNAUTHORIZED, "Missing authorization header").into_response();
            }
        };

        // Validate the token
        let claims = match state.auth.validate_token(token) {
            Ok(claims) => claims,
            Err(e) => {
                warn!("Token validation failed: {}", e);
                return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
            }
        };

        // Parse user ID
        let user_id = match Uuid::parse_str(&claims.sub) {
            Ok(id) => id,
            Err(_) => {
                error!("Invalid user ID in token: {}", claims.sub);
                return (StatusCode::UNAUTHORIZED, "Invalid user ID").into_response();
            }
        };

        // Add user info to request extensions
        req.extensions_mut().insert(AuthenticatedUser {
            user_id,
            email: claims.email,
        });

        next.run(req).await
    }
}

/// API key authentication for daemon connections
#[derive(Debug)]
pub struct AuthenticatedRig {
    pub rig_id: Option<Uuid>,
    pub user_id: Uuid,
    pub permissions: Vec<String>,
}

impl AuthenticatedRig {
    /// Authenticate a rig using an API key
    pub async fn from_api_key(
        db: &PgPool,
        auth_service: &AuthService,
        api_key: &str,
    ) -> Result<Self, AppError> {
        // Find the API key in the database
        let key_record = sqlx::query!(
            r#"
            SELECT ak.owner_user_id, ak.rig_id, ak.permissions, ak.key_hash, ak.is_active, ak.expires_at
            FROM api_keys ak
            WHERE ak.key_prefix = $1 AND ak.is_active = true
            "#,
            &api_key[0..8.min(api_key.len())] // Extract prefix for lookup
        )
        .fetch_optional(db)
        .await
        .map_err(|e| {
            error!("Database error during API key lookup: {}", e);
            AppError::InternalServerError("Authentication failed".to_string())
        })?
        .ok_or_else(|| AppError::Unauthorized("Invalid API key".to_string()))?;

        // Check if the key has expired
        if let Some(expires_at) = key_record.expires_at {
            if expires_at < Utc::now() {
                warn!("Expired API key used");
                return Err(AppError::Unauthorized("API key has expired".to_string()));
            }
        }

        // Verify the API key hash
        if !auth_service
            .verify_api_key(api_key, &key_record.key_hash)
            .map_err(|e| {
                error!("API key verification error: {}", e);
                AppError::InternalServerError("Authentication failed".to_string())
            })?
        {
            warn!("Invalid API key provided");
            return Err(AppError::Unauthorized("Invalid API key".to_string()));
        }

        // Update last used timestamp
        if let Err(e) = sqlx::query!(
            "UPDATE api_keys SET last_used = NOW() WHERE key_hash = $1",
            &key_record.key_hash
        )
        .execute(db)
        .await
        {
            error!("Failed to update API key last_used: {}", e);
            // Don't fail authentication for this
        }

        Ok(AuthenticatedRig {
            rig_id: key_record.rig_id,
            user_id: key_record.owner_user_id,
            permissions: key_record.permissions.unwrap_or_default(),
        })
    }

    /// Check if the rig has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string()) || 
        self.permissions.contains(&"*".to_string()) // Wildcard permission
    }
}

/// FromRef trait implementation for extracting AppState from State
pub trait FromRef<T> {
    fn from_ref(input: &T) -> Self;
}

impl FromRef<AppState> for AppState {
    fn from_ref(input: &AppState) -> Self {
        input.clone()
    }
}