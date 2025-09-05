#!/usr/bin/env python3
"""
Mock API Server for Phase 4.4 Integration Testing
Simulates market data API responses for profit switching tests
"""

from flask import Flask, jsonify, request
from datetime import datetime, timezone
import threading
import time

app = Flask(__name__)

# Global state for market data
current_prices = {
    "KASPA": {"price_eur": 0.15, "network_difficulty": 5.2e15, "block_reward": 440},
    "RVN": {"price_eur": 0.028, "network_difficulty": 67.5e12, "block_reward": 2500}
}

profit_multipliers = {
    "KASPA": 1.0,
    "RVN": 1.0
}

@app.route('/api/v1/coins/prices', methods=['GET'])
def get_coin_prices():
    """Return current coin prices with applied profit multipliers"""
    response = {}
    
    for symbol, data in current_prices.items():
        adjusted_price = data["price_eur"] * profit_multipliers.get(symbol, 1.0)
        response[symbol] = {
            "symbol": symbol,
            "price_eur": adjusted_price,
            "last_updated": int(datetime.now(timezone.utc).timestamp())
        }
    
    return jsonify(response)

@app.route('/api/v1/network/stats', methods=['GET'])
def get_network_stats():
    """Return network difficulty and block reward information"""
    response = {}
    
    for symbol, data in current_prices.items():
        algorithm = "kHeavyHash" if symbol == "KASPA" else "KawPow"
        response[algorithm] = {
            "algorithm": algorithm,
            "network_difficulty": data["network_difficulty"],
            "block_reward": data["block_reward"],
            "last_updated": int(datetime.now(timezone.utc).timestamp())
        }
    
    return jsonify(response)

@app.route('/api/v1/test/set_profitable', methods=['POST'])
def set_most_profitable():
    """Test endpoint to manipulate which coin is most profitable"""
    data = request.json
    coin = data.get('coin', '').upper()
    multiplier = data.get('multiplier', 1.0)
    
    if coin in profit_multipliers:
        # Reset all multipliers to 1.0 first
        for symbol in profit_multipliers:
            profit_multipliers[symbol] = 1.0
            
        # Set the specified coin as most profitable
        profit_multipliers[coin] = multiplier
        
        print(f"🎯 Test API: Set {coin} as most profitable (multiplier: {multiplier}x)")
        return jsonify({"status": "success", "coin": coin, "multiplier": multiplier})
    else:
        return jsonify({"status": "error", "message": f"Unknown coin: {coin}"}), 400

@app.route('/api/v1/bunker_pool/stats', methods=['GET'])
def get_bunker_pool_stats():
    """Return BUNKER POOL statistics for integration testing"""
    return jsonify({
        "kHeavyHash": {
            "algorithm": "kHeavyHash",
            "current_hashrate": 1.25e15,
            "pool_fee_percent": 1.0,
            "minimum_payout": 100.0,
            "effective_fee_percent": 0.5,  # Reduced fee for BUNKER MINER users
            "last_block_time": int(datetime.now(timezone.utc).timestamp()) - 120,
            "active_miners": 1247,
            "network_difficulty": current_prices["KASPA"]["network_difficulty"],
            "pool_luck_24h": 1.08
        },
        "KawPow": {
            "algorithm": "KawPow", 
            "current_hashrate": 8.7e12,
            "pool_fee_percent": 1.0,
            "minimum_payout": 50.0,
            "effective_fee_percent": 0.5,
            "last_block_time": int(datetime.now(timezone.utc).timestamp()) - 95,
            "active_miners": 892,
            "network_difficulty": current_prices["RVN"]["network_difficulty"],
            "pool_luck_24h": 0.94
        }
    })

@app.route('/api/v1/test/status', methods=['GET'])
def test_status():
    """Health check endpoint for integration tests"""
    return jsonify({
        "status": "running",
        "test_mode": True,
        "current_profitable": max(profit_multipliers.items(), key=lambda x: x[1])[0],
        "timestamp": datetime.now(timezone.utc).isoformat()
    })

def log_requests():
    """Background thread to log API requests for debugging"""
    while True:
        time.sleep(5)
        most_profitable = max(profit_multipliers.items(), key=lambda x: x[1])
        print(f"📊 Mock API Status - Most Profitable: {most_profitable[0]} ({most_profitable[1]:.2f}x)")

if __name__ == '__main__':
    print("🌐 Starting Mock API Server for Phase 4.4 Integration Testing")
    print("📡 Endpoints available:")
    print("   GET  /api/v1/coins/prices")
    print("   GET  /api/v1/network/stats") 
    print("   GET  /api/v1/bunker_pool/stats")
    print("   POST /api/v1/test/set_profitable")
    print("   GET  /api/v1/test/status")
    
    # Start background logging thread
    log_thread = threading.Thread(target=log_requests, daemon=True)
    log_thread.start()
    
    app.run(host='127.0.0.1', port=8081, debug=False)