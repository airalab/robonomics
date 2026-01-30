#!/usr/bin/env bash
# Generate weights for all Robonomics pallets using runtime benchmarks
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

# Change to project root
cd "${PROJECT_ROOT}"

# Template path
TEMPLATE="./scripts/weights/frame-weight-template.hbs"

# Check if we're in a nix shell or need to use the built runtime
if [ -z "$RUNTIME_WASM" ]; then
    # Default runtime path for cargo build
    RUNTIME="./target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm"
    
    # Check if runtime exists
    if [ ! -f "$RUNTIME" ]; then
        echo -e "${YELLOW}Runtime WASM not found at $RUNTIME${NC}"
        echo -e "${YELLOW}Building runtime with benchmarking features...${NC}"
        cargo build --release --features runtime-benchmarks -p robonomics-runtime
    fi
else
    RUNTIME="$RUNTIME_WASM"
fi

echo -e "${GREEN}Using runtime: $RUNTIME${NC}"
echo ""

# List of Robonomics custom pallets to benchmark
PALLETS=(
    "pallet_robonomics_datalog:frame/datalog/src/weights.rs"
    "pallet_robonomics_digital_twin:frame/digital-twin/src/weights.rs"
    "pallet_robonomics_launch:frame/launch/src/weights.rs"
    "pallet_robonomics_liability:frame/liability/src/weights.rs"
    "pallet_robonomics_rws:frame/rws/src/weights.rs"
    "pallet_robonomics_cps:frame/cps/src/weights.rs"
    "pallet_wrapped_native:frame/wrapped-native/src/weights.rs"
    "pallet_xcm_info:frame/xcm-info/src/weights.rs"
)

# Function to benchmark a pallet
benchmark_pallet() {
    local pallet_info=$1
    local pallet_name=$(echo "$pallet_info" | cut -d: -f1)
    local output_path=$(echo "$pallet_info" | cut -d: -f2)
    
    echo -e "${GREEN}Benchmarking $pallet_name...${NC}"
    
    if frame-omni-bencher v1 benchmark pallet \
        --runtime "$RUNTIME" \
        --pallet "$pallet_name" \
        --extrinsic "*" \
        --template "$TEMPLATE" \
        --output "$output_path" \
        --header ./LICENSE \
        --steps 50 \
        --repeat 20; then
        echo -e "${GREEN}✓ Successfully generated weights for $pallet_name${NC}"
        echo ""
    else
        echo -e "${RED}✗ Failed to generate weights for $pallet_name${NC}"
        echo ""
        return 1
    fi
}

# Main execution
echo -e "${GREEN}Starting runtime benchmarks for Robonomics pallets${NC}"
echo "=================================================="
echo ""

failed_pallets=()

for pallet_info in "${PALLETS[@]}"; do
    if ! benchmark_pallet "$pallet_info"; then
        failed_pallets+=("$(echo "$pallet_info" | cut -d: -f1)")
    fi
done

echo "=================================================="
if [ ${#failed_pallets[@]} -eq 0 ]; then
    echo -e "${GREEN}All benchmarks completed successfully!${NC}"
    exit 0
else
    echo -e "${RED}Failed to benchmark the following pallets:${NC}"
    for pallet in "${failed_pallets[@]}"; do
        echo -e "${RED}  - $pallet${NC}"
    done
    exit 1
fi
