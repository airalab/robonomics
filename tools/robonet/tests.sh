#!/usr/bin/env bash
#
# Robonomics Integration Test Runner
# 

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="${SCRIPT_DIR}/tests"

# Default values
VERBOSE=false
TEST_FILTER=""

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

log_debug() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${BLUE}[DEBUG]${NC} $1"
    fi
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Display usage
show_help() {
    cat << EOF
Robonomics Zombienet Test Runner

Usage: $0 [OPTIONS]

Options:
  --help              Show this help message
  --verbose, -v       Enable verbose output
  --filter, -f TEST   Run only tests matching TEST pattern
                      Examples: 'xcm', 'upward', 'relay'
  --list, -l          List available tests

Examples:
  $0                          # Run all tests
  $0 --verbose                # Run all tests with verbose output
  $0 --filter xcm             # Run only XCM tests
  $0 -f relay                 # Run only relay token tests

EOF
}

# List available tests
list_tests() {
    log_info "Available test categories:"
    echo "  - network       : Network initialization and block production"
    echo "  - extrinsic     : Basic extrinsic submission"
    echo "  - xcm           : All XCM tests"
    echo "  - upward        : XCM upward messages (parachain → relay)"
    echo "  - downward      : XCM downward messages (relay → parachain)"
    echo "  - relay         : Relay token transfer tests"
    echo "  - assethub      : AssetHub integration tests"
    echo ""
    log_info "Test files:"
    ls -1 "${TESTS_DIR}"/*.js 2>/dev/null | sed 's#.*/##'
}

# Install dependencies
install_dependencies() {
    log_info "Installing test dependencies..."
    cd "$TESTS_DIR"
    
    if [ "$VERBOSE" = true ]; then
        if command_exists yarn; then
            yarn install
        else
            npm install
        fi
    else
        if command_exists yarn; then
            yarn install --silent 2>&1 | grep -v "warning" || true
        else
            npm install --silent 2>&1 | grep -v "npm WARN" || true
        fi
    fi
    
    log_info "Dependencies installed"
}

# Run integration tests
run_tests() {
    # Ensure we're in the tests directory
    cd "$TESTS_DIR"
    
    # Detect node executable (could be 'node' or 'nodejs')
    local node_cmd="node"
    if ! command_exists node && command_exists nodejs; then
        node_cmd="nodejs"
    fi
    
    # Set environment variables based on options
    export TEST_FILTER="$TEST_FILTER"
    export VERBOSE="$VERBOSE"
    
    log_debug "Running tests with filter: ${TEST_FILTER:-none}"
    log_debug "Node command: $node_cmd"
    
    # Run the tests
    if [ "$VERBOSE" = true ]; then
        "$node_cmd" "${TESTS_DIR}/integration-tests.js"
    else
        "$node_cmd" "${TESTS_DIR}/integration-tests.js" 2>&1
    fi
}

# Main execution
main() {
    log_info "==================================="
    log_info "Robonomics Integration Tests"
    log_info "==================================="
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_help
                exit 0
                ;;
            --verbose|-v)
                VERBOSE=true
                log_info "Verbose mode enabled"
                shift
                ;;
            --filter|-f)
                TEST_FILTER="$2"
                log_info "Test filter: $TEST_FILTER"
                shift 2
                ;;
            --list|-l)
                list_tests
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Verify node is installed
    if ! command_exists node && ! command_exists nodejs; then
        log_error "Node.js is not installed. Please install Node.js v18 or higher."
        exit 1
    fi
    
    # Install test dependencies
    install_dependencies
    
    # Run tests
    log_info ""
    log_info "Starting test execution..."
    log_info ""
    
    run_tests
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        log_info ""
        log_info "==================================="
        log_info "Tests completed successfully!"
        log_info "==================================="
    else
        log_error ""
        log_error "==================================="
        log_error "Tests failed with exit code $exit_code"
        log_error "==================================="
    fi
    
    exit $exit_code
}

# Run main function
main "$@"
