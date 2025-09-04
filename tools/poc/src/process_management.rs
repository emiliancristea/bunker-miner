/*!
 * BUNKER MINER - Process Management PoC
 * 
 * This PoC validates our ability to robustly control and monitor third-party
 * mining software as child processes using tokio's async process management.
 * 
 * Success Criteria:
 * - Start and stop mining processes reliably
 * - Parse real-time stdout/stderr for hashrate and share data
 * - Handle process crashes and unexpected termination
 * - Provide secure process isolation and resource monitoring
 */

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Arg, Command};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::Mutex;
use tokio::time::{interval, timeout};
use tracing::{error, info, warn, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerStats {
    pub hashrate_hs: f64,
    pub shares_accepted: u64,
    pub shares_rejected: u64,
    pub uptime_seconds: u64,
    pub last_share_time: Option<chrono::DateTime<Utc>>,
    pub error_count: u64,
    pub temperature_c: Option<u32>,
    pub power_usage_w: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: Option<u32>,
    pub command: String,
    pub args: Vec<String>,
    pub status: ProcessStatus,
    pub start_time: chrono::DateTime<Utc>,
    pub last_output_time: Option<chrono::DateTime<Utc>>,
    pub stats: MinerStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Starting,
    Running,
    Stopped,
    Crashed,
    Timeout,
    Unknown,
}

pub struct MinerProcessManager {
    process: Arc<Mutex<Option<Child>>>,
    info: Arc<Mutex<ProcessInfo>>,
    output_patterns: HashMap<String, Regex>,
}

impl MinerProcessManager {
    pub fn new(command: String, args: Vec<String>) -> Self {
        let mut output_patterns = HashMap::new();
        
        // XMRig patterns (most common Monero miner)
        output_patterns.insert(
            "xmrig_hashrate".to_string(),
            Regex::new(r"speed\s+10s/60s/15m\s+([\d.]+)\s*([KMG]?)H/s").unwrap()
        );
        output_patterns.insert(
            "xmrig_accepted".to_string(),
            Regex::new(r"\[POOL\]\s+accepted\s+\((\d+)/(\d+)\)").unwrap()
        );
        output_patterns.insert(
            "xmrig_rejected".to_string(),
            Regex::new(r"\[POOL\]\s+rejected\s+\((\d+)/(\d+)\)").unwrap()
        );
        
        // lolMiner patterns (popular GPU miner)
        output_patterns.insert(
            "lolminer_hashrate".to_string(),
            Regex::new(r"Total\s+([\d.]+)\s*([KMG]?)H/s").unwrap()
        );
        output_patterns.insert(
            "lolminer_shares".to_string(),
            Regex::new(r"Shares:\s+A:(\d+)\s+R:(\d+)").unwrap()
        );
        
        // T-Rex patterns (NVIDIA GPU miner)
        output_patterns.insert(
            "trex_hashrate".to_string(),
            Regex::new(r"([\d.]+)\s*([KMG]?)H/s").unwrap()
        );
        
        // Generic error patterns
        output_patterns.insert(
            "error_pattern".to_string(),
            Regex::new(r"(?i)(error|failed|exception|crash)").unwrap()
        );
        output_patterns.insert(
            "warning_pattern".to_string(),
            Regex::new(r"(?i)(warning|warn)").unwrap()
        );

        let process_info = ProcessInfo {
            pid: None,
            command: command.clone(),
            args: args.clone(),
            status: ProcessStatus::Stopped,
            start_time: Utc::now(),
            last_output_time: None,
            stats: MinerStats {
                hashrate_hs: 0.0,
                shares_accepted: 0,
                shares_rejected: 0,
                uptime_seconds: 0,
                last_share_time: None,
                error_count: 0,
                temperature_c: None,
                power_usage_w: None,
            },
        };

        Self {
            process: Arc::new(Mutex::new(None)),
            info: Arc::new(Mutex::new(process_info)),
            output_patterns,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut process_guard = self.process.lock().await;
        let mut info_guard = self.info.lock().await;

        if process_guard.is_some() {
            warn!("Process is already running");
            return Ok(());
        }

        info!("Starting miner process: {} {:?}", info_guard.command, info_guard.args);

        let mut cmd = TokioCommand::new(&info_guard.command);
        cmd.args(&info_guard.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd.spawn()
            .context("Failed to start miner process")?;

        let pid = child.id();
        info!("Miner process started with PID: {:?}", pid);

        info_guard.pid = pid;
        info_guard.status = ProcessStatus::Starting;
        info_guard.start_time = Utc::now();

        // Take stdout and stderr for monitoring
        let stdout = child.stdout.take().context("Failed to get stdout")?;
        let stderr = child.stderr.take().context("Failed to get stderr")?;

        *process_guard = Some(child);
        drop(process_guard);
        drop(info_guard);

        // Start output monitoring tasks
        self.monitor_output(stdout, "stdout").await;
        self.monitor_output(stderr, "stderr").await;

        // Start process monitoring task
        self.monitor_process().await;

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut process_guard = self.process.lock().await;
        let mut info_guard = self.info.lock().await;

        if let Some(mut child) = process_guard.take() {
            info!("Stopping miner process PID: {:?}", child.id());

            // Try graceful termination first
            if let Err(e) = child.kill().await {
                warn!("Failed to kill process gracefully: {}", e);
            }

            // Wait for process to exit with timeout
            let exit_result = timeout(Duration::from_secs(10), child.wait()).await;
            
            match exit_result {
                Ok(Ok(status)) => {
                    info!("Process exited with status: {}", status);
                    info_guard.status = ProcessStatus::Stopped;
                }
                Ok(Err(e)) => {
                    error!("Error waiting for process: {}", e);
                    info_guard.status = ProcessStatus::Unknown;
                }
                Err(_) => {
                    warn!("Process termination timed out");
                    info_guard.status = ProcessStatus::Timeout;
                }
            }
        } else {
            warn!("No process to stop");
        }

        info_guard.pid = None;
        Ok(())
    }

    pub async fn get_info(&self) -> ProcessInfo {
        self.info.lock().await.clone()
    }

    pub async fn is_running(&self) -> bool {
        let process_guard = self.process.lock().await;
        process_guard.is_some()
    }

    async fn monitor_output<R>(&self, reader: R, stream_name: &str)
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let info = Arc::clone(&self.info);
        let patterns = self.output_patterns.clone();
        let stream_name = stream_name.to_string();

        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("[{}] {}", stream_name, line);
                
                let mut info_guard = info.lock().await;
                info_guard.last_output_time = Some(Utc::now());

                // Parse line for relevant mining data
                Self::parse_output_line(&line, &mut info_guard, &patterns).await;
                drop(info_guard);
            }
            
            info!("Output monitoring for {} stream ended", stream_name);
        });
    }

    async fn monitor_process(&self) {
        let process = Arc::clone(&self.process);
        let info = Arc::clone(&self.info);

        tokio::spawn(async move {
            let mut check_interval = interval(Duration::from_secs(5));
            
            loop {
                check_interval.tick().await;
                
                let mut process_guard = process.lock().await;
                let mut info_guard = info.lock().await;
                
                if let Some(child) = process_guard.as_mut() {
                    // Check if process is still alive
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            info!("Process exited with status: {}", status);
                            info_guard.status = if status.success() {
                                ProcessStatus::Stopped
                            } else {
                                ProcessStatus::Crashed
                            };
                            *process_guard = None;
                            break;
                        }
                        Ok(None) => {
                            // Process still running
                            if matches!(info_guard.status, ProcessStatus::Starting) {
                                info_guard.status = ProcessStatus::Running;
                                info!("Process confirmed running");
                            }
                            
                            // Update uptime
                            info_guard.stats.uptime_seconds = 
                                (Utc::now() - info_guard.start_time).num_seconds() as u64;
                        }
                        Err(e) => {
                            error!("Error checking process status: {}", e);
                            info_guard.status = ProcessStatus::Unknown;
                        }
                    }
                } else {
                    // No process to monitor
                    break;
                }
                
                drop(process_guard);
                drop(info_guard);
            }
            
            info!("Process monitoring ended");
        });
    }

    async fn parse_output_line(
        line: &str, 
        info: &mut ProcessInfo, 
        patterns: &HashMap<String, Regex>
    ) {
        // Check for hashrate patterns
        if let Some(hashrate_regex) = patterns.get("xmrig_hashrate") {
            if let Some(captures) = hashrate_regex.captures(line) {
                if let Ok(rate) = captures[1].parse::<f64>() {
                    let multiplier = match captures.get(2).map(|m| m.as_str()) {
                        Some("K") => 1_000.0,
                        Some("M") => 1_000_000.0,
                        Some("G") => 1_000_000_000.0,
                        _ => 1.0,
                    };
                    info.stats.hashrate_hs = rate * multiplier;
                    debug!("Updated hashrate: {} H/s", info.stats.hashrate_hs);
                }
            }
        }

        // Check for accepted shares
        if let Some(accepted_regex) = patterns.get("xmrig_accepted") {
            if let Some(captures) = accepted_regex.captures(line) {
                if let Ok(accepted) = captures[1].parse::<u64>() {
                    info.stats.shares_accepted = accepted;
                    info.stats.last_share_time = Some(Utc::now());
                    debug!("Updated accepted shares: {}", accepted);
                }
            }
        }

        // Check for rejected shares
        if let Some(rejected_regex) = patterns.get("xmrig_rejected") {
            if let Some(captures) = rejected_regex.captures(line) {
                if let Ok(rejected) = captures[2].parse::<u64>() {
                    info.stats.shares_rejected = rejected;
                    debug!("Updated rejected shares: {}", rejected);
                }
            }
        }

        // Check for errors
        if let Some(error_regex) = patterns.get("error_pattern") {
            if error_regex.is_match(line) {
                info.stats.error_count += 1;
                warn!("Error detected in miner output: {}", line);
            }
        }
    }
}

async fn run_process_test(miner_command: String, miner_args: Vec<String>) -> Result<()> {
    info!("Starting Process Management PoC test...");
    
    let manager = MinerProcessManager::new(miner_command, miner_args);
    
    // Start the process
    manager.start().await
        .context("Failed to start miner process")?;
    
    info!("Process started successfully, monitoring for 30 seconds...");
    
    // Monitor for 30 seconds
    for i in 1..=6 {
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        let info = manager.get_info().await;
        println!("\n=== Status Update {} ===", i);
        println!("Status: {:?}", info.status);
        println!("PID: {:?}", info.pid);
        println!("Uptime: {}s", info.stats.uptime_seconds);
        println!("Hashrate: {:.2} H/s", info.stats.hashrate_hs);
        println!("Shares - Accepted: {}, Rejected: {}", 
                 info.stats.shares_accepted, info.stats.shares_rejected);
        println!("Errors: {}", info.stats.error_count);
        
        if !manager.is_running().await {
            warn!("Process is no longer running!");
            break;
        }
    }
    
    // Test graceful shutdown
    info!("Testing graceful shutdown...");
    manager.stop().await
        .context("Failed to stop miner process")?;
    
    let final_info = manager.get_info().await;
    println!("\n=== Final Status ===");
    println!("Status: {:?}", final_info.status);
    println!("Final Hashrate: {:.2} H/s", final_info.stats.hashrate_hs);
    println!("Total Shares - Accepted: {}, Rejected: {}", 
             final_info.stats.shares_accepted, final_info.stats.shares_rejected);
    
    Ok(())
}

async fn run_crash_simulation() -> Result<()> {
    info!("Starting crash simulation test...");
    
    // Use a command that will fail quickly for testing
    let manager = MinerProcessManager::new(
        "nonexistent_miner".to_string(),
        vec!["--fake-arg".to_string()]
    );
    
    // This should fail to start
    match manager.start().await {
        Ok(_) => {
            warn!("Expected process start to fail, but it succeeded");
            // Wait a bit to see if it crashes
            tokio::time::sleep(Duration::from_secs(5)).await;
            let info = manager.get_info().await;
            println!("Process status after expected failure: {:?}", info.status);
            manager.stop().await?;
        }
        Err(e) => {
            info!("Process failed to start as expected: {}", e);
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("BUNKER MINER - Process Management PoC");
    
    let matches = Command::new("process-management")
        .about("BUNKER MINER Process Management Proof of Concept")
        .arg(
            Arg::new("miner")
                .long("miner")
                .help("Path to miner executable")
                .default_value("echo")
                .value_name("PATH")
        )
        .arg(
            Arg::new("args")
                .long("args")
                .help("Miner arguments (comma-separated)")
                .default_value("Mining simulation output...")
                .value_name("ARGS")
        )
        .arg(
            Arg::new("test-crash")
                .long("test-crash")
                .help("Test crash handling with invalid process")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    if matches.get_flag("test-crash") {
        run_crash_simulation().await?;
    } else {
        let miner_command = matches.get_one::<String>("miner").unwrap().clone();
        let args_str = matches.get_one::<String>("args").unwrap();
        let miner_args: Vec<String> = args_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        
        run_process_test(miner_command, miner_args).await?;
    }
    
    info!("✅ Process Management PoC completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_manager_creation() {
        let manager = MinerProcessManager::new(
            "test_command".to_string(),
            vec!["arg1".to_string(), "arg2".to_string()]
        );
        
        let info = manager.get_info().await;
        assert_eq!(info.command, "test_command");
        assert_eq!(info.args, vec!["arg1", "arg2"]);
        assert!(matches!(info.status, ProcessStatus::Stopped));
    }

    #[tokio::test]
    async fn test_output_parsing() {
        let mut info = ProcessInfo {
            pid: None,
            command: "test".to_string(),
            args: vec![],
            status: ProcessStatus::Running,
            start_time: Utc::now(),
            last_output_time: None,
            stats: MinerStats {
                hashrate_hs: 0.0,
                shares_accepted: 0,
                shares_rejected: 0,
                uptime_seconds: 0,
                last_share_time: None,
                error_count: 0,
                temperature_c: None,
                power_usage_w: None,
            },
        };

        let mut patterns = HashMap::new();
        patterns.insert(
            "xmrig_hashrate".to_string(),
            Regex::new(r"speed\s+10s/60s/15m\s+([\d.]+)\s*([KMG]?)H/s").unwrap()
        );

        // Test hashrate parsing
        let test_line = "speed 10s/60s/15m 1234.5 H/s max 2000.0 H/s";
        MinerProcessManager::parse_output_line(&test_line, &mut info, &patterns).await;
        
        assert_eq!(info.stats.hashrate_hs, 1234.5);
    }
}