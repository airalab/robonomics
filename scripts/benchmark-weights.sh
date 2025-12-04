#!/bin/bash
# Generate weights for all Robonomics pallets
# This script should be run from the project root directory

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Get the project root (two levels up from scripts/weights/)
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Change to project root
cd "${PROJECT_ROOT}"

# Template path relative to project root
TEMPLATE="./scripts/weights/frame-weight-template.hbs"
RUNTIME="./runtime/robonomics/target/srtool/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm"

echo "Generating weights for datalog..."
frame-omni-bencher v1 benchmark pallet --runtime "${RUNTIME}" --pallet pallet_robonomics_datalog --extrinsic "" --template "${TEMPLATE}" --output frame/datalog/src/weights.rs

echo "Generating weights for digital-twin..."
frame-omni-bencher v1 benchmark pallet --runtime "${RUNTIME}" --pallet pallet_robonomics_digital_twin --extrinsic "" --template "${TEMPLATE}" --output frame/digital-twin/src/weights.rs

echo "Generating weights for launch..."
frame-omni-bencher v1 benchmark pallet --runtime "${RUNTIME}" --pallet pallet_robonomics_launch --extrinsic "" --template "${TEMPLATE}" --output frame/launch/src/weights.rs

echo "Generating weights for liability..."
frame-omni-bencher v1 benchmark pallet --runtime "${RUNTIME}" --pallet pallet_robonomics_liability --extrinsic "" --template "${TEMPLATE}" --output frame/liability/src/weights.rs

echo "Generating weights for rws..."
frame-omni-bencher v1 benchmark pallet --runtime "${RUNTIME}" --pallet pallet_robonomics_rws --extrinsic "" --template "${TEMPLATE}" --output frame/rws/src/weights.rs

echo "All weights generated!"
