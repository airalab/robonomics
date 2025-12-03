#!/usr/bin/env bash

set -e

echo "*** Installing resolc (Revive Solidity Compiler)"

# resolc version to install
RESOLC_VERSION="${RESOLC_VERSION:-1.0.1}"
RESOLC_URL="https://github.com/paritytech/revive/releases/download/v${RESOLC_VERSION}/resolc-linux-amd64-v${RESOLC_VERSION}"

# Download resolc
echo "Downloading resolc v${RESOLC_VERSION}..."
curl -L "${RESOLC_URL}" -o /tmp/resolc

# Verify checksum if provided via environment variable
# Set RESOLC_SHA256 environment variable to enable checksum verification
if [ -n "${RESOLC_SHA256}" ] && command -v sha256sum &> /dev/null; then
    echo "Verifying checksum..."
    echo "${RESOLC_SHA256}  /tmp/resolc" | sha256sum -c - || {
        echo "Error: Checksum verification failed!"
        rm -f /tmp/resolc
        exit 1
    }
    echo "Checksum verification passed"
else
    echo "Note: Skipping checksum verification (set RESOLC_SHA256 environment variable to enable)"
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
