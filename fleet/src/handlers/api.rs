use axum::{extract::{Path, State}, Json};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    models::{ApiKey, ApiKeyResponse, CreateApiKeyRequest},
    AppState,
};

/// List user's API keys
pub async fn list_api_keys(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ApiKeyResponse>>> {
    let keys = sqlx::query_as!(
        ApiKey,
        r#"
        SELECT * FROM api_keys 
        WHERE owner_user_id = $1 AND is_active = true
        ORDER BY created_at DESC
        "#,
        auth_user.user_id
    )
    .fetch_all(state.db.pool())
    .await?;

    let responses: Vec<ApiKeyResponse> = keys.into_iter().map(|key| ApiKeyResponse {
        key_id: key.key_id,
        key_name: key.key_name,
        key_prefix: key.key_prefix,
        actual_key: None, // Never return the actual key in list
        rig_id: key.rig_id,
        permissions: key.permissions,
        is_active: key.is_active,
        last_used: key.last_used,
        expires_at: key.expires_at,
        created_at: key.created_at,
    }).collect();

    Ok(Json(responses))
}

/// Create a new API key
pub async fn create_api_key(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> AppResult<Json<ApiKeyResponse>> {
    // Validate input
    if payload.key_name.trim().is_empty() {
        return Err(AppError::ValidationError("Key name is required".to_string()));
    }

    // Check if key name already exists for this user
    let existing_key = sqlx::query!(
        "SELECT key_id FROM api_keys WHERE owner_user_id = $1 AND key_name = $2 AND is_active = true",
        auth_user.user_id,
        payload.key_name
    )
    .fetch_optional(state.db.pool())
    .await?;

    if existing_key.is_some() {
        return Err(AppError::Conflict("API key with this name already exists".to_string()));
    }

    // If rig_id is specified, verify ownership
    if let Some(rig_id) = payload.rig_id {
        let rig_exists = sqlx::query!(
            "SELECT rig_id FROM rigs WHERE rig_id = $1 AND owner_user_id = $2 AND is_active = true",
            rig_id,
            auth_user.user_id
        )
        .fetch_optional(state.db.pool())
        .await?
        .is_some();

        if !rig_exists {
            return Err(AppError::NotFound("Rig not found".to_string()));
        }
    }

    // Generate API key
    let (api_key, key_hash) = state.auth.generate_api_key()
        .map_err(|e| AppError::InternalServerError(format!("Failed to generate API key: {}", e)))?;

    let key_prefix = api_key[0..8].to_string(); // Extract prefix for display

    // Validate permissions
    let valid_permissions = vec![
        "telemetry:write".to_string(),
        "telemetry:read".to_string(),
        "commands:receive".to_string(),
        "status:update".to_string(),
        "*".to_string(), // Wildcard permission
    ];

    for permission in &payload.permissions {
        if !valid_permissions.contains(permission) {
            return Err(AppError::ValidationError(
                format!("Invalid permission: {}. Valid permissions are: {}", 
                    permission, 
                    valid_permissions.join(", ")
                )
            ));
        }
    }

    // Create API key record
    let key_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (
            key_id, owner_user_id, key_name, key_hash, key_prefix, 
            rig_id, permissions, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        key_id,
        auth_user.user_id,
        payload.key_name,
        key_hash,
        key_prefix,
        payload.rig_id,
        &payload.permissions,
        payload.expires_at
    )
    .execute(state.db.pool())
    .await?;

    info!("New API key created: {} (user: {})", payload.key_name, auth_user.user_id);

    Ok(Json(ApiKeyResponse {
        key_id,
        key_name: payload.key_name,
        key_prefix,
        actual_key: Some(api_key), // Only return actual key on creation
        rig_id: payload.rig_id,
        permissions: payload.permissions,
        is_active: true,
        last_used: None,
        expires_at: payload.expires_at,
        created_at: chrono::Utc::now(),
    }))
}

/// Revoke (deactivate) an API key
pub async fn revoke_api_key(
    auth_user: AuthenticatedUser,
    Path(key_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    // Verify ownership and deactivate key
    let updated_rows = sqlx::query!(
        r#"
        UPDATE api_keys 
        SET is_active = false, updated_at = NOW()
        WHERE key_id = $1 AND owner_user_id = $2 AND is_active = true
        "#,
        key_id,
        auth_user.user_id
    )
    .execute(state.db.pool())
    .await?
    .rows_affected();

    if updated_rows == 0 {
        return Err(AppError::NotFound("API key not found".to_string()));
    }

    info!("API key revoked: {} (user: {})", key_id, auth_user.user_id);

    Ok(Json(serde_json::json!({
        "message": "API key revoked successfully",
        "key_id": key_id
    })))
}