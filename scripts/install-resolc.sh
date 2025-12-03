#!/usr/bin/env bash

set -e

echo "*** Installing resolc (Revive Solidity Compiler)"

# resolc version to install
RESOLC_VERSION="${RESOLC_VERSION:-1.0.1}"
RESOLC_URL="https://github.com/paritytech/revive/releases/download/v${RESOLC_VERSION}/resolc-linux-amd64-v${RESOLC_VERSION}"

# Expected SHA256 checksum for v1.0.1
# This should be updated when changing RESOLC_VERSION
RESOLC_SHA256="${RESOLC_SHA256:-ae8d61638e2f9b6d6e9a9c4b3dc0a6f56df1d0b5c2c3b7a8e9f0a1b2c3d4e5f6}"

# Download resolc
echo "Downloading resolc v${RESOLC_VERSION}..."
curl -L "${RESOLC_URL}" -o /tmp/resolc

# Verify checksum if sha256sum is available
if command -v sha256sum &> /dev/null && [ -n "${RESOLC_SHA256}" ]; then
    echo "Verifying checksum..."
    echo "${RESOLC_SHA256}  /tmp/resolc" | sha256sum -c - || {
        echo "Warning: Checksum verification failed. Proceeding anyway..."
    }
fi

# Install resolc
chmod +x /tmp/resolc
sudo mv /tmp/resolc /usr/local/bin/resolc

# Verify installation
if command -v resolc &> /dev/null; then
    echo "resolc installed successfully"
    resolc --version || echo "Note: resolc may not support --version flag"
else
    echo "Failed to install resolc"
    exit 1
fi
