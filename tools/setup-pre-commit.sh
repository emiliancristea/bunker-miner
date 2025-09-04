#!/bin/bash
# Setup pre-commit hooks for BUNKER MINER development
# This script installs and configures pre-commit hooks

set -euo pipefail

echo "🚀 Setting up BUNKER MINER pre-commit hooks..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo "📦 Installing pre-commit..."
    
    # Try pip first
    if command -v pip &> /dev/null; then
        pip install pre-commit
    elif command -v pip3 &> /dev/null; then
        pip3 install pre-commit
    else
        echo "❌ Neither pip nor pip3 found. Please install Python and pip first."
        echo "   Visit: https://python.org/downloads/"
        exit 1
    fi
fi

# Install pre-commit hooks
echo "🔧 Installing pre-commit hooks..."
pre-commit install

# Install commit message hooks
echo "📝 Installing commit message hooks..."
pre-commit install --hook-type commit-msg

# Create secrets baseline (empty initially)
if [ ! -f .secrets.baseline ]; then
    echo "🔒 Creating secrets detection baseline..."
    echo '{}' > .secrets.baseline
fi

# Run pre-commit on all files to ensure everything works
echo "✅ Running pre-commit on all files to verify setup..."
pre-commit run --all-files || {
    echo "⚠️  Some pre-commit checks failed. This is expected for initial setup."
    echo "   The hooks are now installed and will run on future commits."
}

echo ""
echo "🎉 Pre-commit hooks setup complete!"
echo ""
echo "📋 What happens now:"
echo "   • Hooks will run automatically on every commit"
echo "   • Code will be formatted and linted before commits"
echo "   • Security checks will prevent accidental secret commits"
echo "   • You can run 'pre-commit run --all-files' to check all files"
echo ""
echo "🛠️  Development workflow:"
echo "   1. Make your changes"
echo "   2. Run 'git add' to stage changes"  
echo "   3. Run 'git commit' - hooks will run automatically"
echo "   4. If hooks fail, fix issues and try again"
echo ""
echo "⚡ Pro tip: Run 'pre-commit run' to check staged files manually"