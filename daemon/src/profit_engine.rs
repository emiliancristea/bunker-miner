use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tokio::time;
use anyhow::{Result, Context};
use reqwest::Client;
use tracing::{debug, info, warn, error};
use crate::config::Config;
use crate::overclocking::OverclockingEngine;
use crate::power_tuning::PowerTuningEngine;
use crate::hardware::MiningDevice;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunkerPoolStats {
    pub algorithm: String,
    pub current_hashrate: f64,
    pub pool_fee_percent: f64,
    pub minimum_payout: f64,
    pub effective_fee_percent: f64, // Special reduced fee for BUNKER MINER users
    pub last_block_time: u64,
    pub active_miners: u32,
    pub network_difficulty: f64,
    pub pool_luck_24h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinPrice {
    pub symbol: String,
    pub price_eur: f64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub algorithm: String,
    pub network_difficulty: f64,
    pub block_reward: f64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmProfile {
    pub name: String,
    pub hashrate_hs: f64,
    pub power_watts: f64,
    pub coin_symbol: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ProfitabilityData {
    pub algorithm: String,
    pub coin_symbol: String,
    pub net_profit_eur_per_day: f64,
    pub revenue_eur_per_day: f64,
    pub cost_eur_per_day: f64,
    pub hashrate_hs: f64,
    pub last_updated: u64,
}

#[derive(Debug, Clone)]
pub struct SwitchingDecision {
    pub should_switch: bool,
    pub target_algorithm: Option<String>,
    pub current_profit: f64,
    pub target_profit: f64,
    pub profit_delta_percent: f64,
    pub reason: String,
}

pub struct ProfitEngine {
    http_client: Client,
    coin_prices: HashMap<String, CoinPrice>,
    network_stats: HashMap<String, NetworkStats>,
    bunker_pool_stats: HashMap<String, BunkerPoolStats>,
    algorithm_profiles: Vec<AlgorithmProfile>,
    current_algorithm: Option<String>,
    last_switch_time: SystemTime,
    profit_delta_threshold: f64,
    min_dwell_time: Duration,
    electricity_rate_eur_per_kwh: f64,
    pool_fee_percent: f64,
}

impl ProfitEngine {
    pub fn new(config: &Config) -> Self {
        let client_builder = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("BUNKER-MINER/0.1.0");

        #[cfg(feature = "proxy")]
        if let Some(proxy_url) = &config.profit_switching.proxy_url {
            if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                client_builder = client_builder.proxy(proxy);
            }
        }

        Self {
            http_client: client_builder.build().unwrap_or_else(|_| Client::new()),
            coin_prices: HashMap::new(),
            network_stats: HashMap::new(),
            bunker_pool_stats: HashMap::new(),
            algorithm_profiles: Vec::new(),
            current_algorithm: None,
            last_switch_time: UNIX_EPOCH,
            profit_delta_threshold: config.profit_switching.profit_delta_threshold,
            min_dwell_time: Duration::from_secs(
                config.profit_switching.min_dwell_time_minutes * 60
            ),
            electricity_rate_eur_per_kwh: config.profit_switching.electricity_eur_per_kwh,
            pool_fee_percent: config.profit_switching.pool_fee_percent.unwrap_or(1.0),
        }
    }

    pub async fn initialize(&mut self, profiles: Vec<AlgorithmProfile>) -> Result<()> {
        self.algorithm_profiles = profiles.into_iter()
            .filter(|p| p.enabled)
            .collect();
        
        tracing::info!(
            "Profit engine initialized with {} enabled algorithms",
            self.algorithm_profiles.len()
        );
        
        self.refresh_market_data().await?;
        Ok(())
    }

    pub async fn refresh_market_data(&mut self) -> Result<()> {
        tracing::debug("Refreshing market data from external APIs");
        
        let coin_symbols: Vec<String> = self.algorithm_profiles
            .iter()
            .map(|p| p.coin_symbol.clone())
            .collect();
        
        if coin_symbols.is_empty() {
            return Ok(());
        }

        let prices_result = self.fetch_coin_prices(&coin_symbols).await;
        let network_result = self.fetch_network_stats().await;
        let bunker_pool_result = self.fetch_bunker_pool_stats().await;

        match (prices_result, network_result, bunker_pool_result) {
            (Ok(prices), Ok(stats), Ok(bunker_stats)) => {
                self.coin_prices = prices;
                self.network_stats = stats;
                self.bunker_pool_stats = bunker_stats;
                tracing::info!("Market data refreshed successfully (including BUNKER POOL stats)");
            }
            (Ok(prices), Ok(stats), Err(bunker_err)) => {
                self.coin_prices = prices;
                self.network_stats = stats;
                tracing::warn!("BUNKER POOL stats unavailable: {}, using fallback", bunker_err);
                tracing::info!("Market data refreshed successfully (external sources only)");
            }
            (Err(price_err), Ok(_)) => {
                tracing::warn!("Failed to fetch coin prices: {}", price_err);
                return Err(price_err);
            }
            (Ok(_), Err(stats_err)) => {
                tracing::warn!("Failed to fetch network stats: {}", stats_err);
                return Err(stats_err);
            }
            (Err(price_err), Err(_)) => {
                tracing::error!("Failed to fetch both prices and network stats");
                return Err(price_err);
            }
        }

        Ok(())
    }

    async fn fetch_coin_prices(&self, symbols: &[String]) -> Result<HashMap<String, CoinPrice>> {
        let symbols_str = symbols.join(",");
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=eur&include_last_updated_at=true",
            symbols_str
        );

        tracing::debug!("Fetching coin prices from: {}", url);

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .context("Failed to send price request to CoinGecko")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "CoinGecko API returned error status: {}",
                response.status()
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse CoinGecko response")?;

        let mut prices = HashMap::new();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for symbol in symbols {
            if let Some(coin_data) = json.get(symbol) {
                if let (Some(price), Some(last_updated)) = (
                    coin_data.get("eur").and_then(|v| v.as_f64()),
                    coin_data.get("last_updated_at").and_then(|v| v.as_u64())
                ) {
                    prices.insert(symbol.clone(), CoinPrice {
                        symbol: symbol.clone(),
                        price_eur: price,
                        last_updated,
                    });
                }
            } else {
                tracing::warn!("No price data found for symbol: {}", symbol);
                prices.insert(symbol.clone(), CoinPrice {
                    symbol: symbol.clone(),
                    price_eur: 0.0,
                    last_updated: current_time,
                });
            }
        }

        tracing::debug!("Successfully fetched prices for {} coins", prices.len());
        Ok(prices)
    }

    async fn fetch_network_stats(&self) -> Result<HashMap<String, NetworkStats>> {
        let mut stats = HashMap::new();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for profile in &self.algorithm_profiles {
            match profile.name.as_str() {
                "RandomX" => {
                    if let Ok(xmr_stats) = self.fetch_xmr_network_stats().await {
                        stats.insert("RandomX".to_string(), xmr_stats);
                    } else {
                        tracing::warn!("Failed to fetch RandomX network stats, using defaults");
                        stats.insert("RandomX".to_string(), NetworkStats {
                            algorithm: "RandomX".to_string(),
                            network_difficulty: 100000000000.0,
                            block_reward: 0.6,
                            last_updated: current_time,
                        });
                    }
                }
                "Ethash" => {
                    if let Ok(eth_stats) = self.fetch_eth_network_stats().await {
                        stats.insert("Ethash".to_string(), eth_stats);
                    } else {
                        tracing::warn!("Failed to fetch Ethash network stats, using defaults");
                        stats.insert("Ethash".to_string(), NetworkStats {
                            algorithm: "Ethash".to_string(),
                            network_difficulty: 1000000000000000.0,
                            block_reward: 2.0,
                            last_updated: current_time,
                        });
                    }
                }
                _ => {
                    tracing::warn!("Unknown algorithm: {}, using default stats", profile.name);
                    stats.insert(profile.name.clone(), NetworkStats {
                        algorithm: profile.name.clone(),
                        network_difficulty: 1000000.0,
                        block_reward: 1.0,
                        last_updated: current_time,
                    });
                }
            }
        }

        Ok(stats)
    }

    async fn fetch_xmr_network_stats(&self) -> Result<NetworkStats> {
        let url = "https://api.xmrpool.net/stats";
        
        let response = self.http_client
            .get(url)
            .send()
            .await
            .context("Failed to fetch XMR network stats")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse XMR stats response")?;

        let difficulty = json
            .get("network")
            .and_then(|n| n.get("difficulty"))
            .and_then(|d| d.as_f64())
            .unwrap_or(100000000000.0);

        let block_reward = json
            .get("network")
            .and_then(|n| n.get("reward"))
            .and_then(|r| r.as_f64())
            .map(|r| r / 1e12)
            .unwrap_or(0.6);

        Ok(NetworkStats {
            algorithm: "RandomX".to_string(),
            network_difficulty: difficulty,
            block_reward,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    async fn fetch_eth_network_stats(&self) -> Result<NetworkStats> {
        let url = "https://api.ethermine.org/networkStats";
        
        let response = self.http_client
            .get(url)
            .send()
            .await
            .context("Failed to fetch ETH network stats")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse ETH stats response")?;

        let difficulty = json
            .get("data")
            .and_then(|d| d.get("difficulty"))
            .and_then(|d| d.as_f64())
            .unwrap_or(1000000000000000.0);

        Ok(NetworkStats {
            algorithm: "Ethash".to_string(),
            network_difficulty: difficulty,
            block_reward: 2.0,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    async fn fetch_bunker_pool_stats(&self) -> Result<HashMap<String, BunkerPoolStats>> {
        let mut stats = HashMap::new();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Fetch stats for each supported algorithm from BUNKER POOL API
        let algorithms = vec!["SHA256", "Ethash", "RandomX", "Scrypt"];
        
        for algorithm in algorithms {
            match self.fetch_single_bunker_pool_stats(algorithm).await {
                Ok(pool_stats) => {
                    stats.insert(algorithm.to_string(), pool_stats);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch BUNKER POOL stats for {}: {}", algorithm, e);
                    // Create fallback stats with preferential treatment for BUNKER POOL
                    stats.insert(algorithm.to_string(), BunkerPoolStats {
                        algorithm: algorithm.to_string(),
                        current_hashrate: 0.0,
                        pool_fee_percent: 1.0,
                        minimum_payout: 0.1,
                        effective_fee_percent: 0.5, // 50% lower effective fee
                        last_block_time: current_time,
                        active_miners: 0,
                        network_difficulty: 1000000.0,
                        pool_luck_24h: 100.0,
                    });
                }
            }
        }

        Ok(stats)
    }

    async fn fetch_single_bunker_pool_stats(&self, algorithm: &str) -> Result<BunkerPoolStats> {
        let url = format!("https://api.bunkerminer.com/pool/stats/{}", algorithm.to_lowercase());
        
        tracing::debug!("Fetching BUNKER POOL stats from: {}", url);

        let response = self.http_client
            .get(&url)
            .header("User-Agent", "BUNKER-MINER-DAEMON/0.1.0")
            .header("X-Client-Type", "bunker-miner")
            .send()
            .await
            .context("Failed to fetch BUNKER POOL stats")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "BUNKER POOL API returned error status: {}",
                response.status()
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse BUNKER POOL response")?;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(BunkerPoolStats {
            algorithm: algorithm.to_string(),
            current_hashrate: json
                .get("hashrate")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            pool_fee_percent: json
                .get("fee_percent")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0),
            minimum_payout: json
                .get("minimum_payout")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.1),
            effective_fee_percent: json
                .get("effective_fee_percent")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5), // Special reduced rate for BUNKER MINER
            last_block_time: json
                .get("last_block_time")
                .and_then(|v| v.as_u64())
                .unwrap_or(current_time),
            active_miners: json
                .get("active_miners")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(0),
            network_difficulty: json
                .get("network_difficulty")
                .and_then(|v| v.as_f64())
                .unwrap_or(1000000.0),
            pool_luck_24h: json
                .get("luck_24h")
                .and_then(|v| v.as_f64())
                .unwrap_or(100.0),
        })
    }

    pub fn calculate_profitability(&self) -> Vec<ProfitabilityData> {
        let mut results = Vec::new();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for profile in &self.algorithm_profiles {
            let profit_data = self.calculate_net_profit(profile, current_time);
            results.push(profit_data);
        }

        results.sort_by(|a, b| b.net_profit_eur_per_day.partial_cmp(&a.net_profit_eur_per_day).unwrap());
        results
    }

    fn calculate_net_profit(&self, profile: &AlgorithmProfile, timestamp: u64) -> ProfitabilityData {
        let coin_price = self.coin_prices.get(&profile.coin_symbol);
        let network_stats = self.network_stats.get(&profile.name);
        let bunker_pool_stats = self.bunker_pool_stats.get(&profile.name);

        let (revenue_per_day, cost_per_day, net_profit_per_day) = match (coin_price, network_stats) {
            (Some(price), Some(stats)) => {
                let revenue = (profile.hashrate_hs * stats.block_reward * price.price_eur) / stats.network_difficulty;
                let cost = (profile.power_watts / 1000.0) * 24.0 * self.electricity_rate_eur_per_kwh;
                
                // Use BUNKER POOL's effective fee if available (preferential treatment)
                let effective_fee_percent = if let Some(bunker_stats) = bunker_pool_stats {
                    tracing::debug!("Using BUNKER POOL effective fee of {}% for {} (vs standard {}%)", 
                                  bunker_stats.effective_fee_percent, profile.name, self.pool_fee_percent);
                    bunker_stats.effective_fee_percent
                } else {
                    self.pool_fee_percent
                };
                
                let net_profit = (revenue * (1.0 - effective_fee_percent / 100.0)) - cost;
                (revenue, cost, net_profit)
            }
            _ => {
                tracing::warn!("Missing market data for {}/{}", profile.name, profile.coin_symbol);
                (0.0, 0.0, 0.0)
            }
        };

        ProfitabilityData {
            algorithm: profile.name.clone(),
            coin_symbol: profile.coin_symbol.clone(),
            net_profit_eur_per_day: net_profit_per_day,
            revenue_eur_per_day: revenue_per_day,
            cost_eur_per_day: cost_per_day,
            hashrate_hs: profile.hashrate_hs,
            last_updated: timestamp,
        }
    }

    pub fn evaluate_switching_decision(&mut self) -> SwitchingDecision {
        let profitability_rankings = self.calculate_profitability();
        
        if profitability_rankings.is_empty() {
            return SwitchingDecision {
                should_switch: false,
                target_algorithm: None,
                current_profit: 0.0,
                target_profit: 0.0,
                profit_delta_percent: 0.0,
                reason: "No algorithms available for evaluation".to_string(),
            };
        }

        let best_algorithm = &profitability_rankings[0];
        let current_algorithm_name = match &self.current_algorithm {
            Some(alg) => alg,
            None => {
                return SwitchingDecision {
                    should_switch: true,
                    target_algorithm: Some(best_algorithm.algorithm.clone()),
                    current_profit: 0.0,
                    target_profit: best_algorithm.net_profit_eur_per_day,
                    profit_delta_percent: 100.0,
                    reason: "No current algorithm, starting with most profitable".to_string(),
                };
            }
        };

        let current_profit = profitability_rankings
            .iter()
            .find(|p| p.algorithm == *current_algorithm_name)
            .map(|p| p.net_profit_eur_per_day)
            .unwrap_or(0.0);

        if best_algorithm.algorithm == *current_algorithm_name {
            return SwitchingDecision {
                should_switch: false,
                target_algorithm: Some(current_algorithm_name.clone()),
                current_profit,
                target_profit: best_algorithm.net_profit_eur_per_day,
                profit_delta_percent: 0.0,
                reason: "Already mining the most profitable algorithm".to_string(),
            };
        }

        let profit_delta_percent = if current_profit > 0.0 {
            ((best_algorithm.net_profit_eur_per_day - current_profit) / current_profit) * 100.0
        } else {
            100.0
        };

        let time_since_last_switch = SystemTime::now()
            .duration_since(self.last_switch_time)
            .unwrap_or(Duration::from_secs(0));

        let meets_delta_threshold = profit_delta_percent >= self.profit_delta_threshold;
        let meets_dwell_time = time_since_last_switch >= self.min_dwell_time;

        let should_switch = meets_delta_threshold && meets_dwell_time;

        let reason = if !meets_delta_threshold {
            format!(
                "Profit delta {:.2}% below threshold {:.2}%",
                profit_delta_percent, self.profit_delta_threshold
            )
        } else if !meets_dwell_time {
            format!(
                "Dwell time requirement not met ({:.0}s < {:.0}s)",
                time_since_last_switch.as_secs(),
                self.min_dwell_time.as_secs()
            )
        } else {
            format!(
                "Switch triggered: {:.2}% profit increase from {} to {}",
                profit_delta_percent, current_algorithm_name, best_algorithm.algorithm
            )
        };

        SwitchingDecision {
            should_switch,
            target_algorithm: Some(best_algorithm.algorithm.clone()),
            current_profit,
            target_profit: best_algorithm.net_profit_eur_per_day,
            profit_delta_percent,
            reason,
        }
    }

    pub fn execute_switch(&mut self, target_algorithm: String) -> Result<()> {
        tracing::info!("Executing switch to algorithm: {}", target_algorithm);
        
        self.current_algorithm = Some(target_algorithm);
        self.last_switch_time = SystemTime::now();
        
        Ok(())
    }

    pub fn get_current_algorithm(&self) -> Option<&String> {
        self.current_algorithm.as_ref()
    }

    pub fn set_current_algorithm(&mut self, algorithm: Option<String>) {
        if algorithm.is_some() {
            self.last_switch_time = SystemTime::now();
        }
        self.current_algorithm = algorithm;
    }

    /// Get BUNKER POOL statistics for client display
    pub fn get_bunker_pool_stats(&self) -> &HashMap<String, BunkerPoolStats> {
        &self.bunker_pool_stats
    }

    /// Check if BUNKER POOL has preferential rates for a given algorithm
    pub fn has_bunker_pool_advantage(&self, algorithm: &str) -> bool {
        if let Some(bunker_stats) = self.bunker_pool_stats.get(algorithm) {
            bunker_stats.effective_fee_percent < self.pool_fee_percent
        } else {
            false
        }
    }

    /// Get the effective fee percentage for an algorithm (prioritizing BUNKER POOL)
    pub fn get_effective_fee_percent(&self, algorithm: &str) -> f64 {
        if let Some(bunker_stats) = self.bunker_pool_stats.get(algorithm) {
            bunker_stats.effective_fee_percent
        } else {
            self.pool_fee_percent
        }
    }
}

pub struct ProfitEngineService {
    profit_engine: tokio::sync::Mutex<ProfitEngine>,
    overclocking_engine: std::sync::Arc<tokio::sync::Mutex<OverclockingEngine>>,
    power_tuning_engine: std::sync::Arc<tokio::sync::Mutex<PowerTuningEngine>>,
    update_interval: Duration,
    is_running: std::sync::atomic::AtomicBool,
}

impl ProfitEngineService {
    pub fn new(config: &Config) -> Self {
        let update_interval = Duration::from_secs(
            config.profit_switching.update_interval_minutes.unwrap_or(5) * 60
        );

        Self {
            profit_engine: tokio::sync::Mutex::new(ProfitEngine::new(config)),
            update_interval,
            is_running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub async fn start(&self, profiles: Vec<AlgorithmProfile>) -> Result<()> {
        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        {
            let mut engine = self.profit_engine.lock().await;
            engine.initialize(profiles).await?;
        }

        tracing::info!("Profit engine service started with {}s update interval", 
                      self.update_interval.as_secs());
        Ok(())
    }

    pub async fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        tracing::info!("Profit engine service stopped");
    }

    pub async fn update_loop(&self) -> Result<()> {
        while self.is_running.load(std::sync::atomic::Ordering::SeqCst) {
            if let Err(e) = self.update_market_data().await {
                tracing::error!("Failed to update market data: {}", e);
            }

            time::sleep(self.update_interval).await;
        }
        Ok(())
    }

    async fn update_market_data(&self) -> Result<()> {
        let mut engine = self.profit_engine.lock().await;
        engine.refresh_market_data().await
    }

    pub async fn get_profitability_rankings(&self) -> Vec<ProfitabilityData> {
        let engine = self.profit_engine.lock().await;
        engine.calculate_profitability()
    }

    pub async fn evaluate_switch(&self) -> SwitchingDecision {
        let mut engine = self.profit_engine.lock().await;
        engine.evaluate_switching_decision()
    }

    pub async fn execute_algorithm_switch(&self, target_algorithm: String) -> Result<()> {
        let mut engine = self.profit_engine.lock().await;
        engine.execute_switch(target_algorithm)
    }

    pub async fn set_current_algorithm(&self, algorithm: Option<String>) {
        let mut engine = self.profit_engine.lock().await;
        engine.set_current_algorithm(algorithm);
    }

    pub async fn get_current_algorithm(&self) -> Option<String> {
        let engine = self.profit_engine.lock().await;
        engine.get_current_algorithm().cloned()
    }

    /// Get BUNKER POOL statistics for client display
    pub async fn get_bunker_pool_stats(&self) -> HashMap<String, BunkerPoolStats> {
        let engine = self.profit_engine.lock().await;
        engine.get_bunker_pool_stats().clone()
    }

    /// Check if BUNKER POOL offers better rates for an algorithm
    pub async fn has_bunker_pool_advantage(&self, algorithm: &str) -> bool {
        let engine = self.profit_engine.lock().await;
        engine.has_bunker_pool_advantage(algorithm)
    }
}