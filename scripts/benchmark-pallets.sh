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

# List of pallets to benchmark with their output paths
PALLETS=(
    # System pallets - saved to runtime/robonomics/src/weights/
    "frame_system,runtime/robonomics/src/weights/frame_system.rs"
    "frame_system_extensions,runtime/robonomics/src/weights/frame_system_extensions.rs"
    "pallet_balances,runtime/robonomics/src/weights/pallet_balances.rs"
    "pallet_timestamp,runtime/robonomics/src/weights/pallet_timestamp.rs"
    "pallet_utility,runtime/robonomics/src/weights/pallet_utility.rs"
    "pallet_multisig,runtime/robonomics/src/weights/pallet_multisig.rs"
    "pallet_vesting,runtime/robonomics/src/weights/pallet_vesting.rs"
    "pallet_collator_selection,runtime/robonomics/src/weights/pallet_collator_selection.rs"
    "pallet_session,runtime/robonomics/src/weights/pallet_session.rs"
    "pallet_transaction_payment,runtime/robonomics/src/weights/pallet_transaction_payment.rs"
    "cumulus_pallet_parachain_system,runtime/robonomics/src/weights/cumulus_pallet_parachain_system.rs"
    "cumulus_pallet_weight_reclaim,runtime/robonomics/src/weights/cumulus_pallet_weight_reclaim.rs"
    "cumulus_pallet_xcmp_queue,runtime/robonomics/src/weights/cumulus_pallet_xcmp_queue.rs"
    "pallet_message_queue,runtime/robonomics/src/weights/pallet_message_queue.rs"
    "pallet_xcm,runtime/robonomics/src/weights/pallet_xcm.rs"
    # Robonomics pallets - saved to runtime/robonomics/src/weights/
    "pallet_robonomics_datalog,runtime/robonomics/src/weights/pallet_robonomics_datalog.rs"
    "pallet_robonomics_digital_twin,runtime/robonomics/src/weights/pallet_robonomics_digital_twin.rs"
    "pallet_robonomics_launch,runtime/robonomics/src/weights/pallet_robonomics_launch.rs"
    "pallet_robonomics_liability,runtime/robonomics/src/weights/pallet_robonomics_liability.rs"
    "pallet_robonomics_rws,runtime/robonomics/src/weights/pallet_robonomics_rws.rs"
    "pallet_robonomics_cps,runtime/robonomics/src/weights/pallet_robonomics_cps.rs"
    "pallet_robonomics_claim,runtime/robonomics/src/weights/pallet_robonomics_claim.rs"
    "pallet_xcm_info,runtime/robonomics/src/weights/pallet_xcm_info.rs"
)

# List of XCM pallets to benchmark with their output paths
# XCM pallets requires custom template
XCM_PALLETS=(
    "pallet_xcm_benchmarks::fungible,runtime/robonomics/src/weights/xcm/pallet_xcm_benchmarks_fungible.rs"
    "pallet_xcm_benchmarks::generic,runtime/robonomics/src/weights/xcm/pallet_xcm_benchmarks_generic.rs"
)

# Benchmark configuration (can be overridden with environment variables)
STEPS="${BENCHMARK_STEPS:-50}"
REPEAT="${BENCHMARK_REPEAT:-20}"

# Function to benchmark a pallet
benchmark_pallet() {
    local pallet_info=$1
    local pallet_name=$(echo "$pallet_info" | cut -d, -f1)
    local output_path=$(echo "$pallet_info" | cut -d, -f2)
    
    echo -e "${GREEN}Benchmarking $pallet_name...${NC}"
    
    # Capture output for better error reporting
    local output
    if output=$(frame-omni-bencher v1 benchmark pallet \
        --runtime "$RUNTIME" \
        --pallet "$pallet_name" \
        --extrinsic "*" \
        --output "$output_path" \
        --header "${PROJECT_ROOT}/.github/license-check/HEADER-APACHE2" \
        --wasm-execution=compiled \
        --steps "$STEPS" \
        --repeat "$REPEAT" \
        --heap-pages=4096 \
        --no-storage-info \
        --no-min-squares \
        --no-median-slopes 2>&1); then
        echo -e "${GREEN}✓ Successfully generated weights for $pallet_name${NC}"
        echo ""
    else
        echo -e "${RED}✗ Failed to generate weights for $pallet_name${NC}"
        echo -e "${RED}Error output:${NC}"
        echo "$output" | head -20
        echo ""
        return 1
    fi
}

# Function to benchmark XCM pallet
benchmark_xcm_pallet() {
    local pallet_info=$1
    local pallet_name=$(echo "$pallet_info" | cut -d, -f1)
    local output_path=$(echo "$pallet_info" | cut -d, -f2)
    
    echo -e "${GREEN}Benchmarking $pallet_name...${NC}"
    
    # Capture output for better error reporting
    local output
    if output=$(frame-omni-bencher v1 benchmark pallet \
        --runtime "$RUNTIME" \
        --pallet "$pallet_name" \
        --extrinsic "*" \
        --output "$output_path" \
        --header "${PROJECT_ROOT}/.github/license-check/HEADER-APACHE2" \
        --template "${PROJECT_ROOT}/scripts/weights/xcm-template.hbs" \
        --wasm-execution=compiled \
        --steps "$STEPS" \
        --repeat "$REPEAT" \
        --heap-pages=4096 \
        --no-storage-info \
        --no-min-squares \
        --no-median-slopes 2>&1); then
        echo -e "${GREEN}✓ Successfully generated weights for $pallet_name${NC}"
        echo ""
    else
        echo -e "${RED}✗ Failed to generate weights for $pallet_name${NC}"
        echo -e "${RED}Error output:${NC}"
        echo "$output" | head -20
        echo ""
        return 1
    fi
}

# Main execution
echo -e "${GREEN}Starting runtime benchmarks for all pallets${NC}"
echo "=================================================="
echo -e "Benchmark settings: ${YELLOW}steps=$STEPS, repeat=$REPEAT${NC}"
echo ""

failed_pallets=()

for pallet_info in "${PALLETS[@]}"; do
    if ! benchmark_pallet "$pallet_info"; then
        failed_pallets+=("$(echo "$pallet_info" | cut -d, -f1)")
    fi
done

for pallet_info in "${XCM_PALLETS[@]}"; do
    if ! benchmark_xcm_pallet "$pallet_info"; then
        failed_pallets+=("$(echo "$pallet_info" | cut -d, -f1)")
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
