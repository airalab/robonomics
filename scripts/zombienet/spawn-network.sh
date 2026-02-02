#!/usr/bin/env bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default to robonomics-local configuration
CONFIG="${1:-${SCRIPT_DIR}/configs/robonomics-local.toml}"

zombienet spawn "${CONFIG}" -p native
