use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

use crate::config::{Config, CONFIG_DIR_ENV};
use crate::miner_manifest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Telemetry {
    pub timestamp: u64,
    pub hashrate_hs: f64,
    pub hashrate_unit: String,
    pub hashrate: f64,
    pub shares_accepted: u32,
    pub shares_rejected: u32,
    pub shares_stale: u32,
    pub temperature_c: Option<f64>,
    pub power_watts: Option<f64>,
    pub fan_speed_percent: Option<f64>,
    pub pool_ping_ms: Option<u32>,
    pub algorithm: String,
    pub pool_url: String,
    pub error_message: String,
    pub pool_status: i32,
}

impl Default for Telemetry {
    fn default() -> Self {
        Telemetry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            hashrate_hs: 0.0,
            hashrate_unit: "H/s".to_string(),
            hashrate: 0.0,
            shares_accepted: 0,
            shares_rejected: 0,
            shares_stale: 0,
            temperature_c: None,
            power_watts: None,
            fan_speed_percent: None,
            pool_ping_ms: None,
            algorithm: "unknown".to_string(),
            pool_url: "".to_string(),
            error_message: "".to_string(),
            pool_status: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MinerBinary {
    pub name: String,
    pub version: String,
    pub executable_path: PathBuf,
    pub checksum_sha256: String,
    pub download_url: String,
    pub supported_coins: Vec<String>,
    pub supported_algorithms: Vec<String>,
}

#[async_trait]
pub trait MinerAdapter: Send + Sync {
    fn get_name(&self) -> &str;
    fn get_supported_coins(&self) -> &[String];
    fn get_supported_algorithms(&self) -> &[String];

    fn build_args(&self, config: &Config, device_ids: &[String]) -> Result<Vec<String>>;
    fn get_telemetry_patterns(&self) -> Vec<Regex>;
    fn parse_telemetry_line(&self, line: &str) -> Option<Telemetry>;

    async fn verify_binary(&self, binary_path: &Path) -> Result<()>;
    async fn download_binary(&self, download_dir: &Path) -> Result<PathBuf>;

    fn get_binary_info(&self) -> MinerBinary;
}

pub struct LolMinerAdapter {
    binary_info: MinerBinary,
    hashrate_pattern: Regex,
    shares_pattern: Regex,
    temp_pattern: Regex,
}

impl LolMinerAdapter {
    pub fn new() -> Self {
        let hashrate_pattern = Regex::new(r"GPU\s+\d+:\s+(\d+\.?\d*)\s*(kH/s|MH/s|GH/s|H/s)")
            .expect("Invalid hashrate regex");
        let shares_pattern =
            Regex::new(r"Accepted:\s*(\d+),\s*Rejected:\s*(\d+)").expect("Invalid shares regex");
        let temp_pattern = Regex::new(r"GPU\s+\d+.*?(\d+)°C").expect("Invalid temperature regex");

        LolMinerAdapter {
            binary_info: MinerBinary {
                name: "lolMiner".to_string(),
                version: "1.82".to_string(),
                executable_path: PathBuf::new(),
                checksum_sha256: String::new(),
                download_url: "https://github.com/Lolliedieb/lolMiner-releases/releases/download/1.82/lolMiner_v1.82_Win64.zip".to_string(),
                supported_coins: vec!["ethereum".to_string(), "ethereum_classic".to_string(), "beam".to_string()],
                supported_algorithms: vec!["ethash".to_string(), "etchash".to_string(), "beamhash".to_string()],
            },
            hashrate_pattern,
            shares_pattern,
            temp_pattern,
        }
    }
}

impl Default for LolMinerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MinerAdapter for LolMinerAdapter {
    fn get_name(&self) -> &str {
        &self.binary_info.name
    }

    fn get_supported_coins(&self) -> &[String] {
        &self.binary_info.supported_coins
    }

    fn get_supported_algorithms(&self) -> &[String] {
        &self.binary_info.supported_algorithms
    }

    fn build_args(&self, config: &Config, device_ids: &[String]) -> Result<Vec<String>> {
        let wallet = config.get_active_wallet()?;
        let pool = config.get_active_pool()?;

        let mut args = vec![];

        // Algorithm selection
        match wallet.coin.as_str() {
            "ethereum" => args.push("--algo=ETHASH".to_string()),
            "ethereum_classic" => args.push("--algo=ETCHASH".to_string()),
            "beam" => args.push("--algo=BEAM-III".to_string()),
            _ => return Err(anyhow!("Unsupported coin for lolMiner: {}", wallet.coin)),
        }

        args.push(format!("--pool={}", build_pool_endpoint(pool, "ssl")));

        // Wallet address
        args.push(format!("--user={}", wallet.address));

        // Worker name
        if let Some(worker) = &pool.worker_name {
            args.push(format!("--worker={}", worker));
        }

        // Device selection
        if !device_ids.is_empty() {
            args.push(format!("--devices={}", device_ids.join(",")));
        }

        // Additional optimization flags
        args.push("--apiport=0".to_string()); // Disable API for security
        args.push("--nocolor".to_string()); // Disable colored output for parsing
        args.push("--shortstats=1".to_string()); // Concise statistics

        Ok(args)
    }

    fn get_telemetry_patterns(&self) -> Vec<Regex> {
        vec![
            self.hashrate_pattern.clone(),
            self.shares_pattern.clone(),
            self.temp_pattern.clone(),
        ]
    }

    fn parse_telemetry_line(&self, line: &str) -> Option<Telemetry> {
        let mut telemetry = Telemetry {
            algorithm: "ethash".to_string(),
            ..Telemetry::default()
        };
        let mut updated = false;

        // Parse hashrate
        if let Some(captures) = self.hashrate_pattern.captures(line) {
            if let (Some(hashrate_match), Some(unit_match)) = (captures.get(1), captures.get(2)) {
                if let Ok(hashrate) = hashrate_match.as_str().parse::<f64>() {
                    let unit = unit_match.as_str();
                    telemetry.hashrate = hashrate;
                    telemetry.hashrate_unit = unit.to_string();

                    // Convert to H/s for standardization
                    telemetry.hashrate_hs = match unit {
                        "H/s" => hashrate,
                        "kH/s" => hashrate * 1_000.0,
                        "MH/s" => hashrate * 1_000_000.0,
                        "GH/s" => hashrate * 1_000_000_000.0,
                        _ => hashrate,
                    };

                    updated = true;
                }
            }
        }

        // Parse shares
        if let Some(captures) = self.shares_pattern.captures(line) {
            if let (Some(accepted_match), Some(rejected_match)) = (captures.get(1), captures.get(2))
            {
                if let (Ok(accepted), Ok(rejected)) = (
                    accepted_match.as_str().parse::<u32>(),
                    rejected_match.as_str().parse::<u32>(),
                ) {
                    telemetry.shares_accepted = accepted;
                    telemetry.shares_rejected = rejected;
                    updated = true;
                }
            }
        }

        // Parse temperature
        if let Some(captures) = self.temp_pattern.captures(line) {
            if let Some(temp_match) = captures.get(1) {
                if let Ok(temp) = temp_match.as_str().parse::<f64>() {
                    telemetry.temperature_c = Some(temp);
                    updated = true;
                }
            }
        }

        if updated {
            Some(telemetry)
        } else {
            None
        }
    }

    async fn verify_binary(&self, binary_path: &Path) -> Result<()> {
        verify_miner_executable(binary_path, &self.binary_info).await
    }

    async fn download_binary(&self, download_dir: &Path) -> Result<PathBuf> {
        Err(anyhow!(
            "Automatic download for {} is disabled until verified archive acquisition is implemented; install it under {} manually and provide a trusted checksum",
            self.binary_info.name,
            download_dir.display()
        ))
    }

    fn get_binary_info(&self) -> MinerBinary {
        self.binary_info.clone()
    }
}

pub struct XMRigAdapter {
    binary_info: MinerBinary,
    hashrate_pattern: Regex,
    shares_pattern: Regex,
    temp_pattern: Regex,
}

impl XMRigAdapter {
    pub fn new() -> Self {
        let hashrate_pattern =
            Regex::new(r"(\d+\.?\d*)\s*(H/s|kH/s|MH/s)").expect("Invalid hashrate regex");
        let shares_pattern = Regex::new(r"accepted:\s*(\d+)/(\d+)").expect("Invalid shares regex");
        let temp_pattern = Regex::new(r"temp:\s*(\d+)C").expect("Invalid temperature regex");

        XMRigAdapter {
            binary_info: MinerBinary {
                name: "XMRig".to_string(),
                version: "6.20.0".to_string(),
                executable_path: PathBuf::new(),
                checksum_sha256: String::new(),
                download_url: "https://github.com/xmrig/xmrig/releases/download/v6.20.0/xmrig-6.20.0-msvc-win64.zip".to_string(),
                supported_coins: vec!["monero".to_string(), "wownero".to_string()],
                supported_algorithms: vec!["randomx".to_string(), "randomwow".to_string()],
            },
            hashrate_pattern,
            shares_pattern,
            temp_pattern,
        }
    }
}

impl Default for XMRigAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MinerAdapter for XMRigAdapter {
    fn get_name(&self) -> &str {
        &self.binary_info.name
    }

    fn get_supported_coins(&self) -> &[String] {
        &self.binary_info.supported_coins
    }

    fn get_supported_algorithms(&self) -> &[String] {
        &self.binary_info.supported_algorithms
    }

    fn build_args(&self, config: &Config, device_ids: &[String]) -> Result<Vec<String>> {
        let wallet = config.get_active_wallet()?;
        let pool = config.get_active_pool()?;

        let mut args = vec![
            "-o".to_string(),
            build_pool_endpoint(pool, "tls"),
            "-u".to_string(),
            wallet.address.clone(),
        ];

        // Worker name (password field in XMRig)
        if let Some(worker) = &pool.worker_name {
            args.push("-p".to_string());
            args.push(worker.clone());
        }

        // Algorithm selection
        match wallet.coin.as_str() {
            "monero" => {
                args.push("-a".to_string());
                args.push("rx/0".to_string());
            }
            "wownero" => {
                args.push("-a".to_string());
                args.push("rx/wow".to_string());
            }
            _ => return Err(anyhow!("Unsupported coin for XMRig: {}", wallet.coin)),
        }

        // Device selection (CPU threads)
        if !device_ids.is_empty() {
            if let Ok(threads) = device_ids[0].parse::<u32>() {
                args.push("-t".to_string());
                args.push(threads.to_string());
            }
        }

        // Additional flags
        args.push("--no-color".to_string()); // Disable colors for parsing
        args.push("--print-time=60".to_string()); // Status every minute
        args.push("--http-enabled".to_string());
        args.push("--http-port=0".to_string()); // Disable HTTP API

        Ok(args)
    }

    fn get_telemetry_patterns(&self) -> Vec<Regex> {
        vec![
            self.hashrate_pattern.clone(),
            self.shares_pattern.clone(),
            self.temp_pattern.clone(),
        ]
    }

    fn parse_telemetry_line(&self, line: &str) -> Option<Telemetry> {
        let mut telemetry = Telemetry {
            algorithm: "randomx".to_string(),
            ..Telemetry::default()
        };
        let mut updated = false;

        // Parse hashrate
        if let Some(captures) = self.hashrate_pattern.captures(line) {
            if let (Some(hashrate_match), Some(unit_match)) = (captures.get(1), captures.get(2)) {
                if let Ok(hashrate) = hashrate_match.as_str().parse::<f64>() {
                    let unit = unit_match.as_str();
                    telemetry.hashrate = hashrate;
                    telemetry.hashrate_unit = unit.to_string();

                    // Convert to H/s for standardization
                    telemetry.hashrate_hs = match unit {
                        "H/s" => hashrate,
                        "kH/s" => hashrate * 1_000.0,
                        "MH/s" => hashrate * 1_000_000.0,
                        _ => hashrate,
                    };

                    updated = true;
                }
            }
        }

        // Parse shares (accepted/total format)
        if let Some(captures) = self.shares_pattern.captures(line) {
            if let (Some(accepted_match), Some(total_match)) = (captures.get(1), captures.get(2)) {
                if let (Ok(accepted), Ok(total)) = (
                    accepted_match.as_str().parse::<u32>(),
                    total_match.as_str().parse::<u32>(),
                ) {
                    telemetry.shares_accepted = accepted;
                    telemetry.shares_rejected = total - accepted;
                    updated = true;
                }
            }
        }

        // Parse temperature
        if let Some(captures) = self.temp_pattern.captures(line) {
            if let Some(temp_match) = captures.get(1) {
                if let Ok(temp) = temp_match.as_str().parse::<f64>() {
                    telemetry.temperature_c = Some(temp);
                    updated = true;
                }
            }
        }

        if updated {
            Some(telemetry)
        } else {
            None
        }
    }

    async fn verify_binary(&self, binary_path: &Path) -> Result<()> {
        verify_miner_executable(binary_path, &self.binary_info).await
    }

    async fn download_binary(&self, download_dir: &Path) -> Result<PathBuf> {
        Err(anyhow!(
            "Automatic download for {} is disabled until verified archive acquisition is implemented; install it under {} manually and provide a trusted checksum",
            self.binary_info.name,
            download_dir.display()
        ))
    }

    fn get_binary_info(&self) -> MinerBinary {
        self.binary_info.clone()
    }
}

pub struct MinerManager {
    adapters: HashMap<String, Arc<dyn MinerAdapter>>,
    binaries_dir: PathBuf,
}

impl MinerManager {
    pub fn new() -> Result<Self> {
        let config_dir = match env::var_os(CONFIG_DIR_ENV) {
            Some(path) => PathBuf::from(path),
            None => dirs::config_dir()
                .ok_or_else(|| anyhow!("Could not determine config directory"))?
                .join("bunker-miner"),
        };

        let binaries_dir = config_dir.join("binaries");

        // Create binaries directory
        fs::create_dir_all(&binaries_dir).context("Failed to create binaries directory")?;

        let mut adapters: HashMap<String, Arc<dyn MinerAdapter>> = HashMap::new();
        adapters.insert("lolminer".to_string(), Arc::new(LolMinerAdapter::new()));
        adapters.insert("xmrig".to_string(), Arc::new(XMRigAdapter::new()));

        Ok(MinerManager {
            adapters,
            binaries_dir,
        })
    }

    pub fn get_adapter(&self, name: &str) -> Option<Arc<dyn MinerAdapter>> {
        self.adapters.get(name).cloned()
    }

    pub fn get_adapter_for_coin(&self, coin: &str) -> Option<Arc<dyn MinerAdapter>> {
        for adapter in self.adapters.values() {
            if adapter.get_supported_coins().contains(&coin.to_string()) {
                return Some(adapter.clone());
            }
        }
        None
    }

    pub fn list_adapters(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }

    pub async fn ensure_binary_available(
        &self,
        adapter: &Arc<dyn MinerAdapter>,
    ) -> Result<PathBuf> {
        let binary_info = adapter.get_binary_info();
        let candidate_paths = self.candidate_binary_paths(&binary_info);

        let mut verification_errors = Vec::new();
        for binary_path in &candidate_paths {
            if !binary_path.exists() {
                verification_errors.push(format!("{}: not found", binary_path.display()));
                continue;
            }

            match adapter.verify_binary(binary_path).await {
                Ok(()) => {
                    debug!(
                        "Binary {} verified at {}",
                        binary_info.name,
                        binary_path.display()
                    );
                    return Ok(binary_path.clone());
                }
                Err(error) => {
                    verification_errors.push(format!("{}: {error}", binary_path.display()));
                }
            }
        }

        let binary_dir = self.binaries_dir.join(binary_info.name.to_lowercase());
        fs::create_dir_all(&binary_dir).context("Failed to create binary directory")?;

        Err(anyhow!(
            "{} binary is not installed with a trusted checksum. Automatic miner downloads are disabled until verified archive acquisition is implemented. Install the miner manually under {}, set BUNKER_MINERS_PATH or BUNKER_MINER_{}_PATH, and provide a SHA-256 via sidecar .sha256, BUNKER_MINER_{}_SHA256, BUNKER_MINER_MANIFEST_PATH, or managed miner-manifest.toml. Verification attempts: {}",
            binary_info.name,
            binary_dir.display(),
            miner_env_key(&binary_info.name),
            miner_env_key(&binary_info.name),
            verification_errors.join("; ")
        ))
    }

    fn candidate_binary_paths(&self, binary_info: &MinerBinary) -> Vec<PathBuf> {
        let binary_name = binary_executable_name(binary_info);
        let normalized_name = binary_info.name.to_lowercase();
        let env_key = miner_env_key(&binary_info.name);
        let mut candidates = Vec::new();

        if !binary_info.executable_path.as_os_str().is_empty() {
            candidates.push(binary_info.executable_path.clone());
        }

        if let Ok(path) = std::env::var(format!("BUNKER_MINER_{env_key}_PATH")) {
            candidates.push(PathBuf::from(path));
        }

        if let Ok(miners_root) = std::env::var("BUNKER_MINERS_PATH") {
            let root = PathBuf::from(miners_root);
            candidates.push(root.join(&binary_name));
            candidates.push(root.join(&normalized_name).join(&binary_name));
        }

        candidates.push(self.binaries_dir.join(&normalized_name).join(&binary_name));

        if let Ok(path_path) = which::which(&binary_name) {
            candidates.push(path_path);
        }

        dedupe_paths(candidates)
    }
}

async fn verify_miner_executable(binary_path: &Path, binary_info: &MinerBinary) -> Result<()> {
    let metadata = fs::metadata(binary_path).with_context(|| {
        format!(
            "Failed to read binary metadata for {}",
            binary_path.display()
        )
    })?;

    if !metadata.is_file() {
        return Err(anyhow!(
            "Binary path is not a file: {}",
            binary_path.display()
        ));
    }

    let Some(expected_hash) = expected_sha256(binary_path, binary_info)? else {
        if allow_unverified_miners() {
            warn!(
                "Running unverified {} binary at {} because BUNKER_MINER_ALLOW_UNVERIFIED_MINERS=1",
                binary_info.name,
                binary_path.display()
            );
            return Ok(());
        }

        return Err(anyhow!(
            "No trusted SHA-256 checksum is configured for {}. Provide {}.sha256, BUNKER_MINER_{}_SHA256, BUNKER_MINER_MANIFEST_PATH, or managed miner-manifest.toml.",
            binary_path.display(),
            binary_path.display(),
            miner_env_key(&binary_info.name)
        ));
    };

    let actual_hash = sha256_file(binary_path).await?;
    if actual_hash != expected_hash {
        return Err(anyhow!(
            "SHA-256 mismatch for {}: expected {}, got {}",
            binary_path.display(),
            expected_hash,
            actual_hash
        ));
    }

    Ok(())
}

fn expected_sha256(binary_path: &Path, binary_info: &MinerBinary) -> Result<Option<String>> {
    let env_key = format!("BUNKER_MINER_{}_SHA256", miner_env_key(&binary_info.name));
    if let Ok(value) = std::env::var(env_key) {
        return Ok(Some(parse_sha256_value(&value)?));
    }

    let sidecar_path = binary_path.with_extension(format!(
        "{}sha256",
        binary_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|_| "")
            .unwrap_or("")
    ));
    if sidecar_path.exists() {
        let sidecar = fs::read_to_string(&sidecar_path).with_context(|| {
            format!("Failed to read checksum sidecar {}", sidecar_path.display())
        })?;
        return Ok(Some(parse_sha256_value(&sidecar)?));
    }

    let adjacent_sha256 = PathBuf::from(format!("{}.sha256", binary_path.display()));
    if adjacent_sha256.exists() {
        let sidecar = fs::read_to_string(&adjacent_sha256).with_context(|| {
            format!(
                "Failed to read checksum sidecar {}",
                adjacent_sha256.display()
            )
        })?;
        return Ok(Some(parse_sha256_value(&sidecar)?));
    }

    if let Some(manifest_sha256) = miner_manifest::trusted_sha256_for_binary(
        &binary_info.name,
        &binary_info.version,
        &binary_executable_name(binary_info),
    )? {
        return Ok(Some(manifest_sha256));
    }

    if is_valid_sha256(&binary_info.checksum_sha256) {
        return Ok(Some(binary_info.checksum_sha256.to_ascii_lowercase()));
    }

    Ok(None)
}

async fn sha256_file(path: &Path) -> Result<String> {
    let data = tokio::fs::read(path)
        .await
        .with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(sha256_bytes(&data))
}

fn sha256_bytes(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn parse_sha256_value(value: &str) -> Result<String> {
    let Some(candidate) = value.split_whitespace().find(|part| is_valid_sha256(part)) else {
        return Err(anyhow!(
            "Checksum value must contain a 64-character lowercase or uppercase hex SHA-256 digest"
        ));
    };

    Ok(candidate.to_ascii_lowercase())
}

fn is_valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn binary_executable_name(binary_info: &MinerBinary) -> String {
    match binary_info.name.as_str() {
        "lolMiner" if cfg!(windows) => "lolMiner.exe".to_string(),
        "lolMiner" => "lolMiner".to_string(),
        "XMRig" if cfg!(windows) => "xmrig.exe".to_string(),
        "XMRig" => "xmrig".to_string(),
        name if cfg!(windows) => format!("{}.exe", name.to_lowercase()),
        name => name.to_lowercase(),
    }
}

fn miner_env_key(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn allow_unverified_miners() -> bool {
    std::env::var("BUNKER_MINER_ALLOW_UNVERIFIED_MINERS")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        let normalized = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !deduped
            .iter()
            .any(|existing: &PathBuf| existing == &normalized)
        {
            deduped.push(normalized);
        }
    }

    deduped
}

#[derive(Debug)]
pub enum ProcessStatus {
    Starting,
    Running,
    Stopped,
    Crashed,
    Restarting,
}

pub struct ProcessSupervisor {
    config: Config,
    adapter: Arc<dyn MinerAdapter>,
    binary_path: PathBuf,
    device_ids: Vec<String>,

    // Process state
    child_process: Option<Child>,
    status: ProcessStatus,
    restart_count: u32,
    last_restart_time: Option<SystemTime>,

    // Telemetry
    telemetry_sender: Option<mpsc::UnboundedSender<Telemetry>>,
    latest_telemetry: Arc<RwLock<Option<Telemetry>>>,

    // Watchdog
    max_restart_attempts: u32,
    restart_delay_seconds: u64,
    max_restart_delay_seconds: u64,
}

impl ProcessSupervisor {
    pub fn new(
        config: Config,
        adapter: Arc<dyn MinerAdapter>,
        binary_path: PathBuf,
        device_ids: Vec<String>,
    ) -> Self {
        ProcessSupervisor {
            max_restart_attempts: config.security.max_restart_attempts,
            restart_delay_seconds: config.security.restart_delay_seconds,
            max_restart_delay_seconds: config.security.max_restart_delay_seconds,
            config,
            adapter,
            binary_path,
            device_ids,
            child_process: None,
            status: ProcessStatus::Stopped,
            restart_count: 0,
            last_restart_time: None,
            telemetry_sender: None,
            latest_telemetry: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start(
        &mut self,
        telemetry_sender: mpsc::UnboundedSender<Telemetry>,
    ) -> Result<()> {
        if matches!(self.status, ProcessStatus::Running) {
            return Err(anyhow!("Process is already running"));
        }

        self.telemetry_sender = Some(telemetry_sender);
        self.status = ProcessStatus::Starting;

        info!("Starting miner process: {}", self.adapter.get_name());

        self.spawn_process().await
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.status = ProcessStatus::Stopped;

        if let Some(mut child) = self.child_process.take() {
            info!("Stopping miner process...");

            // Try graceful shutdown first
            if let Err(e) = child.kill().await {
                warn!("Failed to kill child process: {}", e);
            }

            // Wait for process to exit
            match timeout(Duration::from_secs(10), child.wait()).await {
                Ok(Ok(exit_status)) => {
                    info!("Miner process stopped with exit status: {}", exit_status);
                }
                Ok(Err(e)) => {
                    error!("Error waiting for process to stop: {}", e);
                }
                Err(_) => {
                    warn!("Timeout waiting for process to stop - process may still be running");
                }
            }
        }

        Ok(())
    }

    pub async fn supervise(&mut self) -> Result<()> {
        while !matches!(self.status, ProcessStatus::Stopped) {
            if let Some(child) = &mut self.child_process {
                // Check if process is still alive
                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        // Process has exited
                        self.child_process = None;

                        if exit_status.success() {
                            info!("Miner process exited normally");
                            self.status = ProcessStatus::Stopped;
                        } else {
                            error!(
                                "Miner process crashed with exit code: {:?}",
                                exit_status.code()
                            );
                            self.status = ProcessStatus::Crashed;

                            // Attempt restart if within limits
                            if self.restart_count < self.max_restart_attempts {
                                self.status = ProcessStatus::Restarting;

                                let delay = self.calculate_restart_delay();
                                info!(
                                    "Restarting miner in {} seconds (attempt {}/{})",
                                    delay,
                                    self.restart_count + 1,
                                    self.max_restart_attempts
                                );

                                sleep(Duration::from_secs(delay)).await;

                                if let Err(e) = self.spawn_process().await {
                                    error!("Failed to restart miner process: {}", e);
                                    self.status = ProcessStatus::Crashed;
                                }
                            } else {
                                error!(
                                    "Maximum restart attempts reached ({}), giving up",
                                    self.max_restart_attempts
                                );
                                self.status = ProcessStatus::Crashed;
                            }
                        }
                    }
                    Ok(None) => {
                        // Process is still running
                        self.status = ProcessStatus::Running;
                    }
                    Err(e) => {
                        error!("Error checking process status: {}", e);
                        self.status = ProcessStatus::Crashed;
                    }
                }
            }

            // Check every second
            sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    async fn spawn_process(&mut self) -> Result<()> {
        let args = self
            .adapter
            .build_args(&self.config, &self.device_ids)
            .context("Failed to build miner arguments")?;

        info!(
            "Spawning miner: {} {}",
            self.binary_path.display(),
            args.join(" ")
        );

        let mut child = Command::new(&self.binary_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn miner process")?;

        // Set up telemetry parsing
        if let Some(stdout) = child.stdout.take() {
            let adapter = self.adapter.clone();
            let telemetry_sender = self.telemetry_sender.clone();
            let latest_telemetry = self.latest_telemetry.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    debug!("Miner output: {}", line);

                    if let Some(telemetry) = adapter.parse_telemetry_line(&line) {
                        // Update latest telemetry
                        {
                            let mut latest = latest_telemetry.write().await;
                            *latest = Some(telemetry.clone());
                        }

                        // Send to telemetry channel
                        if let Some(sender) = &telemetry_sender {
                            if let Err(e) = sender.send(telemetry) {
                                error!("Failed to send telemetry: {}", e);
                                break;
                            }
                        }
                    }
                }
            });
        }

        self.child_process = Some(child);
        self.status = ProcessStatus::Running;
        self.restart_count += 1;
        self.last_restart_time = Some(SystemTime::now());

        info!("✓ Miner process started successfully");

        Ok(())
    }

    fn calculate_restart_delay(&self) -> u64 {
        // Exponential backoff: delay * 2^(restart_count - 1)
        let delay = self.restart_delay_seconds * (2_u64.pow(self.restart_count.saturating_sub(1)));
        delay.min(self.max_restart_delay_seconds)
    }

    pub async fn get_latest_telemetry(&self) -> Option<Telemetry> {
        self.latest_telemetry.read().await.clone()
    }

    pub fn get_status(&self) -> &ProcessStatus {
        &self.status
    }

    pub fn device_ids(&self) -> &[String] {
        &self.device_ids
    }

    pub fn get_restart_count(&self) -> u32 {
        self.restart_count
    }
}

fn build_pool_endpoint(pool: &crate::config::PoolConfig, secure_scheme: &str) -> String {
    let raw_url = pool.url.trim().trim_end_matches('/');
    let (host, scheme_requests_tls) = match raw_url.split_once("://") {
        Some((scheme, host)) => {
            let scheme = scheme.to_ascii_lowercase();
            (host, scheme.contains("ssl") || scheme.contains("tls"))
        }
        None => (raw_url, false),
    };

    if pool.ssl || scheme_requests_tls {
        format!("{secure_scheme}://{}:{}", host, pool.port)
    } else {
        format!("{}:{}", host, pool.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ShellTelemetryAdapter {
        args: Vec<String>,
        parser: LolMinerAdapter,
        supported_coins: Vec<String>,
        supported_algorithms: Vec<String>,
    }

    impl ShellTelemetryAdapter {
        fn new() -> Self {
            let args = if cfg!(windows) {
                vec![
                    "/C".to_string(),
                    "echo GPU 0: 25.5 MH/s && echo Accepted: 15, Rejected: 2".to_string(),
                ]
            } else {
                vec![
                    "-c".to_string(),
                    "printf 'GPU 0: 25.5 MH/s\nAccepted: 15, Rejected: 2\n'".to_string(),
                ]
            };

            Self {
                args,
                parser: LolMinerAdapter::new(),
                supported_coins: vec![],
                supported_algorithms: vec![],
            }
        }
    }

    #[async_trait]
    impl MinerAdapter for ShellTelemetryAdapter {
        fn get_name(&self) -> &str {
            "shell-telemetry-test"
        }

        fn get_supported_coins(&self) -> &[String] {
            &self.supported_coins
        }

        fn get_supported_algorithms(&self) -> &[String] {
            &self.supported_algorithms
        }

        fn build_args(&self, _config: &Config, _device_ids: &[String]) -> Result<Vec<String>> {
            Ok(self.args.clone())
        }

        fn get_telemetry_patterns(&self) -> Vec<Regex> {
            self.parser.get_telemetry_patterns()
        }

        fn parse_telemetry_line(&self, line: &str) -> Option<Telemetry> {
            self.parser.parse_telemetry_line(line)
        }

        async fn verify_binary(&self, _binary_path: &Path) -> Result<()> {
            Ok(())
        }

        async fn download_binary(&self, _download_dir: &Path) -> Result<PathBuf> {
            Err(anyhow!("test adapter does not download binaries"))
        }

        fn get_binary_info(&self) -> MinerBinary {
            MinerBinary {
                name: self.get_name().to_string(),
                version: "test".to_string(),
                executable_path: PathBuf::new(),
                checksum_sha256: String::new(),
                download_url: String::new(),
                supported_coins: vec![],
                supported_algorithms: vec![],
            }
        }
    }

    #[test]
    fn test_lolminer_args_generation() {
        let mut config = Config::default();
        config.wallets.get_mut("ethereum_main").unwrap().address =
            "0x742d35Cc6635C0532925a3b8D400cdFb7021f39f".to_string();

        let adapter = LolMinerAdapter::new();
        let device_ids = vec!["0".to_string(), "1".to_string()];

        let args = adapter.build_args(&config, &device_ids).unwrap();

        assert!(args.contains(&"--algo=ETHASH".to_string()));
        assert!(args.contains(&"--pool=ssl://pool.bunkerminer.com:3334".to_string()));
        assert!(!args.iter().any(|arg| arg.contains("ssl://stratum")));
        assert!(args
            .iter()
            .any(|arg| arg.contains("742d35Cc6635C0532925a3b8D400cdFb7021f39f")));
        assert!(args.iter().any(|arg| arg.contains("--devices=0,1")));
    }

    #[test]
    fn test_xmrig_args_generation() {
        let mut config = Config::default();
        config.mining.active_coin = "monero".to_string();
        config.mining.active_wallet = "monero_main".to_string();
        config.mining.active_pool = "minexmr".to_string();

        config.wallets.get_mut("monero_main").unwrap().address = 
            "42ey1afDFnn4886T7196doS9GPMzexD9gXpsZJDwVjeRVdFCSoHnv7KPbBeGpzJBzHRCAs9UxqeoyFQMYbqSWYTfJJQAWDm".to_string();

        let adapter = XMRigAdapter::new();
        let device_ids = vec!["8".to_string()]; // 8 CPU threads

        let args = adapter.build_args(&config, &device_ids).unwrap();

        assert!(args.contains(&"-a".to_string()));
        assert!(args.contains(&"rx/0".to_string()));
        assert!(args.contains(&"-t".to_string()));
        assert!(args.contains(&"8".to_string()));
        assert!(args.contains(&"pool.minexmr.com:4444".to_string()));
    }

    #[test]
    fn test_lolminer_telemetry_parsing() {
        let adapter = LolMinerAdapter::new();

        // Test hashrate parsing
        let line = "GPU 0: 25.5 MH/s, GPU 1: 24.8 MH/s";
        let telemetry = adapter.parse_telemetry_line(line);
        assert!(telemetry.is_some());
        let tel = telemetry.unwrap();
        assert!(tel.hashrate > 0.0);
        assert_eq!(tel.hashrate_unit, "MH/s");

        // Test shares parsing
        let line = "Accepted: 15, Rejected: 2";
        let telemetry = adapter.parse_telemetry_line(line);
        assert!(telemetry.is_some());
        let tel = telemetry.unwrap();
        assert_eq!(tel.shares_accepted, 15);
        assert_eq!(tel.shares_rejected, 2);
    }

    #[tokio::test]
    async fn test_miner_manager_creation() {
        let manager = MinerManager::new().unwrap();

        assert!(manager.get_adapter("lolminer").is_some());
        assert!(manager.get_adapter("xmrig").is_some());
        assert!(manager.get_adapter("nonexistent").is_none());

        assert!(manager.get_adapter_for_coin("ethereum").is_some());
        assert!(manager.get_adapter_for_coin("monero").is_some());
        assert!(manager.get_adapter_for_coin("bitcoin").is_none());
    }

    #[tokio::test]
    async fn test_binary_verification_accepts_sidecar_sha256() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let adapter = LolMinerAdapter::new();
        let binary_info = adapter.get_binary_info();
        let binary_path = temp_dir.path().join(binary_executable_name(&binary_info));
        let binary_contents = b"fake miner binary for checksum verification";

        fs::write(&binary_path, binary_contents).unwrap();
        fs::write(
            binary_path.with_extension("sha256"),
            format!(
                "{}  {}\n",
                sha256_bytes(binary_contents),
                binary_path.display()
            ),
        )
        .unwrap();

        adapter.verify_binary(&binary_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_binary_verification_rejects_sidecar_mismatch() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let adapter = XMRigAdapter::new();
        let binary_info = adapter.get_binary_info();
        let binary_path = temp_dir.path().join(binary_executable_name(&binary_info));

        fs::write(&binary_path, b"fake miner binary").unwrap();
        fs::write(
            binary_path.with_extension("sha256"),
            format!("{}\n", "0".repeat(64)),
        )
        .unwrap();

        let error = adapter.verify_binary(&binary_path).await.unwrap_err();
        assert!(error.to_string().contains("SHA-256 mismatch"));
    }

    #[tokio::test]
    async fn test_process_supervisor_streams_miner_telemetry() {
        let binary_path = if cfg!(windows) {
            PathBuf::from("cmd")
        } else {
            PathBuf::from("/bin/sh")
        };
        let adapter: Arc<dyn MinerAdapter> = Arc::new(ShellTelemetryAdapter::new());
        let mut supervisor =
            ProcessSupervisor::new(Config::default(), adapter, binary_path, Vec::new());
        let (telemetry_tx, mut telemetry_rx) = mpsc::unbounded_channel();

        supervisor.start(telemetry_tx).await.unwrap();

        let telemetry = timeout(Duration::from_secs(5), telemetry_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(telemetry.algorithm, "ethash");
        assert!(telemetry.hashrate_hs > 0.0);

        supervisor.stop().await.unwrap();
    }
}
