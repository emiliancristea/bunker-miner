use axum::{extract::State, Json};
use serde_json::Value;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    models::{AuthResponse, LoginRequest, RegisterRequest, User, UserProfile},
    AppState,
};

/// Register a new user
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    // Validate input
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err(AppError::ValidationError("Email and password are required".to_string()));
    }

    if payload.password.len() < 8 {
        return Err(AppError::ValidationError("Password must be at least 8 characters".to_string()));
    }

    // Check if user already exists
    let existing_user = sqlx::query!(
        "SELECT user_id FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(state.db.pool())
    .await?;

    if existing_user.is_some() {
        return Err(AppError::Conflict("User with this email already exists".to_string()));
    }

    // Hash password
    let password_hash = state.auth.hash_password(&payload.password)
        .map_err(|e| AppError::InternalServerError(format!("Failed to hash password: {}", e)))?;

    // Create user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO users (user_id, email, password_hash)
        VALUES ($1, $2, $3)
        "#,
        user_id,
        payload.email,
        password_hash
    )
    .execute(state.db.pool())
    .await?;

    // Fetch the created user
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE user_id = $1",
        user_id
    )
    .fetch_one(state.db.pool())
    .await?;

    // Generate JWT token
    let access_token = state.auth.generate_token(&user)
        .map_err(|e| AppError::InternalServerError(format!("Failed to generate token: {}", e)))?;

    // Get rig count
    let rig_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM rigs WHERE owner_user_id = $1 AND is_active = true",
        user_id
    )
    .fetch_one(state.db.pool())
    .await?
    .unwrap_or(0);

    info!("New user registered: {}", user.email);

    Ok(Json(AuthResponse {
        access_token: access_token.clone(),
        refresh_token: access_token, // For simplicity, using same token
        expires_in: 86400, // 24 hours
        user: UserProfile {
            user_id: user.user_id,
            email: user.email,
            created_at: user.created_at,
            rig_count,
        },
    }))
}

/// Login user
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    // Validate input
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err(AppError::ValidationError("Email and password are required".to_string()));
    }

    // Find user
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE email = $1 AND is_active = true",
        payload.email
    )
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    // Verify password
    let password_valid = state.auth.verify_password(&payload.password, &user.password_hash)
        .map_err(|e| AppError::InternalServerError(format!("Failed to verify password: {}", e)))?;

    if !password_valid {
        warn!("Invalid password attempt for user: {}", payload.email);
        return Err(AppError::Unauthorized("Invalid email or password".to_string()));
    }

    // Generate JWT token
    let access_token = state.auth.generate_token(&user)
        .map_err(|e| AppError::InternalServerError(format!("Failed to generate token: {}", e)))?;

    // Get rig count
    let rig_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM rigs WHERE owner_user_id = $1 AND is_active = true",
        user.user_id
    )
    .fetch_one(state.db.pool())
    .await?
    .unwrap_or(0);

    info!("User logged in: {}", user.email);

    Ok(Json(AuthResponse {
        access_token: access_token.clone(),
        refresh_token: access_token, // For simplicity, using same token
        expires_in: 86400, // 24 hours
        user: UserProfile {
            user_id: user.user_id,
            email: user.email,
            created_at: user.created_at,
            rig_count,
        },
    }))
}

/// Refresh JWT token
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> AppResult<Json<AuthResponse>> {
    let refresh_token = payload.get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::ValidationError("Refresh token is required".to_string()))?;

    // Validate the refresh token (in a real app, you'd have separate refresh tokens)
    let claims = state.auth.validate_token(refresh_token)
        .map_err(|_| AppError::Unauthorized("Invalid refresh token".to_string()))?;

    // Find user
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))?;

    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE user_id = $1 AND is_active = true",
        user_id
    )
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    // Generate new JWT token
    let access_token = state.auth.generate_token(&user)
        .map_err(|e| AppError::InternalServerError(format!("Failed to generate token: {}", e)))?;

    // Get rig count
    let rig_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM rigs WHERE owner_user_id = $1 AND is_active = true",
        user.user_id
    )
    .fetch_one(state.db.pool())
    .await?
    .unwrap_or(0);

    Ok(Json(AuthResponse {
        access_token: access_token.clone(),
        refresh_token: access_token, // For simplicity, using same token
        expires_in: 86400, // 24 hours
        user: UserProfile {
            user_id: user.user_id,
            email: user.email,
            created_at: user.created_at,
            rig_count,
        },
    }))
}

/// Get user profile
pub async fn get_profile(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> AppResult<Json<UserProfile>> {
    // Get rig count
    let rig_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM rigs WHERE owner_user_id = $1 AND is_active = true",
        auth_user.user_id
    )
    .fetch_one(state.db.pool())
    .await?
    .unwrap_or(0);

    // Get user info
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE user_id = $1",
        auth_user.user_id
    )
    .fetch_one(state.db.pool())
    .await?;

    Ok(Json(UserProfile {
        user_id: user.user_id,
        email: user.email,
        created_at: user.created_at,
        rig_count,
    }))
}