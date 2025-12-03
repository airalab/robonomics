#!/usr/bin/env bash

set -e

echo "*** Installing resolc (Revive Solidity Compiler)"

# resolc version to install
RESOLC_VERSION="${RESOLC_VERSION:-1.0.1}"
RESOLC_URL="https://github.com/paritytech/revive/releases/download/v${RESOLC_VERSION}/resolc-linux-amd64-v${RESOLC_VERSION}"

# Download and install resolc
echo "Downloading resolc v${RESOLC_VERSION}..."
curl -L "${RESOLC_URL}" -o /tmp/resolc
chmod +x /tmp/resolc
sudo mv /tmp/resolc /usr/local/bin/resolc

# Verify installation
if command -v resolc &> /dev/null; then
    echo "resolc installed successfully"
    resolc --version
else
    echo "Failed to install resolc"
    exit 1
fi
