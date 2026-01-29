#!/bin/sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

subxt codegen --derive Clone --derive-for-type pallet_robonomics_cps::NodeId=Copy > ${SCRIPT_DIR}/../src/robonomics_runtime.rs
