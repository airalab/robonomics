#!/usr/bin/env bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
nix run github:paritytech/zombienet/v1.3.138 -- spawn ${SCRIPT_DIR}/robonomics-local.toml -p native
