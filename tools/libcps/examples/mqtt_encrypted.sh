#!/bin/bash
# Example: MQTT bridge with encryption

export ROBONOMICS_WS_URL=ws://localhost:9944
export ROBONOMICS_SURI="//Alice"
export ROBONOMICS_MQTT_BROKER=mqtt://localhost:1883

RECEIVER="5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"

# Subscribe to MQTT topic and encrypt messages before storing on-chain
cps mqtt subscribe \
  "sensors/temperature" \
  5 \
  --receiver-public "$RECEIVER" \
  --cipher xchacha20 \
  --scheme sr25519

echo "✅ MQTT bridge running with encryption"
