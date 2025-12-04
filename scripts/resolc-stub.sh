#!/bin/bash
# Stub wrapper for resolc compiler
# 
# This script provides a minimal stub implementation of resolc (Solidity to PolkaVM compiler)
# to satisfy build requirements for pallet-revive-fixtures when building with runtime-benchmarks.
#
# Background:
# The pallet-revive (from pallet-xcm dependency) requires both solc and resolc compilers
# for its fixtures. Since resolc is not readily available in standard package repositories
# and the actual contracts are not used by robonomics runtime, this stub satisfies the
# build requirements by producing empty but valid JSON output.
#
# Installation:
#   sudo cp scripts/resolc-stub.sh /usr/local/bin/resolc
#   sudo chmod +x /usr/local/bin/resolc
#
# Note: This is only needed for builds with --features runtime-benchmarks

# Check if we're being called with --standard-json
if [ "$1" = "--standard-json" ]; then
    # Read the JSON input from stdin (not used, but must be consumed to prevent broken pipe errors)
    # or stdin buffer issues when the caller expects the script to read input)
    cat > /dev/null
    
    # Return a minimal valid JSON response with empty compilation results
    cat << 'EOFJSON'
{
  "contracts": {},
  "sources": {}
}
EOFJSON
    exit 0
fi

# For version checks or any other invocation
echo "resolc stub version 1.0.0 (minimal implementation for robonomics benchmarking)"
exit 0
