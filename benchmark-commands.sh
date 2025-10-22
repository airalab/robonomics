#!/bin/bash
# Commands to generate weights for all Robonomics pallets

echo "Generating weights for datalog..."
frame-omni-bencher v1 benchmark pallet --runtime target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm --pallet pallet_robonomics_datalog --extrinsic "" --template ./frame-weight-template.hbs --output frame/datalog/src/weights.rs

echo "Generating weights for digital-twin..."
frame-omni-bencher v1 benchmark pallet --runtime target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm --pallet pallet_robonomics_digital_twin --extrinsic "" --template ./frame-weight-template.hbs --output frame/digital-twin/src/weights.rs

echo "Generating weights for launch..."
frame-omni-bencher v1 benchmark pallet --runtime target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm --pallet pallet_robonomics_launch --extrinsic "" --template ./frame-weight-template.hbs --output frame/launch/src/weights.rs

echo "Generating weights for liability..."
frame-omni-bencher v1 benchmark pallet --runtime target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm --pallet pallet_robonomics_liability --extrinsic "" --template ./frame-weight-template.hbs --output frame/liability/src/weights.rs

echo "Generating weights for rws..."
frame-omni-bencher v1 benchmark pallet --runtime target/release/wbuild/robonomics-runtime/robonomics_runtime.compact.compressed.wasm --pallet pallet_robonomics_rws --extrinsic "" --template ./frame-weight-template.hbs --output frame/rws/src/weights.rs

echo "All weights generated!"
