# libcps Examples

This directory contains example scripts and code demonstrating libcps usage.

## Prerequisites

1. Running Robonomics node:
   ```bash
   robonomics --dev --tmp
   ```

2. Set environment variables:
   ```bash
   export ROBONOMICS_WS_URL=ws://localhost:9944
   export ROBONOMICS_SURI="//Alice"  # Your seed phrase
   ```

## CLI Examples (Shell Scripts)

### Encryption Examples
- `encrypt_xchacha20.sh` - XChaCha20-Poly1305 encryption (default, recommended)
- `encrypt_ed25519.sh` - ED25519 scheme for IoT compatibility

### MQTT Integration
- `mqtt_encrypted.sh` - Subscribe to MQTT with encryption

## Library Examples (Rust)

### MQTT Bridge Example
- `mqtt_bridge.rs` - Demonstrates using MQTT bridge from library code

Run the example:
```bash
cargo run --example mqtt_bridge
```

This example shows:
- How to configure MQTT and blockchain connections
- Using `mqtt::parse_mqtt_url()` for URL parsing
- Setting up subscribe bridge with custom message handler
- Setting up publish bridge with custom publish handler

## Running Shell Examples

```bash
# Make scripts executable
chmod +x tools/libcps/examples/*.sh

# Run an example
./tools/libcps/examples/encrypt_xchacha20.sh
```

## Important Notes

⚠️ **Encryption requires BOTH**:
1. **Sender's seed phrase** (`--suri` or `ROBONOMICS_SURI`)
2. **Receiver's public key** (`--receiver-public`)

Without both parameters, data will be stored as plaintext.
