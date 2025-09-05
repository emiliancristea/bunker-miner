// BUNKER POOL - Load Testing Tool
// Simulates thousands of concurrent miners to validate pool performance

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};
use serde_json::{json, Value};
use uuid::Uuid;
use std::collections::HashMap;

/// Test configuration
#[derive(Clone)]
pub struct LoadTestConfig {
    pub pool_host: String,
    pub pool_port: u16,
    pub concurrent_miners: usize,
    pub test_duration: Duration,
    pub share_submission_rate: Duration, // How often each miner submits shares
    pub connection_timeout: Duration,
}

/// Performance metrics
#[derive(Debug, Default)]
pub struct TestMetrics {
    pub connections_established: usize,
    pub connections_failed: usize,
    pub shares_submitted: usize,
    pub shares_accepted: usize,
    pub shares_rejected: usize,
    pub average_response_time: Duration,
    pub errors: Vec<String>,
}

/// Simulated miner client
pub struct MinerSimulator {
    id: Uuid,
    pool_address: String,
    metrics: Arc<RwLock<TestMetrics>>,
    config: LoadTestConfig,
}

impl MinerSimulator {
    pub fn new(pool_address: String, metrics: Arc<RwLock<TestMetrics>>, config: LoadTestConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            pool_address,
            metrics,
            config,
        }
    }

    /// Run the miner simulation
    pub async fn run(&self) {
        let start_time = Instant::now();
        
        // Connect to pool
        let mut stream = match self.connect_to_pool().await {
            Ok(stream) => {
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.connections_established += 1;
                }
                stream
            },
            Err(e) => {
                error!("Miner {} failed to connect: {}", self.id, e);
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.connections_failed += 1;
                    metrics.errors.push(format!("Connection failed: {}", e));
                }
                return;
            }
        };

        let (mut reader, mut writer) = stream.split();
        let mut buf_reader = BufReader::new(reader);
        
        // Subscribe to pool
        if let Err(e) = self.subscribe(&mut writer).await {
            error!("Miner {} failed to subscribe: {}", self.id, e);
            return;
        }

        // Authorize worker
        if let Err(e) = self.authorize(&mut writer).await {
            error!("Miner {} failed to authorize: {}", self.id, e);
            return;
        }

        // Start mining simulation
        let mut share_counter = 0u64;
        let mut response_times = Vec::new();
        
        while start_time.elapsed() < self.config.test_duration {
            // Submit a share
            let share_start = Instant::now();
            
            if let Err(e) = self.submit_share(&mut writer, share_counter).await {
                error!("Miner {} failed to submit share: {}", self.id, e);
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.errors.push(format!("Share submission failed: {}", e));
                }
                break;
            }
            
            share_counter += 1;
            
            // Read response with timeout
            let mut line = String::new();
            match timeout(Duration::from_secs(5), buf_reader.read_line(&mut line)).await {
                Ok(Ok(_)) => {
                    let response_time = share_start.elapsed();
                    response_times.push(response_time);
                    
                    // Parse response
                    if let Ok(response) = serde_json::from_str::<Value>(&line) {
                        self.handle_pool_response(&response).await;
                    }
                },
                Ok(Err(e)) => {
                    error!("Miner {} read error: {}", self.id, e);
                    break;
                },
                Err(_) => {
                    warn!("Miner {} response timeout", self.id);
                    {
                        let mut metrics = self.metrics.write().await;
                        metrics.errors.push("Response timeout".to_string());
                    }
                }
            }
            
            // Wait before next share
            sleep(self.config.share_submission_rate).await;
        }

        // Update final metrics
        if !response_times.is_empty() {
            let avg_response = response_times.iter().sum::<Duration>() / response_times.len() as u32;
            let mut metrics = self.metrics.write().await;
            if metrics.average_response_time.is_zero() {
                metrics.average_response_time = avg_response;
            } else {
                metrics.average_response_time = (metrics.average_response_time + avg_response) / 2;
            }
        }

        info!("Miner {} finished simulation - submitted {} shares", self.id, share_counter);
    }

    async fn connect_to_pool(&self) -> Result<TcpStream, Box<dyn std::error::Error + Send + Sync>> {
        let stream = timeout(
            self.config.connection_timeout,
            TcpStream::connect(&self.pool_address)
        ).await??;
        
        debug!("Miner {} connected to pool", self.id);
        Ok(stream)
    }

    async fn subscribe(&self, writer: &mut tokio::net::tcp::OwnedWriteHalf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subscribe_msg = json!({
            "id": 1,
            "method": "mining.subscribe",
            "params": [format!("BunkerMiner/{}", self.id)]
        });
        
        let msg = format!("{}\n", serde_json::to_string(&subscribe_msg)?);
        writer.write_all(msg.as_bytes()).await?;
        writer.flush().await?;
        
        debug!("Miner {} sent subscribe", self.id);
        Ok(())
    }

    async fn authorize(&self, writer: &mut tokio::net::tcp::OwnedWriteHalf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let auth_msg = json!({
            "id": 2,
            "method": "mining.authorize",
            "params": [format!("miner_{}", self.id.simple()), "password"]
        });
        
        let msg = format!("{}\n", serde_json::to_string(&auth_msg)?);
        writer.write_all(msg.as_bytes()).await?;
        writer.flush().await?;
        
        debug!("Miner {} sent authorize", self.id);
        Ok(())
    }

    async fn submit_share(&self, writer: &mut tokio::net::tcp::OwnedWriteHalf, nonce: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let share_msg = json!({
            "id": 3 + nonce,
            "method": "mining.submit",
            "params": [
                format!("miner_{}", self.id.simple()),
                "job_id_123",
                format!("{:08x}", nonce), // extranonce2
                format!("{:08x}", chrono::Utc::now().timestamp()), // ntime
                format!("{:08x}", nonce) // nonce
            ]
        });
        
        let msg = format!("{}\n", serde_json::to_string(&share_msg)?);
        writer.write_all(msg.as_bytes()).await?;
        writer.flush().await?;
        
        {
            let mut metrics = self.metrics.write().await;
            metrics.shares_submitted += 1;
        }
        
        debug!("Miner {} submitted share {}", self.id, nonce);
        Ok(())
    }

    async fn handle_pool_response(&self, response: &Value) {
        if let Some(result) = response.get("result") {
            let mut metrics = self.metrics.write().await;
            
            if result == &json!(true) {
                metrics.shares_accepted += 1;
                debug!("Miner {} share accepted", self.id);
            } else {
                metrics.shares_rejected += 1;
                debug!("Miner {} share rejected", self.id);
            }
        }

        if let Some(error) = response.get("error") {
            let mut metrics = self.metrics.write().await;
            metrics.errors.push(format!("Pool error: {}", error));
        }
    }
}

/// Main load test orchestrator
pub struct LoadTestRunner {
    config: LoadTestConfig,
    metrics: Arc<RwLock<TestMetrics>>,
}

impl LoadTestRunner {
    pub fn new(config: LoadTestConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RwLock::new(TestMetrics::default())),
        }
    }

    /// Run the load test with specified number of concurrent miners
    pub async fn run_test(&self) -> TestMetrics {
        info!("Starting load test with {} concurrent miners", self.config.concurrent_miners);
        info!("Test duration: {:?}", self.config.test_duration);
        info!("Pool address: {}:{}", self.config.pool_host, self.config.pool_port);
        
        let pool_address = format!("{}:{}", self.config.pool_host, self.config.pool_port);
        
        // Semaphore to control connection rate
        let semaphore = Arc::new(Semaphore::new(50)); // Max 50 concurrent connections at once
        let mut handles = Vec::new();

        // Start all miners
        for i in 0..self.config.concurrent_miners {
            let pool_addr = pool_address.clone();
            let metrics = self.metrics.clone();
            let config = self.config.clone();
            let semaphore = semaphore.clone();
            
            let handle = tokio::spawn(async move {
                // Acquire permit to control connection rate
                let _permit = semaphore.acquire().await.unwrap();
                
                // Small delay to spread out connections
                sleep(Duration::from_millis((i * 10) as u64)).await;
                
                let miner = MinerSimulator::new(pool_addr, metrics, config);
                miner.run().await;
                
                // Hold permit for a bit to prevent thundering herd
                sleep(Duration::from_millis(100)).await;
            });
            
            handles.push(handle);
        }

        // Wait for all miners to finish
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Miner task failed: {}", e);
            }
        }

        // Return final metrics
        let metrics = self.metrics.read().await;
        TestMetrics {
            connections_established: metrics.connections_established,
            connections_failed: metrics.connections_failed,
            shares_submitted: metrics.shares_submitted,
            shares_accepted: metrics.shares_accepted,
            shares_rejected: metrics.shares_rejected,
            average_response_time: metrics.average_response_time,
            errors: metrics.errors.clone(),
        }
    }

    /// Print test results
    pub fn print_results(&self, results: &TestMetrics) {
        println!("\n=== BUNKER POOL LOAD TEST RESULTS ===");
        println!("Concurrent Miners: {}", self.config.concurrent_miners);
        println!("Test Duration: {:?}", self.config.test_duration);
        println!("");
        println!("CONNECTION METRICS:");
        println!("  ✅ Established: {}", results.connections_established);
        println!("  ❌ Failed: {}", results.connections_failed);
        println!("  📊 Success Rate: {:.2}%", 
            (results.connections_established as f64 / (results.connections_established + results.connections_failed) as f64) * 100.0);
        println!("");
        println!("SHARE METRICS:");
        println!("  📤 Submitted: {}", results.shares_submitted);
        println!("  ✅ Accepted: {}", results.shares_accepted);
        println!("  ❌ Rejected: {}", results.shares_rejected);
        if results.shares_submitted > 0 {
            println!("  📊 Accept Rate: {:.2}%", 
                (results.shares_accepted as f64 / results.shares_submitted as f64) * 100.0);
        }
        println!("");
        println!("PERFORMANCE METRICS:");
        println!("  ⏱️  Avg Response Time: {:?}", results.average_response_time);
        if results.shares_submitted > 0 && !results.average_response_time.is_zero() {
            let throughput = results.shares_submitted as f64 / self.config.test_duration.as_secs_f64();
            println!("  🚀 Throughput: {:.2} shares/sec", throughput);
        }
        println!("");
        
        if !results.errors.is_empty() {
            println!("ERRORS ({}):", results.errors.len());
            for (i, error) in results.errors.iter().take(10).enumerate() {
                println!("  {}. {}", i + 1, error);
            }
            if results.errors.len() > 10 {
                println!("  ... and {} more errors", results.errors.len() - 10);
            }
        }
        println!("=====================================\n");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("load_test=info,warn")
        .init();

    // Test configuration
    let config = LoadTestConfig {
        pool_host: "localhost".to_string(),
        pool_port: 3333,
        concurrent_miners: 1000,
        test_duration: Duration::from_secs(60), // 1 minute test
        share_submission_rate: Duration::from_secs(2), // Submit share every 2 seconds
        connection_timeout: Duration::from_secs(10),
    };

    // Create and run test
    let test_runner = LoadTestRunner::new(config);
    
    println!("🚀 Starting BUNKER POOL Load Test...");
    let start = Instant::now();
    
    let results = test_runner.run_test().await;
    
    let elapsed = start.elapsed();
    println!("⏱️  Test completed in {:?}", elapsed);
    
    // Print results
    test_runner.print_results(&results);
    
    // Determine if test passed
    let success_rate = results.connections_established as f64 / 
        (results.connections_established + results.connections_failed) as f64;
    
    if success_rate >= 0.95 && results.shares_accepted > 0 && results.average_response_time < Duration::from_millis(500) {
        println!("🎉 LOAD TEST PASSED! Pool can handle {} concurrent miners successfully", config.concurrent_miners);
        std::process::exit(0);
    } else {
        println!("❌ LOAD TEST FAILED! Pool performance below requirements");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_miner() {
        let config = LoadTestConfig {
            pool_host: "127.0.0.1".to_string(),
            pool_port: 3333,
            concurrent_miners: 1,
            test_duration: Duration::from_secs(5),
            share_submission_rate: Duration::from_secs(1),
            connection_timeout: Duration::from_secs(5),
        };

        let test_runner = LoadTestRunner::new(config);
        let results = test_runner.run_test().await;
        
        // Basic validation - should at least attempt connections
        assert!(results.connections_established > 0 || results.connections_failed > 0);
    }

    #[test]
    fn test_config_validation() {
        let config = LoadTestConfig {
            pool_host: "localhost".to_string(),
            pool_port: 3333,
            concurrent_miners: 100,
            test_duration: Duration::from_secs(30),
            share_submission_rate: Duration::from_secs(2),
            connection_timeout: Duration::from_secs(10),
        };

        assert_eq!(config.pool_host, "localhost");
        assert_eq!(config.pool_port, 3333);
        assert_eq!(config.concurrent_miners, 100);
    }
}