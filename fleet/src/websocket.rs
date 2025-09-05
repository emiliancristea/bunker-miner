use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedRig,
    error::AppError,
    models::{DashboardMessage, RigMessage, RigTelemetry},
    AppState,
};

/// WebSocket connection manager
#[derive(Debug)]
pub struct WebSocketManager {
    /// Active rig connections: rig_id -> connection info
    rig_connections: DashMap<Uuid, RigConnection>,
    /// Active dashboard connections: connection_id -> dashboard info
    dashboard_connections: DashMap<u64, DashboardConnection>,
    /// Broadcast channel for telemetry updates
    telemetry_sender: broadcast::Sender<TelemetryBroadcast>,
    /// Connection ID counter
    next_connection_id: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct RigConnection {
    pub user_id: Uuid,
    pub rig_name: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub sender: tokio::sync::mpsc::UnboundedSender<Message>,
}

#[derive(Debug)]
pub struct DashboardConnection {
    pub user_id: Uuid,
    pub subscribed_rigs: Vec<Uuid>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub sender: tokio::sync::mpsc::UnboundedSender<Message>,
}

#[derive(Debug, Clone)]
pub struct TelemetryBroadcast {
    pub rig_id: Uuid,
    pub user_id: Uuid,
    pub data: RigTelemetry,
}

/// Query parameters for WebSocket connections
#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    pub api_key: Option<String>,
    pub token: Option<String>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (telemetry_sender, _) = broadcast::channel(10000);
        
        Self {
            rig_connections: DashMap::new(),
            dashboard_connections: DashMap::new(),
            telemetry_sender,
            next_connection_id: AtomicU64::new(1),
        }
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        ConnectionStats {
            rig_connections: self.rig_connections.len(),
            dashboard_connections: self.dashboard_connections.len(),
        }
    }

    /// Add a rig connection
    pub fn add_rig_connection(&self, rig_id: Uuid, connection: RigConnection) {
        info!("Rig {} connected (user: {})", rig_id, connection.user_id);
        self.rig_connections.insert(rig_id, connection);
    }

    /// Remove a rig connection
    pub fn remove_rig_connection(&self, rig_id: &Uuid) {
        if let Some((_, connection)) = self.rig_connections.remove(rig_id) {
            info!("Rig {} disconnected (user: {})", rig_id, connection.user_id);
        }
    }

    /// Add a dashboard connection
    pub fn add_dashboard_connection(&self, connection_id: u64, connection: DashboardConnection) {
        info!("Dashboard connection {} established (user: {})", connection_id, connection.user_id);
        self.dashboard_connections.insert(connection_id, connection);
    }

    /// Remove a dashboard connection
    pub fn remove_dashboard_connection(&self, connection_id: &u64) {
        if let Some((_, connection)) = self.dashboard_connections.remove(connection_id) {
            info!("Dashboard connection {} closed (user: {})", connection_id, connection.user_id);
        }
    }

    /// Broadcast telemetry to subscribed dashboard connections
    pub async fn broadcast_telemetry(&self, rig_id: Uuid, user_id: Uuid, data: RigTelemetry) {
        let broadcast = TelemetryBroadcast { rig_id, user_id, data: data.clone() };
        
        // Send to broadcast channel
        if let Err(e) = self.telemetry_sender.send(broadcast) {
            warn!("Failed to send telemetry broadcast: {}", e);
        }

        // Send directly to subscribed dashboard connections
        let message = DashboardMessage::TelemetryUpdate { rig_id, data };
        let message_json = match serde_json::to_string(&message) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize telemetry message: {}", e);
                return;
            }
        };

        for connection in self.dashboard_connections.iter() {
            let dashboard_conn = connection.value();
            
            // Only send to connections from the same user that are subscribed to this rig
            if dashboard_conn.user_id == user_id && dashboard_conn.subscribed_rigs.contains(&rig_id) {
                if let Err(e) = dashboard_conn.sender.send(Message::Text(message_json.clone())) {
                    warn!("Failed to send telemetry to dashboard connection {}: {}", connection.key(), e);
                }
            }
        }
    }

    /// Send a command to a specific rig
    pub async fn send_command_to_rig(&self, rig_id: Uuid, command: RigMessage) -> Result<(), AppError> {
        let connection = self.rig_connections
            .get(&rig_id)
            .ok_or_else(|| AppError::NotFound("Rig not connected".to_string()))?;

        let message_json = serde_json::to_string(&command)
            .map_err(|e| AppError::Serialization(e))?;

        connection.sender
            .send(Message::Text(message_json))
            .map_err(|_| AppError::WebSocket("Failed to send command to rig".to_string()))?;

        Ok(())
    }

    /// Get all rigs connected for a specific user
    pub fn get_user_rigs(&self, user_id: Uuid) -> Vec<Uuid> {
        self.rig_connections
            .iter()
            .filter(|entry| entry.value().user_id == user_id)
            .map(|entry| *entry.key())
            .collect()
    }

    /// Update rig heartbeat
    pub fn update_rig_heartbeat(&self, rig_id: &Uuid) {
        if let Some(mut connection) = self.rig_connections.get_mut(rig_id) {
            connection.last_heartbeat = chrono::Utc::now();
        }
    }

    /// Get next connection ID
    fn next_connection_id(&self) -> u64 {
        self.next_connection_id.fetch_add(1, Ordering::Relaxed)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionStats {
    pub rig_connections: usize,
    pub dashboard_connections: usize,
}

/// WebSocket handler for rig connections
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_rig_socket(socket, params, state))
}

/// WebSocket handler for dashboard connections
pub async fn dashboard_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_dashboard_socket(socket, params, state))
}

/// Handle rig WebSocket connections
async fn handle_rig_socket(socket: WebSocket, params: WebSocketQuery, state: AppState) {
    let api_key = match params.api_key {
        Some(key) => key,
        None => {
            error!("Rig WebSocket connection attempted without API key");
            return;
        }
    };

    // Authenticate the rig using API key
    let auth_rig = match AuthenticatedRig::from_api_key(
        state.db.pool(),
        &state.auth,
        &api_key,
    ).await {
        Ok(auth) => auth,
        Err(e) => {
            error!("Rig authentication failed: {}", e);
            return;
        }
    };

    info!("Rig authenticated successfully (user: {})", auth_rig.user_id);

    // Set up the WebSocket connection
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Determine the rig ID
    let rig_id = auth_rig.rig_id.unwrap_or_else(|| Uuid::new_v4());

    // Create connection info
    let connection = RigConnection {
        user_id: auth_rig.user_id,
        rig_name: format!("Rig-{}", rig_id),
        connected_at: chrono::Utc::now(),
        last_heartbeat: chrono::Utc::now(),
        sender: tx,
    };

    // Register the connection
    state.ws_manager.add_rig_connection(rig_id, connection);

    // Send authentication response
    let auth_response = RigMessage::AuthResponse {
        success: true,
        message: "Authentication successful".to_string(),
        assigned_rig_id: Some(rig_id),
    };

    if let Ok(msg_json) = serde_json::to_string(&auth_response) {
        if let Err(e) = ws_sender.send(Message::Text(msg_json)).await {
            error!("Failed to send auth response to rig: {}", e);
            return;
        }
    }

    // Handle outgoing messages
    let ws_manager_clone = state.ws_manager.clone();
    let rig_id_clone = rig_id;
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                error!("Failed to send message to rig {}: {}", rig_id_clone, e);
                break;
            }
        }
        ws_manager_clone.remove_rig_connection(&rig_id_clone);
    });

    // Handle incoming messages
    let db = state.db.clone();
    let ws_manager = state.ws_manager.clone();
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<RigMessage>(&text) {
                    Ok(rig_msg) => {
                        if let Err(e) = handle_rig_message(rig_id, rig_msg, &db, &ws_manager).await {
                            error!("Error handling rig message: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse rig message: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Rig {} WebSocket connection closed", rig_id);
                break;
            }
            Err(e) => {
                error!("WebSocket error for rig {}: {}", rig_id, e);
                break;
            }
            _ => {}
        }
    }

    // Clean up connection
    ws_manager.remove_rig_connection(&rig_id);
}

/// Handle dashboard WebSocket connections
async fn handle_dashboard_socket(socket: WebSocket, params: WebSocketQuery, state: AppState) {
    let token = match params.token {
        Some(token) => token,
        None => {
            error!("Dashboard WebSocket connection attempted without token");
            return;
        }
    };

    // Validate JWT token
    let claims = match state.auth.validate_token(&token) {
        Ok(claims) => claims,
        Err(e) => {
            error!("Dashboard authentication failed: {}", e);
            return;
        }
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            error!("Invalid user ID in token: {}", claims.sub);
            return;
        }
    };

    info!("Dashboard authenticated successfully (user: {})", user_id);

    // Set up the WebSocket connection
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let connection_id = state.ws_manager.next_connection_id();

    // Create connection info
    let connection = DashboardConnection {
        user_id,
        subscribed_rigs: Vec::new(),
        connected_at: chrono::Utc::now(),
        sender: tx,
    };

    // Register the connection
    state.ws_manager.add_dashboard_connection(connection_id, connection);

    // Handle outgoing messages
    let ws_manager_clone = state.ws_manager.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                error!("Failed to send message to dashboard {}: {}", connection_id, e);
                break;
            }
        }
        ws_manager_clone.remove_dashboard_connection(&connection_id);
    });

    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<DashboardMessage>(&text) {
                    Ok(dashboard_msg) => {
                        if let Err(e) = handle_dashboard_message(connection_id, dashboard_msg, &state.ws_manager).await {
                            error!("Error handling dashboard message: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse dashboard message: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Dashboard {} WebSocket connection closed", connection_id);
                break;
            }
            Err(e) => {
                error!("WebSocket error for dashboard {}: {}", connection_id, e);
                break;
            }
            _ => {}
        }
    }

    // Clean up connection
    state.ws_manager.remove_dashboard_connection(&connection_id);
}

/// Handle messages from rigs
async fn handle_rig_message(
    rig_id: Uuid,
    message: RigMessage,
    db: &crate::database::DatabasePool,
    ws_manager: &WebSocketManager,
) -> Result<(), AppError> {
    match message {
        RigMessage::Telemetry { data, .. } => {
            // Store telemetry in database
            sqlx::query!(
                r#"
                INSERT INTO rig_telemetry (
                    rig_id, algorithm, total_hashrate, total_power, avg_temperature,
                    device_count, shares_accepted, shares_rejected, pool_url,
                    profit_eur_day, device_telemetry
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
                rig_id,
                data.algorithm,
                data.total_hashrate,
                data.total_power,
                data.avg_temperature,
                data.device_count,
                data.shares_accepted,
                data.shares_rejected,
                data.pool_url,
                data.profit_eur_day,
                data.device_telemetry
            )
            .execute(db.pool())
            .await?;

            // Get the user ID for this rig
            let user_id = sqlx::query_scalar!(
                "SELECT owner_user_id FROM rigs WHERE rig_id = $1",
                rig_id
            )
            .fetch_optional(db.pool())
            .await?
            .ok_or_else(|| AppError::NotFound("Rig not found".to_string()))?;

            // Broadcast to dashboard connections
            ws_manager.broadcast_telemetry(rig_id, user_id, data).await;
        }
        RigMessage::Heartbeat { .. } => {
            ws_manager.update_rig_heartbeat(&rig_id);
        }
        RigMessage::CommandResponse { response } => {
            info!("Received command response from rig {}: {:?}", rig_id, response);
            // Handle command response (could store in DB or forward to dashboard)
        }
        _ => {
            warn!("Unexpected message type from rig {}: {:?}", rig_id, message);
        }
    }

    Ok(())
}

/// Handle messages from dashboard connections
async fn handle_dashboard_message(
    connection_id: u64,
    message: DashboardMessage,
    ws_manager: &WebSocketManager,
) -> Result<(), AppError> {
    match message {
        DashboardMessage::Subscribe { rig_ids } => {
            if let Some(mut connection) = ws_manager.dashboard_connections.get_mut(&connection_id) {
                connection.subscribed_rigs = rig_ids;
                info!("Dashboard {} subscribed to {} rigs", connection_id, connection.subscribed_rigs.len());
            }
        }
        DashboardMessage::Unsubscribe { rig_ids } => {
            if let Some(mut connection) = ws_manager.dashboard_connections.get_mut(&connection_id) {
                connection.subscribed_rigs.retain(|id| !rig_ids.contains(id));
                info!("Dashboard {} unsubscribed from rigs", connection_id);
            }
        }
        _ => {
            warn!("Unexpected message type from dashboard {}: {:?}", connection_id, message);
        }
    }

    Ok(())
}