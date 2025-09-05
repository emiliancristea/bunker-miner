use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User account model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub user_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Mining rig model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Rig {
    pub rig_id: Uuid,
    pub owner_user_id: Uuid,
    pub rig_name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub status: RigStatus,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Rig status enumeration
#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "rig_status", rename_all = "lowercase")]
pub enum RigStatus {
    Online,
    Offline,
    Mining,
    Idle,
    Error,
    Maintenance,
}

/// API key model for rig authentication
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_id: Uuid,
    pub owner_user_id: Uuid,
    pub key_name: String,
    pub key_hash: String,
    pub key_prefix: String, // First 8 characters for display
    pub rig_id: Option<Uuid>, // Optional association with a specific rig
    pub permissions: Vec<String>, // JSON array of permissions
    pub is_active: bool,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Rig telemetry data model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RigTelemetry {
    pub telemetry_id: Uuid,
    pub rig_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub algorithm: String,
    pub total_hashrate: f64,
    pub total_power: f64,
    pub avg_temperature: f64,
    pub device_count: i32,
    pub shares_accepted: i64,
    pub shares_rejected: i64,
    pub pool_url: String,
    pub profit_eur_day: Option<f64>,
    pub device_telemetry: serde_json::Value, // JSON array of per-device telemetry
}

/// Device telemetry within a rig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTelemetry {
    pub device_id: String,
    pub device_name: String,
    pub hashrate: f64,
    pub power: f64,
    pub temperature: f64,
    pub fan_speed: u32,
    pub utilization: u32,
    pub status: String,
}

/// Request/Response DTOs for API endpoints

/// User registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// User login request  
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Authentication response with JWT
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: UserProfile,
}

/// User profile information
#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub user_id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub rig_count: i64,
}

/// Rig registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRigRequest {
    pub rig_name: String,
    pub description: Option<String>,
    pub location: Option<String>,
}

/// Rig response with aggregated data
#[derive(Debug, Serialize)]
pub struct RigResponse {
    pub rig_id: Uuid,
    pub rig_name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub status: RigStatus,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub current_telemetry: Option<RigTelemetry>,
    pub is_connected: bool,
    pub uptime_hours: Option<f64>,
}

/// API key creation request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub key_name: String,
    pub rig_id: Option<Uuid>,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// API key response (with actual key only shown once)
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub key_id: Uuid,
    pub key_name: String,
    pub key_prefix: String,
    pub actual_key: Option<String>, // Only present in creation response
    pub rig_id: Option<Uuid>,
    pub permissions: Vec<String>,
    pub is_active: bool,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Command to send to a rig
#[derive(Debug, Serialize, Deserialize)]
pub struct RigCommand {
    pub command_id: Uuid,
    pub command_type: String,
    pub parameters: serde_json::Value,
    pub timeout_seconds: u32,
}

/// Command response from a rig
#[derive(Debug, Serialize, Deserialize)]
pub struct RigCommandResponse {
    pub command_id: Uuid,
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub execution_time_ms: u64,
}

/// WebSocket message types for rig communication
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RigMessage {
    /// Authentication message from rig to server
    Auth {
        api_key: String,
        rig_id: Option<Uuid>,
    },
    /// Authentication response from server to rig
    AuthResponse {
        success: bool,
        message: String,
        assigned_rig_id: Option<Uuid>,
    },
    /// Telemetry data from rig to server
    Telemetry {
        rig_id: Uuid,
        data: RigTelemetry,
    },
    /// Command from server to rig
    Command {
        command: RigCommand,
    },
    /// Command response from rig to server
    CommandResponse {
        response: RigCommandResponse,
    },
    /// Heartbeat/ping message
    Heartbeat {
        timestamp: DateTime<Utc>,
    },
    /// Error message
    Error {
        message: String,
        code: Option<String>,
    },
}

/// WebSocket message types for dashboard communication
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DashboardMessage {
    /// Subscribe to updates for specific rigs
    Subscribe {
        rig_ids: Vec<Uuid>,
    },
    /// Unsubscribe from updates
    Unsubscribe {
        rig_ids: Vec<Uuid>,
    },
    /// Real-time telemetry update
    TelemetryUpdate {
        rig_id: Uuid,
        data: RigTelemetry,
    },
    /// Rig status update
    StatusUpdate {
        rig_id: Uuid,
        status: RigStatus,
        timestamp: DateTime<Utc>,
    },
    /// Rig connected/disconnected
    ConnectionUpdate {
        rig_id: Uuid,
        connected: bool,
        timestamp: DateTime<Utc>,
    },
    /// Error message
    Error {
        message: String,
    },
}

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub email: String,
    pub exp: usize, // Expiration time
    pub iat: usize, // Issued at
    pub iss: String, // Issuer
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            user_id: user.user_id,
            email: user.email,
            created_at: user.created_at,
            rig_count: 0, // Will be populated by the handler
        }
    }
}