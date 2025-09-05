use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{AppError, AppResult},
    models::{
        RegisterRigRequest, Rig, RigCommand, RigCommandResponse, RigResponse, 
        RigStatus, RigTelemetry, RigMessage
    },
    AppState,
};

/// Query parameters for rig listing
#[derive(Debug, Deserialize)]
pub struct ListRigsQuery {
    pub status: Option<RigStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for telemetry
#[derive(Debug, Deserialize)]
pub struct TelemetryQuery {
    pub rig_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

/// Response for telemetry queries
#[derive(Debug, Serialize)]
pub struct TelemetryResponse {
    pub data: Vec<RigTelemetry>,
    pub total_count: i64,
    pub has_more: bool,
}

/// List user's rigs
pub async fn list_rigs(
    auth_user: AuthenticatedUser,
    Query(query): Query<ListRigsQuery>,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<RigResponse>>> {
    let limit = query.limit.unwrap_or(50).min(100); // Max 100 rigs per request
    let offset = query.offset.unwrap_or(0);

    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT * FROM rigs WHERE owner_user_id = "
    );
    query_builder.push_bind(auth_user.user_id);
    query_builder.push(" AND is_active = true");

    if let Some(status) = query.status {
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    query_builder.push(" ORDER BY created_at DESC LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let rigs = query_builder
        .build_query_as::<Rig>()
        .fetch_all(state.db.pool())
        .await?;

    // Get current telemetry for each rig
    let mut rig_responses = Vec::new();
    let connected_rigs = state.ws_manager.get_user_rigs(auth_user.user_id);

    for rig in rigs {
        // Get latest telemetry
        let current_telemetry = sqlx::query_as!(
            RigTelemetry,
            r#"
            SELECT * FROM rig_telemetry 
            WHERE rig_id = $1 
            ORDER BY timestamp DESC 
            LIMIT 1
            "#,
            rig.rig_id
        )
        .fetch_optional(state.db.pool())
        .await?;

        // Calculate uptime
        let uptime_hours = if let Some(last_seen) = rig.last_seen {
            Some((Utc::now() - last_seen).num_minutes() as f64 / 60.0)
        } else {
            None
        };

        rig_responses.push(RigResponse {
            rig_id: rig.rig_id,
            rig_name: rig.rig_name,
            description: rig.description,
            location: rig.location,
            status: rig.status,
            last_seen: rig.last_seen,
            created_at: rig.created_at,
            current_telemetry,
            is_connected: connected_rigs.contains(&rig.rig_id),
            uptime_hours,
        });
    }

    Ok(Json(rig_responses))
}

/// Register a new rig
pub async fn register_rig(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<RegisterRigRequest>,
) -> AppResult<Json<RigResponse>> {
    // Validate input
    if payload.rig_name.trim().is_empty() {
        return Err(AppError::ValidationError("Rig name is required".to_string()));
    }

    // Check if rig name already exists for this user
    let existing_rig = sqlx::query!(
        "SELECT rig_id FROM rigs WHERE owner_user_id = $1 AND rig_name = $2 AND is_active = true",
        auth_user.user_id,
        payload.rig_name
    )
    .fetch_optional(state.db.pool())
    .await?;

    if existing_rig.is_some() {
        return Err(AppError::Conflict("Rig with this name already exists".to_string()));
    }

    // Create new rig
    let rig_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO rigs (rig_id, owner_user_id, rig_name, description, location)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        rig_id,
        auth_user.user_id,
        payload.rig_name,
        payload.description,
        payload.location
    )
    .execute(state.db.pool())
    .await?;

    // Fetch the created rig
    let rig = sqlx::query_as!(
        Rig,
        "SELECT * FROM rigs WHERE rig_id = $1",
        rig_id
    )
    .fetch_one(state.db.pool())
    .await?;

    info!("New rig registered: {} (user: {})", rig.rig_name, auth_user.user_id);

    Ok(Json(RigResponse {
        rig_id: rig.rig_id,
        rig_name: rig.rig_name,
        description: rig.description,
        location: rig.location,
        status: rig.status,
        last_seen: rig.last_seen,
        created_at: rig.created_at,
        current_telemetry: None,
        is_connected: false,
        uptime_hours: None,
    }))
}

/// Get specific rig details
pub async fn get_rig(
    auth_user: AuthenticatedUser,
    Path(rig_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<RigResponse>> {
    // Fetch rig and verify ownership
    let rig = sqlx::query_as!(
        Rig,
        "SELECT * FROM rigs WHERE rig_id = $1 AND owner_user_id = $2 AND is_active = true",
        rig_id,
        auth_user.user_id
    )
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| AppError::NotFound("Rig not found".to_string()))?;

    // Get latest telemetry
    let current_telemetry = sqlx::query_as!(
        RigTelemetry,
        r#"
        SELECT * FROM rig_telemetry 
        WHERE rig_id = $1 
        ORDER BY timestamp DESC 
        LIMIT 1
        "#,
        rig_id
    )
    .fetch_optional(state.db.pool())
    .await?;

    // Check if rig is connected
    let connected_rigs = state.ws_manager.get_user_rigs(auth_user.user_id);
    let is_connected = connected_rigs.contains(&rig_id);

    // Calculate uptime
    let uptime_hours = if let Some(last_seen) = rig.last_seen {
        Some((Utc::now() - last_seen).num_minutes() as f64 / 60.0)
    } else {
        None
    };

    Ok(Json(RigResponse {
        rig_id: rig.rig_id,
        rig_name: rig.rig_name,
        description: rig.description,
        location: rig.location,
        status: rig.status,
        last_seen: rig.last_seen,
        created_at: rig.created_at,
        current_telemetry,
        is_connected,
        uptime_hours,
    }))
}

/// Update rig information
pub async fn update_rig(
    auth_user: AuthenticatedUser,
    Path(rig_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<RegisterRigRequest>,
) -> AppResult<Json<RigResponse>> {
    // Validate input
    if payload.rig_name.trim().is_empty() {
        return Err(AppError::ValidationError("Rig name is required".to_string()));
    }

    // Verify ownership and update rig
    let updated_rows = sqlx::query!(
        r#"
        UPDATE rigs 
        SET rig_name = $1, description = $2, location = $3, updated_at = NOW()
        WHERE rig_id = $4 AND owner_user_id = $5 AND is_active = true
        "#,
        payload.rig_name,
        payload.description,
        payload.location,
        rig_id,
        auth_user.user_id
    )
    .execute(state.db.pool())
    .await?
    .rows_affected();

    if updated_rows == 0 {
        return Err(AppError::NotFound("Rig not found".to_string()));
    }

    // Fetch updated rig
    let rig = sqlx::query_as!(
        Rig,
        "SELECT * FROM rigs WHERE rig_id = $1",
        rig_id
    )
    .fetch_one(state.db.pool())
    .await?;

    info!("Rig updated: {} (user: {})", rig.rig_name, auth_user.user_id);

    Ok(Json(RigResponse {
        rig_id: rig.rig_id,
        rig_name: rig.rig_name,
        description: rig.description,
        location: rig.location,
        status: rig.status,
        last_seen: rig.last_seen,
        created_at: rig.created_at,
        current_telemetry: None,
        is_connected: false,
        uptime_hours: None,
    }))
}

/// Delete (deactivate) a rig
pub async fn delete_rig(
    auth_user: AuthenticatedUser,
    Path(rig_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    // Verify ownership and deactivate rig
    let updated_rows = sqlx::query!(
        r#"
        UPDATE rigs 
        SET is_active = false, updated_at = NOW()
        WHERE rig_id = $1 AND owner_user_id = $2 AND is_active = true
        "#,
        rig_id,
        auth_user.user_id
    )
    .execute(state.db.pool())
    .await?
    .rows_affected();

    if updated_rows == 0 {
        return Err(AppError::NotFound("Rig not found".to_string()));
    }

    // Also deactivate associated API keys
    sqlx::query!(
        r#"
        UPDATE api_keys 
        SET is_active = false, updated_at = NOW()
        WHERE rig_id = $1
        "#,
        rig_id
    )
    .execute(state.db.pool())
    .await?;

    info!("Rig deleted: {} (user: {})", rig_id, auth_user.user_id);

    Ok(Json(serde_json::json!({
        "message": "Rig deleted successfully",
        "rig_id": rig_id
    })))
}

/// Send command to a rig
pub async fn send_command(
    auth_user: AuthenticatedUser,
    Path(rig_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<RigCommand>,
) -> AppResult<Json<serde_json::Value>> {
    // Verify ownership
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

    // Create command message
    let command_message = RigMessage::Command { command: payload };

    // Send command to rig via WebSocket
    state.ws_manager.send_command_to_rig(rig_id, command_message).await?;

    info!("Command sent to rig: {} (user: {})", rig_id, auth_user.user_id);

    Ok(Json(serde_json::json!({
        "message": "Command sent successfully",
        "rig_id": rig_id
    })))
}

/// Get telemetry data
pub async fn get_telemetry(
    auth_user: AuthenticatedUser,
    Query(query): Query<TelemetryQuery>,
    State(state): State<AppState>,
) -> AppResult<Json<TelemetryResponse>> {
    let limit = query.limit.unwrap_or(100).min(1000); // Max 1000 records per request
    let from = query.from.unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
    let to = query.to.unwrap_or_else(|| Utc::now());

    let mut conditions = vec!["r.owner_user_id = $1".to_string()];
    let mut bind_count = 1;

    // Add rig filter if specified
    if query.rig_id.is_some() {
        bind_count += 1;
        conditions.push(format!("rt.rig_id = ${}", bind_count));
    }

    // Add time range filters
    bind_count += 1;
    conditions.push(format!("rt.timestamp >= ${}", bind_count));
    bind_count += 1;
    conditions.push(format!("rt.timestamp <= ${}", bind_count));

    let where_clause = conditions.join(" AND ");
    let query_sql = format!(
        r#"
        SELECT rt.* FROM rig_telemetry rt
        JOIN rigs r ON rt.rig_id = r.rig_id
        WHERE {}
        ORDER BY rt.timestamp DESC
        LIMIT ${}
        "#,
        where_clause,
        bind_count + 1
    );

    let mut query_builder = sqlx::query_as::<_, RigTelemetry>(&query_sql);
    query_builder = query_builder.bind(auth_user.user_id);

    if let Some(rig_id) = query.rig_id {
        query_builder = query_builder.bind(rig_id);
    }

    query_builder = query_builder.bind(from);
    query_builder = query_builder.bind(to);
    query_builder = query_builder.bind(limit + 1); // Get one extra to check if there are more

    let mut data = query_builder.fetch_all(state.db.pool()).await?;
    let has_more = data.len() > limit as usize;
    if has_more {
        data.pop(); // Remove the extra record
    }

    // Get total count for the time range
    let count_sql = format!(
        r#"
        SELECT COUNT(*) FROM rig_telemetry rt
        JOIN rigs r ON rt.rig_id = r.rig_id
        WHERE {}
        "#,
        where_clause
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    count_query = count_query.bind(auth_user.user_id);

    if let Some(rig_id) = query.rig_id {
        count_query = count_query.bind(rig_id);
    }

    count_query = count_query.bind(from);
    count_query = count_query.bind(to);

    let total_count = count_query.fetch_one(state.db.pool()).await?;

    Ok(Json(TelemetryResponse {
        data,
        total_count,
        has_more,
    }))
}