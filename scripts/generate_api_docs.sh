#!/bin/bash

# BUNKER MINER - API Documentation Generation Script
# Generates comprehensive documentation from Protocol Buffer definitions

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🔧 BUNKER MINER API Documentation Generation"
echo "Project root: $PROJECT_ROOT"

# Change to project root
cd "$PROJECT_ROOT"

# Create output directories
mkdir -p docs/api
mkdir -p docs/api/schemas
mkdir -p daemon/src/generated
mkdir -p client/src/generated

# Check if buf is installed
if ! command -v buf &> /dev/null; then
    echo "❌ Error: buf is not installed"
    echo "Please install buf from https://docs.buf.build/installation"
    exit 1
fi

echo "📋 Linting Protocol Buffer definitions..."
buf lint protos/

echo "🔍 Checking for breaking changes..."
# Skip breaking change check on first run or if no previous version exists
if git rev-parse --verify HEAD~1 >/dev/null 2>&1; then
    buf breaking protos/ --against ".git#branch=HEAD~1,subdir=protos" || {
        echo "⚠️  Breaking changes detected in API"
        echo "If this is intentional, consider bumping the API version"
    }
else
    echo "ℹ️  Skipping breaking change check (first commit or no previous version)"
fi

echo "🏗️  Generating code for all languages..."
buf generate protos/

echo "📚 Generating additional documentation..."

# Generate a comprehensive API overview
cat > docs/api/README.md << 'EOF'
# BUNKER MINER Daemon API Documentation

This directory contains comprehensive documentation for the BUNKER MINER daemon API.

## Files

- `index.html` - Interactive HTML documentation (open in browser)
- `schemas/` - JSON Schema definitions for API validation
- `README.md` - This overview file

## API Version

- **Version**: v1 (0.1.0)
- **Protocol**: gRPC with Protocol Buffers
- **Status**: STABLE

## Quick Start

1. **View Documentation**: Open `index.html` in your web browser
2. **Validate Requests**: Use JSON schemas in `schemas/` directory
3. **Generate Client Code**: Use `buf generate` with your target language

## Security

This API requires:
- localhost binding by default
- TLS encryption for remote access
- Authentication for administrative operations
- Rate limiting on all endpoints

For detailed security information, see [ADR-004-Daemon-API-Security-Design.md](../ADRs/ADR-004-Daemon-API-Security-Design.md).

## Support

For API questions and issues, please refer to the project documentation or create an issue in the repository.
EOF

echo "🎯 Validation: Checking generated files..."

# Verify key generated files exist
REQUIRED_FILES=(
    "docs/api/index.html"
    "daemon/src/generated/bunker.daemon.v1.rs"
    "client/src/generated/daemon_api.v1.pb.h"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        echo "✅ Generated: $file"
    else
        echo "❌ Missing: $file"
        exit 1
    fi
done

echo "📊 Statistics:"
echo "  Protocol Buffer files processed: $(find protos -name "*.proto" | wc -l)"
echo "  Generated Rust files: $(find daemon/src/generated -name "*.rs" | wc -l)"
echo "  Generated C++ files: $(find client/src/generated -name "*.h" -o -name "*.cc" | wc -l)"
echo "  JSON Schema files: $(find docs/api/schemas -name "*.json" | wc -l)"

echo "✅ API documentation generation completed successfully!"
echo "📖 Open docs/api/index.html in your browser to view the documentation"