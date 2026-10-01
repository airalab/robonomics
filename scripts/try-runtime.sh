#!/usr/bin/env bash
# Dry-run runtime upgrade on live chain using public RPC
# This script should be run from the project root directory

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Get the project root
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# Public endpoints
KUSAMA_PUBLIC_ENDPOINT="wss://kusama.rpc.robonomics.network"
POLKADOT_PUBLIC_ENDPOINT="wss://polkadot.rpc.robonomics.network"

# Change to project root
cd "${PROJECT_ROOT}"

# Exit with install suggestions if a required binary is missing
require_bin() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo -e "${RED}Error: '$1' not found in PATH${NC}" >&2
        echo -e "${YELLOW}Install it with: $2${NC}" >&2
        echo -e "${YELLOW}Or use Nix flakes: nix develop -c ./scripts/try-runtime.sh${NC}" >&2
        exit 1
    fi
}
require_bin try-runtime "cargo install --git https://github.com/paritytech/try-runtime-cli --locked"

# Check if we're in a nix shell or need to use the built runtime
if [ -z "$RUNTIME_WASM" ]; then
    # Default runtime path for cargo build
    RUNTIME="./target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm"
    
    # Check if runtime exists
    if [ ! -f "$RUNTIME" ]; then
        echo -e "${YELLOW}Runtime WASM not found at $RUNTIME${NC}"
        echo -e "${YELLOW}Building runtime with try-runtime features...${NC}"
        require_bin cargo "https://rustup.rs"
        cargo build --release --features try-runtime -p robonomics-runtime
        
        # Verify the build succeeded
        if [ ! -f "$RUNTIME" ]; then
            echo -e "${RED}Error: Failed to build runtime WASM${NC}"
            echo -e "${RED}Expected file at: $RUNTIME${NC}"
            exit 1
        fi
    fi
else
    RUNTIME="$RUNTIME_WASM"
fi

echo -e "${GREEN}Using runtime: $RUNTIME${NC}"
echo ""

# Main execution
echo -e "${GREEN}Starting try-runtime on live chain${NC}"
echo "=================================================="

if [ -z "$1" ]; then
    ENDPOINT=$POLKADOT_PUBLIC_ENDPOINT
elif [ "$1" == "kusama" ]; then
    ENDPOINT=$KUSAMA_PUBLIC_ENDPOINT
elif [ "$1" == "polkadot" ]; then
    ENDPOINT=$POLKADOT_PUBLIC_ENDPOINT
else
    echo -e "${RED} Invalid argument: should be set to 'kusama' or 'polkadot'.${NC}"
    exit 1
fi

echo -e "${GREEN} Endpoint: ${ENDPOINT}${NC}"
echo ""

try-runtime --runtime $RUNTIME on-runtime-upgrade --checks all --blocktime 6000 live --uri $ENDPOINT 
