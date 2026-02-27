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

# Check if we're in a nix shell or need to use the built runtime
if [ -z "$RUNTIME_WASM" ]; then
    # Default runtime path for cargo build
    RUNTIME="./target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm"
    
    # Check if runtime exists
    if [ ! -f "$RUNTIME" ]; then
        echo -e "${YELLOW}Runtime WASM not found at $RUNTIME${NC}"
        echo -e "${YELLOW}Building runtime with try-runtime features...${NC}"
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
    ENDPOINT=$KUSAMA_PUBLIC_ENDPOINT
elif [ "$1" == "kusama" ]; then
    ENDPOINT=$KUSAMA_PUBLIC_ENDPOINT
elif [ "$1" == "polkadot" ]; then
    ENDPOINT=$POLKADOT_PUBLIC_ENDPOINT
else
    echo -e "${RED} Invaid argument: should be set to 'kusama' or 'polkadot'.${NC}"
    exit 1
fi

echo -e "${GREEN} Endpoint: ${ENDPOINT}${NC}"
echo ""

try-runtime --runtime $RUNTIME on-runtime-upgrade --checks all --blocktime 6000 live --uri $ENDPOINT 
