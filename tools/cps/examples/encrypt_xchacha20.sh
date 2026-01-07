#!/bin/bash
# Example: Encrypt node payload using XChaCha20-Poly1305

# Setup
export ROBONOMICS_WS_URL=ws://localhost:9944
export ROBONOMICS_SURI="//Alice"  # Sender's seed phrase (REQUIRED for encryption)

# Bob's address as receiver
RECEIVER="5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"

# Create encrypted node
echo "Creating node with encrypted payload..."
cps create \
  --payload 'secret sensor data: temp=22.5C' \
  --receiver-public "$RECEIVER" \
  --cipher xchacha20 \
  --scheme sr25519

echo "✅ Node created with XChaCha20-Poly1305 encryption"
