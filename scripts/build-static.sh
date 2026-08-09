#!/bin/sh

set -e

# Get the directory where this script is located
SCRIPT_DIR="$(dirname "$(realpath "$0")")"
# Get the project root
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

GREEN='\033[0;32m'
NC='\033[0m' # No Color

if [ -z "$1" ]; then
    PROFILE="release"
else
    PROFILE="$1"
fi
echo -e "${GREEN}Building profile: ${PROFILE}${NC}"
echo ""

docker run -v "${PROJECT_ROOT}":/build -it \
    -e SUBSTRATE_CLI_GIT_COMMIT_HASH="$(git rev-parse --short HEAD)" \
    -e RUSTFLAGS="-Clink-arg=-lzstd" \
    -e SKIP_STORAGE_ACCESS_TEST_RUNTIME_WASM_BUILD=1 \
    --rm $(docker build -q "${PROJECT_ROOT}/scripts/docker/builder") \
    cargo build --profile "${PROFILE}"
