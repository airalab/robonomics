#!/usr/bin/env bash
# Building Robonomics runtime development chain spec 
# This script should be run from the project root directory

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$(dirname "$(realpath "$0")")"
# Get the project root
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Change to project root
cd "${PROJECT_ROOT}"

# Check if we're in a nix shell or need to use the built runtime
if [ -z "$RUNTIME_WASM" ]; then
    # Default runtime path for cargo build
    RUNTIME="./target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm"
    
    # Check if runtime exists
    if [ ! -f "$RUNTIME" ]; then
        echo -e "${YELLOW}Runtime WASM not found at $RUNTIME${NC}"
        echo -e "${YELLOW}Building runtime...${NC}"
        cargo build --release -p robonomics-runtime
        
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

chain-spec-builder create -t development -r "$RUNTIME" --relay-chain rococo-local --para-id 2048 named-preset development
