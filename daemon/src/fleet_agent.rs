use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::FleetModeConfig;
use crate::telemetry::TelemetryData;

/// Fleet agent for connecting to centralized fleet management controller
pub struct FleetAgent {
    config: FleetModeConfig,
    telemetry_receiver: mpsc::Receiver<TelemetryData>,
    command_sender: broadcast::Sender<RemoteCommand>,
    connection_state: Arc<RwLock<ConnectionState>>,
    shutdown_receiver: broadcast::Receiver<()>,
}

/// Connection state tracking
#[derive(Debug, Clone, Default)]
pub struct ConnectionState {
    pub connected: bool,
    pub last_connection_attempt: Option<Instant>,
    pub retry_count: u32,
    pub assigned_rig_id: Option<Uuid>,
    pub last_heartbeat: Option<Instant>,
}

/// Remote command received from fleet controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCommand {
    pub command_id: Uuid,
    pub command_type: String,
    pub parameters: serde_json::Value,
    pub timeout_seconds: Option<u32>,
}

/// Command response to send back to controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub command_id: Uuid,
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub execution_time_ms: u64,
}

/// WebSocket message types for communication with fleet controller
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FleetMessage {
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
    Telemetry { rig_id: Uuid, data: TelemetryData },
    /// Command from server to rig
    Command { command: RemoteCommand },
    /// Command response from rig to server
    CommandResponse { response: CommandResponse },
    /// Heartbeat/ping message
    Heartbeat {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Error message
    Error {
        message: String,
        code: Option<String>,
    },
}

impl FleetAgent {
    /// Create a new fleet agent
    pub fn new(
        config: FleetModeConfig,
        telemetry_receiver: mpsc::Receiver<TelemetryData>,
        shutdown_receiver: broadcast::Receiver<()>,
    ) -> (Self, broadcast::Receiver<RemoteCommand>) {
        let (command_sender, command_receiver) = broadcast::channel(100);
        let connection_state = Arc::new(RwLock::new(ConnectionState::default()));

        let agent = Self {
            config,
            telemetry_receiver,
            command_sender,
            connection_state,
            shutdown_receiver,
        };

        (agent, command_receiver)
    }

    /// Start the fleet agent main loop
    pub async fn run(mut self) -> Result<()> {
        if !self.config.enabled {
            debug!("Fleet mode is disabled, not starting agent");
            return Ok(());
        }

        if self.config.api_key.is_none() {
            warn!("Fleet mode is enabled but no API key is configured");
            return Err(anyhow!("Fleet mode requires API key configuration"));
        }

        info!("🚀 Starting Fleet Agent for rig: {}", self.config.rig_name);
        info!("   Controller URL: {}", self.config.controller_url);
        info!("   Remote commands: {}", self.config.allow_remote_commands);

        loop {
            // Attempt connection to fleet controller
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("Fleet agent connection ended gracefully");
                    break;
                }
                Err(e) => {
                    error!("Fleet agent connection error: {}", e);

                    // Update connection state
                    {
                        let mut state = self.connection_state.write().await;
                        state.connected = false;
                        state.retry_count += 1;
                        state.last_connection_attempt = Some(Instant::now());
                    }

                    // Check if we should retry
                    if !self.should_retry().await {
                        error!("Maximum retry attempts reached, stopping fleet agent");
                        break;
                    }

                    // Wait before retrying with exponential backoff
                    let delay = self.calculate_retry_delay().await;
                    warn!("Retrying connection in {} seconds...", delay.as_secs());

                    tokio::select! {
                        _ = time::sleep(delay) => {}
                        _ = self.shutdown_receiver.recv() => {
                            info!("Shutdown signal received, stopping fleet agent");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Connect to fleet controller and run main communication loop
    async fn connect_and_run(&mut self) -> Result<()> {
        info!("🔌 Connecting to fleet controller...");

        // Attempt WebSocket connection
        let (ws_stream, response) = connect_async(&self.config.controller_url)
            .await
            .context("Failed to connect to fleet controller")?;

        info!(
            "✓ Connected to fleet controller (status: {})",
            response.status()
        );

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            state.connected = true;
            state.retry_count = 0;
            state.last_connection_attempt = Some(Instant::now());
        }

        // Send authentication message
        let auth_message = FleetMessage::Auth {
            api_key: self.config.api_key.as_ref().unwrap().clone(),
            rig_id: self
                .config
                .rig_id
                .as_ref()
                .and_then(|id| Uuid::parse_str(id).ok()),
        };

        let auth_json = serde_json::to_string(&auth_message)
            .context("Failed to serialize authentication message")?;

        ws_sender
            .send(Message::Text(auth_json))
            .await
            .context("Failed to send authentication message")?;

        // Heartbeats share the main WebSocket sender with telemetry and command responses.
        let telemetry_interval = Duration::from_secs(self.config.telemetry_interval_seconds as u64);
        let mut heartbeat_interval = time::interval(telemetry_interval);

        // Main message handling loop
        loop {
            tokio::select! {
                // Send periodic heartbeat
                _ = heartbeat_interval.tick() => {
                    let heartbeat_message = FleetMessage::Heartbeat {
                        timestamp: chrono::Utc::now(),
                    };

                    if let Ok(heartbeat_json) = serde_json::to_string(&heartbeat_message) {
                        if let Err(e) = ws_sender.send(Message::Text(heartbeat_json)).await {
                            error!("Failed to send heartbeat: {}", e);
                            break;
                        }
                    }
                }

                // Handle incoming WebSocket messages
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.handle_incoming_message(&text, &mut ws_sender).await {
                                error!("Error handling incoming message: {}", e);
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("WebSocket connection closed by server");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            break;
                        }
                        _ => {} // Ignore other message types
                    }
                }

                // Handle outgoing telemetry data
                Some(telemetry) = self.telemetry_receiver.recv() => {
                    if let Err(e) = self.send_telemetry(telemetry, &mut ws_sender).await {
                        error!("Failed to send telemetry: {}", e);
                    }
                }

                // Handle shutdown signal
                _ = self.shutdown_receiver.recv() => {
                    info!("Shutdown signal received, closing fleet connection");
                    let _ = ws_sender.send(Message::Close(None)).await;
                    break;
                }
            }
        }

        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            state.connected = false;
        }

        Ok(())
    }

    /// Handle incoming messages from fleet controller
    async fn handle_incoming_message(
        &self,
        message_text: &str,
        ws_sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    ) -> Result<()> {
        let message: FleetMessage =
            serde_json::from_str(message_text).context("Failed to parse incoming fleet message")?;

        match message {
            FleetMessage::AuthResponse {
                success,
                message,
                assigned_rig_id,
            } => {
                if success {
                    info!("✓ Fleet authentication successful: {}", message);
                    if let Some(rig_id) = assigned_rig_id {
                        let mut state = self.connection_state.write().await;
                        state.assigned_rig_id = Some(rig_id);
                        info!("   Assigned rig ID: {}", rig_id);
                    }
                } else {
                    error!("❌ Fleet authentication failed: {}", message);
                    return Err(anyhow!("Authentication failed: {}", message));
                }
            }

            FleetMessage::Command { command } => {
                if self.config.allow_remote_commands {
                    info!(
                        "📡 Received remote command: {} (ID: {})",
                        command.command_type, command.command_id
                    );

                    // Check if command is allowed
                    if !self.config.allowed_commands.contains(&command.command_type) {
                        warn!(
                            "Command {} is not in allowed commands list",
                            command.command_type
                        );
                        let response = CommandResponse {
                            command_id: command.command_id,
                            success: false,
                            message: format!("Command {} is not allowed", command.command_type),
                            data: None,
                            execution_time_ms: 0,
                        };
                        self.send_command_response(response, ws_sender).await?;
                        return Ok(());
                    }

                    // Forward command to local handlers
                    if let Err(e) = self.command_sender.send(command) {
                        error!("Failed to forward remote command: {}", e);
                    }
                } else {
                    warn!(
                        "Remote commands are disabled, ignoring command: {}",
                        command.command_type
                    );
                    let response = CommandResponse {
                        command_id: command.command_id,
                        success: false,
                        message: "Remote commands are disabled".to_string(),
                        data: None,
                        execution_time_ms: 0,
                    };
                    self.send_command_response(response, ws_sender).await?;
                }
            }

            FleetMessage::Error { message, code } => {
                error!("Fleet controller error: {} (code: {:?})", message, code);
                return Err(anyhow!("Fleet controller error: {}", message));
            }

            _ => {
                debug!("Received unexpected message type from fleet controller");
            }
        }

        Ok(())
    }

    /// Send telemetry data to fleet controller
    async fn send_telemetry(
        &self,
        telemetry: TelemetryData,
        ws_sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    ) -> Result<()> {
        let state = self.connection_state.read().await;
        if let Some(rig_id) = state.assigned_rig_id {
            let telemetry_message = FleetMessage::Telemetry {
                rig_id,
                data: telemetry,
            };

            let telemetry_json = serde_json::to_string(&telemetry_message)
                .context("Failed to serialize telemetry message")?;

            ws_sender
                .send(Message::Text(telemetry_json))
                .await
                .context("Failed to send telemetry message")?;

            debug!("📊 Sent telemetry to fleet controller");
        } else {
            debug!("No rig ID assigned yet, skipping telemetry");
        }

        Ok(())
    }

    /// Send command response back to fleet controller
    async fn send_command_response(
        &self,
        response: CommandResponse,
        ws_sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    ) -> Result<()> {
        let response_message = FleetMessage::CommandResponse { response };

        let response_json = serde_json::to_string(&response_message)
            .context("Failed to serialize command response")?;

        ws_sender
            .send(Message::Text(response_json))
            .await
            .context("Failed to send command response")?;

        debug!("📤 Sent command response to fleet controller");

        Ok(())
    }

    /// Check if we should retry connection
    async fn should_retry(&self) -> bool {
        let state = self.connection_state.read().await;

        if self.config.retry_settings.max_attempts == 0 {
            return true; // Unlimited retries
        }

        state.retry_count < self.config.retry_settings.max_attempts
    }

    /// Calculate delay before next retry attempt
    async fn calculate_retry_delay(&self) -> Duration {
        let state = self.connection_state.read().await;

        let delay_seconds = (self.config.retry_settings.initial_delay_seconds as f64
            * self
                .config
                .retry_settings
                .backoff_multiplier
                .powi(state.retry_count as i32))
        .min(self.config.retry_settings.max_delay_seconds as f64)
            as u64;

        Duration::from_secs(delay_seconds)
    }

    /// Get current connection state
    pub async fn get_connection_state(&self) -> ConnectionState {
        self.connection_state.read().await.clone()
    }
}
