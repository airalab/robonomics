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

# Exit with install suggestions if a required binary is missing
require_bin() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo -e "${RED}Error: '$1' not found in PATH${NC}" >&2
        echo -e "${YELLOW}Install it with: $2${NC}" >&2
        echo -e "${YELLOW}Or use Nix flakes: nix develop -c ./scripts/development-chain.sh${NC}" >&2
        exit 1
    fi
}
require_bin chain-spec-builder "cargo install staging-chain-spec-builder"
case "$1" in
    docker) require_bin docker "https://docs.docker.com/get-docker/" ;;
    run) require_bin polkadot-omni-node "cargo install polkadot-omni-node" ;;
esac

# Check if we're in a nix shell or need to use the built runtime
if [ -z "$RUNTIME_WASM" ]; then
    # Default runtime path for cargo build
    RUNTIME="./target/release/wbuild/robonomics-runtime/development_runtime.rs.compact.compressed.wasm"
    
    # Check if runtime exists
    if [ ! -f "$RUNTIME" ]; then
        echo -e "${YELLOW}Runtime WASM not found at $RUNTIME${NC}"
        echo -e "${YELLOW}Building runtime...${NC}"
        require_bin cargo "https://rustup.rs"
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

echo -e "${GREEN}Using runtime:${NC} $RUNTIME"

SPEC_TMP_DIR=$(mktemp -d)
trap 'rm -rf "$SPEC_TMP_DIR"' EXIT

chain-spec-builder -c "${SPEC_TMP_DIR}/dev.json" create \
    -n "Robonomics Dev" -i robonomics-dev \
    -t development -r "$RUNTIME" --para-id 2000 --relay-chain westend-local \
    --properties tokenSymbol=XRT,tokenDecimals=9 \
    named-preset development

echo -e "${GREEN}Dev Spec created: ${NC}${SPEC_TMP_DIR}/dev.json"

if [ "$1" == "docker" ]; then
    echo -e "${GREEN}Building docker...${NC}"
    cp "${PROJECT_ROOT}/docker/Dockerfile" "${SPEC_TMP_DIR}"
    cp "${PROJECT_ROOT}/chain-spec/polkadot-parachain.raw.json" "${SPEC_TMP_DIR}/parachain.json"
    docker build "${SPEC_TMP_DIR}"
elif [ "$1" == "run" ]; then
    echo -e "${GREEN}Running...${NC}"
    polkadot-omni-node --chain "${SPEC_TMP_DIR}/dev.json" --dev
fi
