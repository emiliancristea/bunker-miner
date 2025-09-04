#!/bin/bash
# Check for hardcoded secrets, wallet addresses, and sensitive information
# Part of BUNKER MINER pre-commit security checks

set -euo pipefail

EXIT_CODE=0

echo "🔍 Checking for hardcoded secrets and sensitive information..."

# Patterns to detect potential secrets
WALLET_PATTERN="(bc1|[13])[a-zA-HJ-NP-Z0-9]{25,62}"  # Bitcoin addresses
ETH_PATTERN="0x[a-fA-F0-9]{40}"                        # Ethereum addresses
KASPA_PATTERN="kaspa:[a-z0-9]{61,63}"                  # Kaspa addresses
PRIVATE_KEY_PATTERN="[0-9a-fA-F]{64}"                  # 32-byte hex keys
API_KEY_PATTERN="sk_[a-zA-Z0-9]{32,}"                  # API keys

# Check staged files for sensitive patterns
FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(rs|cpp|hpp|c|h|toml|json|yml|yaml)$' || true)

if [ -n "$FILES" ]; then
    for file in $FILES; do
        echo "Checking: $file"
        
        # Skip test files and example configurations
        if [[ "$file" =~ (test|spec|example|template) ]]; then
            continue
        fi
        
        # Check for wallet addresses
        if git show ":$file" | grep -qE "$WALLET_PATTERN"; then
            echo "❌ Potential wallet address found in: $file"
            EXIT_CODE=1
        fi
        
        if git show ":$file" | grep -qE "$ETH_PATTERN"; then
            echo "❌ Potential Ethereum address found in: $file"
            EXIT_CODE=1
        fi
        
        if git show ":$file" | grep -qE "$KASPA_PATTERN"; then
            echo "❌ Potential Kaspa address found in: $file"
            EXIT_CODE=1
        fi
        
        # Check for potential private keys (exclude test vectors)
        if git show ":$file" | grep -qE "$PRIVATE_KEY_PATTERN" && ! [[ "$file" =~ test ]]; then
            echo "❌ Potential private key found in: $file"
            EXIT_CODE=1
        fi
        
        # Check for API keys
        if git show ":$file" | grep -qE "$API_KEY_PATTERN"; then
            echo "❌ Potential API key found in: $file"
            EXIT_CODE=1
        fi
        
        # Check for common secret keywords
        SECRET_KEYWORDS=("password\s*=" "secret\s*=" "token\s*=" "key\s*=" "private_key")
        for keyword in "${SECRET_KEYWORDS[@]}"; do
            if git show ":$file" | grep -qiE "$keyword.*['\"][^'\"]{8,}['\"]"; then
                echo "⚠️  Potential hardcoded secret keyword in: $file (keyword: $keyword)"
                echo "   Please ensure this is not a real secret"
            fi
        done
    done
fi

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ No hardcoded secrets detected"
else
    echo ""
    echo "🛡️  Security Check Failed!"
    echo "   Hardcoded secrets pose a serious security risk."
    echo "   Please use environment variables or secure configuration files."
    echo "   For development, use placeholder values like 'YOUR_WALLET_ADDRESS_HERE'"
fi

exit $EXIT_CODE