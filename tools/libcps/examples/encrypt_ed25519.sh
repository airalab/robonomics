#!/bin/bash
# Example: Encrypt using ED25519 scheme (Home Assistant compatible)

export ROBONOMICS_WS_URL=ws://localhost:9944
export ROBONOMICS_SURI="//Alice"  # Sender's seed phrase (REQUIRED)

RECEIVER="5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"

# Create with ED25519 encryption
cps create \
  --payload 'home assistant encrypted data' \
  --receiver-public "$RECEIVER" \
  --cipher aesgcm256 \
  --scheme ed25519

echo "✅ Node created with ED25519 + AES-GCM-256"
