#!/usr/bin/env bash
#
# Robonomics Zombienet Integration Test Runner
# 
# This script sets up the environment, spawns a test network using zombienet,
# and runs integration tests.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Configuration
ZOMBIENET_VERSION="v1.3.106"
POLKADOT_VERSION="v1.15.2"
ZOMBIENET_BIN="${SCRIPT_DIR}/bin/zombienet"
POLKADOT_BIN="${SCRIPT_DIR}/bin/polkadot"
ROBONOMICS_BIN="${PROJECT_ROOT}/target/release/robonomics"
CONFIG_FILE="${SCRIPT_DIR}/robonomics-local.toml"
TESTS_DIR="${SCRIPT_DIR}/tests"

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Download a file if it doesn't exist
download_if_missing() {
    local url="$1"
    local output="$2"
    
    if [ ! -f "$output" ]; then
        log_info "Downloading $(basename "$output")..."
        if curl -L -o "$output" "$url"; then
            chmod +x "$output"
            # Verify the file is executable
            if [ ! -x "$output" ]; then
                log_error "Downloaded file is not executable: $output"
                return 1
            fi
            log_info "Successfully downloaded $(basename "$output")"
        else
            log_error "Failed to download from $url"
            return 1
        fi
    else
        log_info "$(basename "$output") already exists, skipping download"
    fi
}

# Setup environment
setup_environment() {
    log_info "Setting up environment..."
    
    # Create bin directory
    mkdir -p "${SCRIPT_DIR}/bin"
    
    # Check for Node.js
    if ! command_exists node; then
        log_error "Node.js is not installed. Please install Node.js to run tests."
        exit 1
    fi
    
    # Check for npm/yarn
    if ! command_exists npm && ! command_exists yarn; then
        log_error "npm or yarn is not installed. Please install one of them."
        exit 1
    fi
    
    # Detect platform
    local platform="linux-x64"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        platform="macos"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        platform="linux-x64"
    else
        log_warn "Unsupported platform: $OSTYPE. Trying linux-x64..."
        platform="linux-x64"
    fi
    
    # Download zombienet if not present
    if [ ! -f "$ZOMBIENET_BIN" ]; then
        log_info "Downloading zombienet ${ZOMBIENET_VERSION} for ${platform}..."
        local zombienet_url="https://github.com/paritytech/zombienet/releases/download/${ZOMBIENET_VERSION}/zombienet-${platform}"
        download_if_missing "$zombienet_url" "$ZOMBIENET_BIN"
    fi
    
    # Download polkadot if not present
    # Note: The polkadot binary from releases is universal (works on Linux and macOS)
    if [ ! -f "$POLKADOT_BIN" ]; then
        log_info "Downloading polkadot ${POLKADOT_VERSION}..."
        local polkadot_url="https://github.com/paritytech/polkadot-sdk/releases/download/polkadot-${POLKADOT_VERSION}/polkadot"
        download_if_missing "$polkadot_url" "$POLKADOT_BIN"
    fi
    
    # Check if robonomics binary exists
    if [ ! -f "$ROBONOMICS_BIN" ]; then
        log_warn "Robonomics binary not found at ${ROBONOMICS_BIN}"
        log_info "Building robonomics..."
        cd "$PROJECT_ROOT"
        cargo build --release
        
        if [ ! -f "$ROBONOMICS_BIN" ]; then
            log_error "Failed to build robonomics binary"
            exit 1
        fi
    fi
    
    log_info "Robonomics binary: ${ROBONOMICS_BIN}"
    log_info "Polkadot binary: ${POLKADOT_BIN}"
    log_info "Zombienet binary: ${ZOMBIENET_BIN}"
    
    # Install test dependencies
    log_info "Installing test dependencies..."
    cd "$TESTS_DIR"
    if command_exists yarn; then
        yarn install
    else
        npm install
    fi
    
    log_info "Environment setup complete"
}

# Run zombienet tests
run_tests() {
    log_info "Starting zombienet network and tests..."
    
    # Ensure we're in the zombienet directory
    cd "$SCRIPT_DIR"
    
    # Update the config to use the correct binaries
    log_info "Using configuration: ${CONFIG_FILE}"
    
    # Detect node executable (could be 'node' or 'nodejs')
    local node_cmd="node"
    if ! command_exists node && command_exists nodejs; then
        node_cmd="nodejs"
    fi
    
    # Run zombienet with test command
    log_info "Spawning test network with zombienet..."
    
    # Run zombienet in test mode
    # The test script will be executed after the network is up
    "$ZOMBIENET_BIN" test "$CONFIG_FILE" \
        --provider native \
        -- "$node_cmd" "${TESTS_DIR}/integration-tests.js"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up..."
    # Zombienet handles cleanup automatically
}

# Trap cleanup on exit
trap cleanup EXIT INT TERM

# Main execution
main() {
    log_info "==================================="
    log_info "Robonomics Zombienet Integration Tests"
    log_info "==================================="
    
    # Parse arguments
    SKIP_SETUP=false
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-setup)
                SKIP_SETUP=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --skip-setup    Skip environment setup and dependency installation"
                echo "  --help          Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    if [ "$SKIP_SETUP" = false ]; then
        setup_environment
    else
        log_warn "Skipping environment setup"
    fi
    
    run_tests
    
    log_info "==================================="
    log_info "Tests completed successfully!"
    log_info "==================================="
}

# Run main function
main "$@"
