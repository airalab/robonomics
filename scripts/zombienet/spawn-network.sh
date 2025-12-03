#!/usr/bin/env bash
#
# Helper script for manual network spawning and inspection
# Use this to spawn a zombienet network without running automated tests
#

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZOMBIENET_BIN="${SCRIPT_DIR}/bin/zombienet"
CONFIG_FILE="${SCRIPT_DIR}/robonomics-local.toml"

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check if zombienet exists
if [ ! -f "$ZOMBIENET_BIN" ]; then
    log_warn "Zombienet binary not found. Run './run-tests.sh' first to download it."
    exit 1
fi

log_info "Starting Robonomics test network..."
log_info "Network will run until you press Ctrl+C"
log_info ""
log_info "Connection endpoints:"
log_info "  Relay Chain (Alice): ws://127.0.0.1:9944"
log_info "  Relay Chain (Bob):   ws://127.0.0.1:9945"
log_info "  Parachain (Coll-01): ws://127.0.0.1:9988"
log_info "  Parachain (Coll-02): ws://127.0.0.1:9989"
log_info ""
log_info "You can connect to these endpoints using:"
log_info "  - Polkadot.js Apps: https://polkadot.js.org/apps/"
log_info "  - @polkadot/api in your scripts"
log_info ""

# Spawn network
"$ZOMBIENET_BIN" spawn "$CONFIG_FILE" --provider native
