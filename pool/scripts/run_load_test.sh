#!/bin/bash
# BUNKER POOL - Load Testing Script
# Validates pool performance with 1,000 concurrent miners

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
POOL_HOST="${POOL_HOST:-localhost}"
POOL_PORT="${POOL_PORT:-3333}"
CONCURRENT_MINERS="${CONCURRENT_MINERS:-1000}"
TEST_DURATION="${TEST_DURATION:-60}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Usage function
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "OPTIONS:"
    echo "  --host HOST          Pool host [default: localhost]"
    echo "  --port PORT          Pool port [default: 3333]"
    echo "  --miners COUNT       Number of concurrent miners [default: 1000]"
    echo "  --duration SECONDS   Test duration in seconds [default: 60]"
    echo "  --help              Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  POOL_HOST           Same as --host"
    echo "  POOL_PORT           Same as --port"
    echo "  CONCURRENT_MINERS   Same as --miners"
    echo "  TEST_DURATION       Same as --duration"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --host)
            POOL_HOST="$2"
            shift 2
            ;;
        --port)
            POOL_PORT="$2"
            shift 2
            ;;
        --miners)
            CONCURRENT_MINERS="$2"
            shift 2
            ;;
        --duration)
            TEST_DURATION="$2"
            shift 2
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Main function
main() {
    log_info "BUNKER POOL Load Testing"
    log_info "Pool: ${POOL_HOST}:${POOL_PORT}"
    log_info "Concurrent Miners: ${CONCURRENT_MINERS}"
    log_info "Test Duration: ${TEST_DURATION}s"
    echo ""

    cd "$PROJECT_ROOT"

    # Check if pool is reachable
    log_info "Checking pool connectivity..."
    if ! nc -z -w5 "$POOL_HOST" "$POOL_PORT" 2>/dev/null; then
        log_warning "Pool is not reachable at ${POOL_HOST}:${POOL_PORT}"
        log_warning "Make sure the pool is running or adjust host/port settings"
        echo ""
    fi

    # Build the load test binary
    log_info "Building load test binary..."
    if ! cargo build --bin load-test --release; then
        log_error "Failed to build load test binary"
        exit 1
    fi

    log_success "Load test binary built successfully"

    # Run the load test
    log_info "Starting load test..."
    echo ""

    # Set environment variables for the test
    export POOL_HOST POOL_PORT CONCURRENT_MINERS TEST_DURATION

    # Run the test
    if ./target/release/load-test; then
        log_success "Load test completed successfully!"
        log_success "Pool validated for ${CONCURRENT_MINERS} concurrent miners"
    else
        log_error "Load test failed!"
        log_error "Pool performance below requirements"
        exit 1
    fi
}

# Execute main function
main "$@"