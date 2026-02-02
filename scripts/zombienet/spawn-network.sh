#!/usr/bin/env bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default to xcm-tests configuration
CONFIG="${1:-${SCRIPT_DIR}/configs/xcm-tests.toml}"

zombienet spawn "${CONFIG}" -p native
