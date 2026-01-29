#!/usr/bin/env bash
#
# Robonomics Integration Test Runner
# 

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="${SCRIPT_DIR}/tests"

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Run integration tests
run_tests() {
    # Ensure we're in the zombienet directory
    cd "$SCRIPT_DIR"
    
    # Detect node executable (could be 'node' or 'nodejs')
    local node_cmd="node"
    if ! command_exists node && command_exists nodejs; then
        node_cmd="nodejs"
    fi
    
    "$node_cmd" "${TESTS_DIR}/integration-tests.js"
}

# Main execution
main() {
    log_info "==================================="
    log_info "Robonomics Integration Tests"
    log_info "==================================="
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --help          Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    # Install test dependencies
    log_info "Installing test dependencies..."
    cd "$TESTS_DIR"
    if command_exists yarn; then
        yarn install
    else
        npm install
    fi
    
    run_tests
    
    log_info "==================================="
    log_info "Tests completed successfully!"
    log_info "==================================="
}

# Run main function
main "$@"
