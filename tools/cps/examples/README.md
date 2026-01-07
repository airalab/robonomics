# libcps CLI Examples

This directory contains example scripts demonstrating libcps CLI usage.

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

## Encryption Examples

### Basic Encryption
- `encrypt_xchacha20.sh` - XChaCha20-Poly1305 encryption (default, recommended)
- `encrypt_ed25519.sh` - ED25519 scheme for IoT compatibility

### MQTT Integration
- `mqtt_encrypted.sh` - Subscribe to MQTT with encryption

## Running Examples

```bash
# Make scripts executable
chmod +x tools/cps/examples/*.sh

# Run an example
./tools/cps/examples/encrypt_xchacha20.sh
```

## Important Notes

⚠️ **Encryption requires BOTH**:
1. **Sender's seed phrase** (`--suri` or `ROBONOMICS_SURI`)
2. **Receiver's public key** (`--receiver-public`)

Without both parameters, data will be stored as plaintext.
