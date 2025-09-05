// BUNKER POOL - High-Performance Stratum Server
// TCP server capable of handling thousands of concurrent miner connections

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{
    connection::{ConnectionConfig, MinerConnection},
    job_manager::{CoinDaemonConfig, JobManager},
    protocol::{Algorithm, MiningJob, ShareSubmission},
};

/// Stratum server configuration
#[derive(Debug, Clone)]
pub struct StratumServerConfig {
    pub bind_address: SocketAddr,
    pub algorithm: Algorithm,
    pub max_connections: usize,
    pub connection_config: ConnectionConfig,
    pub coin_daemon: CoinDaemonConfig,
}

/// Stratum server statistics
#[derive(Debug, Default, Clone)]
pub struct StratumStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_shares: u64,
    pub valid_shares: u64,
    pub invalid_shares: u64,
    pub blocks_found: u64,
}

/// High-performance Stratum mining server
pub struct StratumServer {
    config: StratumServerConfig,
    stats: Arc<RwLock<StratumStats>>,
    active_connections: Arc<RwLock<HashMap<Uuid, Arc<RwLock<super::protocol::MinerSession>>>>>,
    job_manager: Arc<JobManager>,
    share_processor_sender: mpsc::Sender<(Uuid, ShareSubmission)>,
}

impl StratumServer {
    pub fn new(
        config: StratumServerConfig,
        share_processor_sender: mpsc::Sender<(Uuid, ShareSubmission)>,
    ) -> Self {
        // Create job broadcaster channel
        let (job_tx, job_rx) = mpsc::channel::<MiningJob>(1000);

        // Create job manager
        let job_manager = Arc::new(JobManager::new(
            config.coin_daemon.clone(),
            job_tx,
        ));

        Self {
            config,
            stats: Arc::new(RwLock::new(StratumStats::default())),
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            job_manager,
            share_processor_sender,
        }
    }

    /// Start the Stratum server
    pub async fn start(&self) -> Result<(), anyhow::Error> {
        info!("Starting BUNKER POOL Stratum server on {}", self.config.bind_address);
        info!("Algorithm: {:?}", self.config.algorithm);
        info!("Max connections: {}", self.config.max_connections);

        // Start job manager
        let job_manager = Arc::clone(&self.job_manager);
        tokio::spawn(async move {
            if let Err(e) = job_manager.start().await {
                error!("Job manager failed: {}", e);
            }
        });

        // Bind TCP listener
        let listener = TcpListener::bind(self.config.bind_address).await?;
        info!("Stratum server listening on {}", self.config.bind_address);

        // Connection cleanup task
        self.start_cleanup_task().await;

        // Statistics reporting task
        self.start_stats_task().await;

        // Accept connections loop
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    // Check connection limit
                    let connection_count = {
                        let connections = self.active_connections.read().await;
                        connections.len()
                    };

                    if connection_count >= self.config.max_connections {
                        warn!("Connection limit reached ({}/{}), rejecting connection from {}", 
                              connection_count, self.config.max_connections, addr);
                        let _ = stream.shutdown().await;
                        continue;
                    }

                    // Handle new connection
                    self.handle_new_connection(stream, addr).await;
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Handle new miner connection
    async fn handle_new_connection(&self, stream: TcpStream, addr: SocketAddr) {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_connections += 1;
            stats.active_connections += 1;
        }

        // Create disconnect channel
        let (disconnect_tx, mut disconnect_rx) = mpsc::channel::<Uuid>(1);

        // Create miner connection
        let connection = MinerConnection::new(
            stream,
            addr,
            self.config.algorithm,
            self.config.connection_config.clone(),
            mpsc::channel(100).0, // Job receiver (will be connected properly)
            self.share_processor_sender.clone(),
            disconnect_tx,
        );

        let session_id = {
            let session = connection.session.read().await;
            session.id
        };

        // Store connection
        {
            let mut connections = self.active_connections.write().await;
            connections.insert(session_id, Arc::clone(&connection.session));
        }

        // Handle connection in separate task
        let stats = Arc::clone(&self.stats);
        let connections = Arc::clone(&self.active_connections);
        tokio::spawn(async move {
            // Handle the connection
            connection.handle().await;

            // Connection ended, clean up
            {
                let mut conns = connections.write().await;
                conns.remove(&session_id);
            }

            {
                let mut stats = stats.write().await;
                if stats.active_connections > 0 {
                    stats.active_connections -= 1;
                }
            }

            info!("Connection {} closed", session_id);
        });

        // Handle disconnection notifications
        let connections_clone = Arc::clone(&self.active_connections);
        let stats_clone = Arc::clone(&self.stats);
        tokio::spawn(async move {
            while let Some(session_id) = disconnect_rx.recv().await {
                {
                    let mut connections = connections_clone.write().await;
                    connections.remove(&session_id);
                }

                {
                    let mut stats = stats_clone.write().await;
                    if stats.active_connections > 0 {
                        stats.active_connections -= 1;
                    }
                }

                info!("Miner {} disconnected", session_id);
            }
        });
    }

    /// Start connection cleanup task
    async fn start_cleanup_task(&self) {
        let connections = Arc::clone(&self.active_connections);
        let stats = Arc::clone(&self.stats);
        let max_idle_time = self.config.connection_config.max_idle_time;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                let now = chrono::Utc::now();
                let mut to_remove = Vec::new();

                {
                    let connections = connections.read().await;
                    for (session_id, session) in connections.iter() {
                        let session = session.read().await;
                        let idle_time = now.signed_duration_since(session.last_activity);

                        if idle_time.to_std().unwrap_or_default() > max_idle_time {
                            to_remove.push(*session_id);
                        }
                    }
                }

                if !to_remove.is_empty() {
                    let mut connections = connections.write().await;
                    let mut stats = stats.write().await;

                    for session_id in to_remove {
                        connections.remove(&session_id);
                        if stats.active_connections > 0 {
                            stats.active_connections -= 1;
                        }
                        info!("Cleaned up idle connection: {}", session_id);
                    }
                }
            }
        });
    }

    /// Start statistics reporting task
    async fn start_stats_task(&self) {
        let stats = Arc::clone(&self.stats);
        let connections = Arc::clone(&self.active_connections);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes

            loop {
                interval.tick().await;

                let (stats_snapshot, connection_count) = {
                    let stats = stats.read().await;
                    let connections = connections.read().await;
                    (stats.clone(), connections.len())
                };

                info!("BUNKER POOL Statistics:");
                info!("  Active connections: {}", connection_count);
                info!("  Total connections: {}", stats_snapshot.total_connections);
                info!("  Total shares: {}", stats_snapshot.total_shares);
                info!("  Valid shares: {}", stats_snapshot.valid_shares);
                info!("  Invalid shares: {}", stats_snapshot.invalid_shares);
                info!("  Blocks found: {}", stats_snapshot.blocks_found);

                if stats_snapshot.total_shares > 0 {
                    let accept_rate = (stats_snapshot.valid_shares as f64 / stats_snapshot.total_shares as f64) * 100.0;
                    info!("  Accept rate: {:.2}%", accept_rate);
                }
            }
        });
    }

    /// Get current server statistics
    pub async fn get_stats(&self) -> StratumStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Update share statistics
    pub async fn update_share_stats(&self, valid: bool, is_block: bool) {
        let mut stats = self.stats.write().await;
        stats.total_shares += 1;

        if valid {
            stats.valid_shares += 1;
            if is_block {
                stats.blocks_found += 1;
            }
        } else {
            stats.invalid_shares += 1;
        }
    }

    /// Broadcast difficulty change to all miners
    pub async fn broadcast_difficulty(&self, difficulty: f64) {
        let connections = self.active_connections.read().await;
        let notification = super::protocol::StratumNotification::set_difficulty(difficulty);
        
        if let Ok(message) = notification.to_json() {
            for (session_id, session) in connections.iter() {
                let session = session.read().await;
                if session.subscribed && session.authorized {
                    // In a real implementation, we would send this through the connection's write channel
                    info!("Would send difficulty {} to miner {}", difficulty, session_id);
                }
            }
        }
    }

    /// Get list of connected miners
    pub async fn get_connected_miners(&self) -> Vec<(Uuid, super::protocol::MinerSession)> {
        let connections = self.active_connections.read().await;
        let mut miners = Vec::new();

        for (session_id, session) in connections.iter() {
            let session = session.read().await;
            miners.push((*session_id, session.clone()));
        }

        miners
    }

    /// Kick miner by session ID
    pub async fn kick_miner(&self, session_id: Uuid) -> bool {
        let mut connections = self.active_connections.write().await;
        if connections.remove(&session_id).is_some() {
            info!("Kicked miner: {}", session_id);
            true
        } else {
            false
        }
    }

    /// Set difficulty for specific miner
    pub async fn set_miner_difficulty(&self, session_id: Uuid, difficulty: f64) -> bool {
        let connections = self.active_connections.read().await;
        if let Some(session) = connections.get(&session_id) {
            let mut session = session.write().await;
            session.difficulty = difficulty;
            info!("Set difficulty {} for miner {}", difficulty, session_id);
            true
        } else {
            false
        }
    }
}

impl Drop for StratumServer {
    fn drop(&mut self) {
        info!("BUNKER POOL Stratum server shutting down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_config() -> StratumServerConfig {
        StratumServerConfig {
            bind_address: "127.0.0.1:3333".parse().unwrap(),
            algorithm: Algorithm::Sha256,
            max_connections: 1000,
            connection_config: ConnectionConfig::default(),
            coin_daemon: CoinDaemonConfig {
                url: "http://localhost:8332".to_string(),
                username: "test".to_string(),
                password: "test".to_string(),
                algorithm: Algorithm::Sha256,
                block_poll_interval: Duration::from_secs(10),
                timeout: Duration::from_secs(30),
                coinbase_address: "1BunkerPoolTest".to_string(),
                extra_data: Some("BUNKER POOL".to_string()),
            },
        }
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = create_test_config();
        let (tx, _rx) = mpsc::channel(100);
        let server = StratumServer::new(config, tx);
        
        let stats = server.get_stats().await;
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_connections, 0);
    }

    #[tokio::test]
    async fn test_share_stats_update() {
        let config = create_test_config();
        let (tx, _rx) = mpsc::channel(100);
        let server = StratumServer::new(config, tx);
        
        server.update_share_stats(true, false).await;
        server.update_share_stats(false, false).await;
        server.update_share_stats(true, true).await;
        
        let stats = server.get_stats().await;
        assert_eq!(stats.total_shares, 3);
        assert_eq!(stats.valid_shares, 2);
        assert_eq!(stats.invalid_shares, 1);
        assert_eq!(stats.blocks_found, 1);
    }
}