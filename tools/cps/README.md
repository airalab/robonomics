# 🌳 libcps - Robonomics CPS Library & CLI

A comprehensive Rust library and command-line interface for managing hierarchical Cyber-Physical Systems on the Robonomics blockchain.

## 📦 Packages

This crate provides two components:

### 1. **libcps** (Library)
A reusable library for building applications that interact with the Robonomics CPS pallet.

### 2. **cps** (CLI Binary)
A beautiful command-line interface for quick access to CPS pallet functionality.

## ✨ Features

- 🎨 **Beautiful colored output** with emojis and ASCII art (CLI)
- 🔐 **Multi-algorithm AEAD encryption** (XChaCha20-Poly1305, AES-256-GCM, ChaCha20-Poly1305)
- 🔑 **Dual keypair support** (SR25519 for Substrate, ED25519 for IoT/Home Assistant)
- 📡 **MQTT bridge** for IoT device integration
- 🌲 **Hierarchical tree visualization** of CPS nodes (CLI)
- ⚙️ **Flexible configuration** via environment variables or CLI args
- 🔒 **Secure by design** with proper key management and ECDH key agreement
- 📚 **Comprehensive documentation** for library API
- 🔧 **Type-safe blockchain integration** via subxt

## 📦 Installation

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
libcps = "0.1.0"
```

### CLI Tool from Crates.io

```bash
cargo install libcps
```

### From Source

```bash
# Clone the repository
git clone https://github.com/airalab/robonomics
cd robonomics

# Build the library
cargo build --release --package libcps --lib

# Build the CLI tool
cargo build --release --package libcps --bin cps

# The binary will be at: target/release/cps
```

### Add CLI to PATH (optional)

```bash
sudo cp target/release/cps /usr/local/bin/
```

## 📚 Library Usage

### Quick Start

```rust
use libcps::{Client, Config, types::NodeData};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to blockchain
    let config = Config {
        ws_url: "ws://localhost:9944".to_string(),
        suri: Some("//Alice".to_string()),
    };
    
    let client = Client::new(&config).await?;
    
    // Create node data
    let plain_data = NodeData::plain("sensor reading: 22.5C");
    let encrypted_data = NodeData::encrypted_xchacha(vec![1, 2, 3]);
    
    // Use client.api to interact with blockchain
    // (requires generated metadata from running node)
    
    Ok(())
}
```

### Encryption Example

#### SR25519 Encryption (Substrate Native)

```rust
use libcps::crypto::{Cipher, EncryptionAlgorithm, CryptoScheme};

fn encrypt_sr25519_example() -> anyhow::Result<()> {
    // Create cipher with SR25519 scheme
    let sender_cipher = Cipher::new(
        "//Alice".to_string(),
        EncryptionAlgorithm::XChaCha20Poly1305,
        CryptoScheme::Sr25519
    )?;

    let receiver_cipher = Cipher::new(
        "//Bob".to_string(),
        EncryptionAlgorithm::XChaCha20Poly1305,
        CryptoScheme::Sr25519
    )?;

    let plaintext = b"secret message";
    let receiver_public = receiver_cipher.public_key();

    // Encrypt with specific algorithm
    let encrypted = sender_cipher.encrypt(plaintext, &receiver_public)?;

    // Decrypt with sender verification (recommended for security)
    let sender_public = sender_cipher.public_key();
    let decrypted = receiver_cipher.decrypt(&encrypted, Some(&sender_public))?;
    assert_eq!(plaintext, &decrypted[..]);

    // Decrypt without sender verification (accepts from any sender)
    let decrypted_any = receiver_cipher.decrypt(&encrypted, None)?;
    assert_eq!(plaintext, &decrypted_any[..]);

    Ok(())
}
```

#### ED25519 Encryption (IoT / Home Assistant Compatible)

```rust
use libcps::crypto::{Cipher, EncryptionAlgorithm, CryptoScheme};

fn encrypt_ed25519_example() -> anyhow::Result<()> {
    // Create cipher with ED25519 scheme
    let sender_cipher = Cipher::new(
        "//Alice".to_string(),
        EncryptionAlgorithm::AesGcm256,
        CryptoScheme::Ed25519
    )?;

    let receiver_cipher = Cipher::new(
        "//Bob".to_string(),
        EncryptionAlgorithm::AesGcm256,
        CryptoScheme::Ed25519
    )?;

    let plaintext = b"secret message for home assistant";
    let receiver_public = receiver_cipher.public_key();

    // Encrypt with ED25519
    let encrypted = sender_cipher.encrypt(plaintext, &receiver_public)?;

    // Decrypt
    let decrypted = receiver_cipher.decrypt(&encrypted, None)?;

    assert_eq!(plaintext, &decrypted[..]);
    Ok(())
}
```

### MQTT Configuration Example

```rust
use libcps::mqtt::Config as MqttConfig;

let mqtt_config = MqttConfig {
    broker: "mqtt://localhost:1883".to_string(),
    username: Some("user".to_string()),
    password: Some("pass".to_string()),
    client_id: Some("my-client".to_string()),
};
```

## 🚀 CLI Quick Start

### 1. Set up your environment

```bash
# Set blockchain endpoint
export ROBONOMICS_WS_URL=ws://localhost:9944

# Set your account (development account for testing)
export ROBONOMICS_SURI=//Alice

# Optional: Set MQTT broker
export ROBONOMICS_MQTT_BROKER=mqtt://localhost:1883
```

### 2. Create your first node

```bash
# Create a root node
cps create --meta '{"type":"building","name":"HQ"}' --payload '{"status":"online"}'

# Create a child node
cps create --parent 0 --meta '{"type":"room","name":"Server Room"}' --payload '{"temp":"22C"}'
```

### 3. View your CPS tree

```bash
cps show 0
```

Output:
```
🌳 CPS Node ID: 0

├─ 📝 Owner: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
├─ 📊 Meta: {
     "type": "building",
     "name": "HQ"
   }
└─ 🔐 Payload: {
     "status": "online"
   }

   👶 Children: (1 nodes)
      └─ NodeId: 1
```

## 📖 Commands

### `show <node_id>`

Display node information and its children in a beautiful tree format.

```bash
# Show node 0
cps show 0

# Show node with decryption attempt
cps show 5 --decrypt
```

### `create`

Create a new node (root or child).

```bash
# Create root node
cps create --meta '{"type":"sensor"}' --payload '22.5C'

# Create child node
cps create --parent 0 --payload 'operational data'

# Create with encryption (SR25519, default)
cps create --parent 0 --payload 'secret data' --receiver-public <RECEIVER_ADDRESS>

# Create with ED25519 encryption
cps create --parent 0 --payload 'secret data' --receiver-public <RECEIVER_ADDRESS> --scheme ed25519

# Create with specific cipher
cps create --parent 0 --payload 'secret data' --receiver-public <RECEIVER_ADDRESS> --cipher aesgcm256
```

**Options:**
- `--parent <id>`: Parent node ID (omit for root node)
- `--meta <data>`: Metadata (configuration data)
- `--payload <data>`: Payload (operational data)
- `--receiver-public <address>`: Receiver public key or SS58 address for encryption (required to encrypt data)
- `--cipher <algorithm>`: Encryption algorithm (xchacha20, aesgcm256, chacha20) [default: xchacha20]
- `--scheme <type>`: Cryptographic scheme (sr25519, ed25519) [default: sr25519]

### `set-meta <node_id> <data>`

Update node metadata.

```bash
# Update metadata
cps set-meta 5 '{"name":"Updated Sensor"}'

# Update with encryption
cps set-meta 5 'private config' --receiver-public <RECEIVER_ADDRESS>

# Update with ED25519 encryption
cps set-meta 5 'private config' --receiver-public <RECEIVER_ADDRESS> --scheme ed25519
```

### `set-payload <node_id> <data>`

Update node payload (operational data).

```bash
# Update temperature reading
cps set-payload 5 '23.1C'

# Update with encryption
cps set-payload 5 'encrypted telemetry' --receiver-public <RECEIVER_ADDRESS>

# Update with ED25519 and AES-GCM
cps set-payload 5 'encrypted telemetry' --receiver-public <RECEIVER_ADDRESS> --scheme ed25519 --cipher aesgcm256
```

### `move <node_id> <new_parent_id>`

Move a node to a new parent.

```bash
# Move node 5 under node 3
cps move 5 3
```

**Features:**
- Automatic cycle detection (prevents moving a node under its own descendant)
- Path validation

### `remove <node_id>`

Delete a node (must have no children).

```bash
# Remove node with confirmation
cps remove 5

# Remove without confirmation
cps remove 5 --force
```

### `mqtt subscribe <topic> <node_id>`

Subscribe to MQTT topic and update node payload with received messages.

```bash
# Subscribe to sensor data
cps mqtt subscribe "sensors/temp01" 5

# Subscribe with encryption (SR25519)
cps mqtt subscribe "sensors/temp01" 5 --receiver-public <RECEIVER_ADDRESS>

# Subscribe with ED25519 encryption (Home Assistant compatible)
cps mqtt subscribe "homeassistant/sensor/temp" 5 --receiver-public <RECEIVER_ADDRESS> --scheme ed25519

# Subscribe with specific cipher
cps mqtt subscribe "sensors/temp01" 5 --receiver-public <RECEIVER_ADDRESS> --cipher aesgcm256
```

**Behavior:**
- Connects to MQTT broker
- Subscribes to specified topic
- On each message: updates node payload
- Displays colorful logs for each update

**Example output:**
```
📡 Connecting to MQTT broker...
✅ Connected to mqtt://localhost:1883
📥 Subscribed to topic: sensors/temp01
🔄 Listening for messages...

[2025-12-04 10:30:15] 📨 Received: 22.5C
✅ Updated node 5 payload

[2025-12-04 10:30:45] 📨 Received: 23.1C
✅ Updated node 5 payload
```

### `mqtt publish <topic> <node_id>`

Monitor node payload and publish changes to MQTT topic.

```bash
# Publish node changes
cps mqtt publish "actuators/valve01" 10

# Publish with custom polling interval (seconds)
cps mqtt publish "actuators/valve01" 10 --interval 5
```

**Behavior:**
- Polls node payload every N seconds (default: 5)
- Publishes to MQTT when payload changes
- Automatically decrypts encrypted payloads

## ⚙️ Configuration

### Environment Variables

```bash
# Blockchain connection
export ROBONOMICS_WS_URL=ws://localhost:9944

# Account credentials
export ROBONOMICS_SURI=//Alice
# Or use a seed phrase:
# export ROBONOMICS_SURI="your twelve word seed phrase here goes like this"

# MQTT configuration
export ROBONOMICS_MQTT_BROKER=mqtt://localhost:1883
export ROBONOMICS_MQTT_USERNAME=myuser
export ROBONOMICS_MQTT_PASSWORD=mypass
export ROBONOMICS_MQTT_CLIENT_ID=cps-cli
```

### CLI Arguments (override environment variables)

```bash
cps --ws-url ws://localhost:9944 \
    --suri //Alice \
    --mqtt-broker mqtt://localhost:1883 \
    --mqtt-username myuser \
    --mqtt-password mypass \
    show 0
```

## 🔐 Encryption Requirements

### What You Need

To encrypt data, you **MUST provide BOTH**:

1. **Sender's Seed Phrase** (your account):
   - Via `--suri` CLI argument, OR
   - Via `ROBONOMICS_SURI` environment variable
   - Example: `//Alice`, `//Bob`, or a 12/24-word seed phrase

2. **Receiver's Public Key** (who can decrypt):
   - Via `--receiver-public` CLI argument
   - Supports SS58 addresses or hex-encoded public keys
   - Example: `5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty`

### Quick Example

```bash
# Setup sender credentials
export ROBONOMICS_SURI="//Alice"

# Encrypt payload for Bob
cps create \
  --payload 'secret data' \
  --receiver-public 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty
```

### What Happens Without Encryption

If you **omit** `--receiver-public`, data is stored as **plaintext** (no encryption):

```bash
# This creates plaintext data (visible to everyone)
cps create --payload 'public data'
```

### Error Messages

- `"SURI required for encryption"` → Set `--suri` or `ROBONOMICS_SURI`
- `"Invalid receiver address"` → Check the SS58 address or hex format

### See Also

- [Examples directory](examples/) for working scripts
- [Encryption section](#-encryption) for algorithm details

## 🔐 Encryption

The CLI supports multiple cryptographic schemes and AEAD encryption algorithms:

### Cryptographic Schemes

The library supports two cryptographic schemes for encryption:

#### **SR25519** (Default - Substrate Native)
- Uses Ristretto255 curve for ECDH
- Native to Substrate/Polkadot ecosystem
- Best for: Substrate blockchain operations
- Key agreement: Ristretto255 scalar multiplication

#### **ED25519** (IoT Compatible)
- Uses X25519 ECDH (ED25519 → Curve25519 conversion)
- Compatible with standard ED25519 implementations
- Best for: IoT devices, Home Assistant integration, standard cryptography
- Key agreement: ED25519 → Curve25519 → X25519

### Encryption Algorithms

Three AEAD ciphers are supported:

1. **XChaCha20-Poly1305** (Default)
   - 24-byte nonce (collision-resistant)
   - ~680 MB/s software performance
   - Best for: General purpose, portable

2. **AES-256-GCM**
   - 12-byte nonce
   - ~2-3 GB/s with AES-NI hardware acceleration
   - Best for: High throughput with hardware support

3. **ChaCha20-Poly1305**
   - 12-byte nonce
   - ~600 MB/s software performance
   - Best for: Portable performance without hardware acceleration

### How it works

1. **Key Derivation (ECDH + HKDF)**
   - For SR25519: Derive shared secret using Ristretto255 ECDH
   - For ED25519: Derive shared secret using X25519 ECDH
   - Apply HKDF-SHA256 with algorithm-specific info string

2. **Encryption (AEAD)**
   - Encrypt data with derived 32-byte key
   - Generate random nonce per message (size varies by algorithm)
   - Add authentication tag (AEAD)

3. **Message Format**
   ```json
   {
     "version": 1,
     "algorithm": "xchacha20",
     "from": "5GrwvaEF...",
     "nonce": "base64-encoded",
     "ciphertext": "base64-encoded"
   }
   ```

### Sender Verification

Decryption supports **optional sender verification** for enhanced security:

- **With verification** (recommended): Verifies the message sender's identity before decrypting
- **Without verification**: Decrypts messages from any sender (useful for anonymous scenarios)

```bash
# Encrypt with SR25519 (default) and XChaCha20 (default)
cps create --payload 'secret data' --receiver-public <RECEIVER_ADDRESS>

# Encrypt with ED25519 cryptographic scheme
cps create --payload 'secret data' --receiver-public <RECEIVER_ADDRESS> --scheme ed25519

# Encrypt with different algorithm
cps create --payload 'secret data' --receiver-public <RECEIVER_ADDRESS> --cipher aesgcm256

# Combine scheme and cipher selection
cps create --payload 'secret data' --receiver-public <RECEIVER_ADDRESS> --scheme ed25519 --cipher aesgcm256

# Decrypt when viewing
cps show 5 --decrypt --scheme sr25519

# The CLI always performs sender verification when available
```

### Library Usage

```rust
use libcps::crypto::{Cipher, EncryptionAlgorithm, CryptoScheme};

// Create a Cipher instance
let cipher = Cipher::new(
    "//Alice".to_string(),
    EncryptionAlgorithm::XChaCha20Poly1305,
    CryptoScheme::Sr25519,
)?;

// Get public key
let my_public = cipher.public_key();

// Encrypt data
let plaintext = b"Hello, World!";
let receiver_public = [0u8; 32]; // receiver's public key
let encrypted = cipher.encrypt(plaintext, &receiver_public)?;

// Decrypt data
let decrypted = cipher.decrypt(&encrypted, None)?;
```

### Home Assistant Compatibility

When using with Home Assistant Robonomics integration, use ED25519 cryptographic scheme:

```bash
# Subscribe to Home Assistant data with ED25519
cps mqtt subscribe "homeassistant/sensor/temperature" 5 --receiver-public <RECEIVER_ADDRESS> --scheme ed25519

# Update node with ED25519 encryption
cps set-payload 5 "encrypted data" --receiver-public <RECEIVER_ADDRESS> --scheme ed25519
```

## 📡 MQTT Bridge

The MQTT bridge enables seamless IoT integration:

### Subscribe: MQTT → Blockchain

```bash
cps mqtt subscribe "sensors/temperature" 5
```

Flow:
```
MQTT Topic → CPS CLI → Blockchain Node
    ↓             ↓            ↓
"22.5C"      Receive      Update Payload
```

### Publish: Blockchain → MQTT

```bash
cps mqtt publish "actuators/valve" 10 --interval 5
```

Flow:
```
Blockchain Node → CPS CLI → MQTT Topic
       ↓             ↓           ↓
  Payload      Poll every   Publish on
   Change       5 seconds     change
```

## 🎯 Use Cases

### 1. IoT Sensor Network

```bash
# Create building structure
cps create --meta '{"type":"building"}'
cps create --parent 0 --meta '{"type":"floor","number":1}'
cps create --parent 1 --meta '{"type":"room","name":"Server Room"}'

# Bridge sensor data
cps mqtt subscribe "sensors/room1/temp" 2
cps mqtt subscribe "sensors/room1/humidity" 2
```

### 2. Smart Home Automation

```bash
# Create home hierarchy
cps create --meta '{"type":"home"}'
cps create --parent 0 --meta '{"type":"room","name":"Kitchen"}'
cps create --parent 1 --meta '{"type":"device","name":"Smart Light"}'

# Control devices
cps mqtt publish "devices/kitchen/light/state" 2
```

### 3. Industrial Monitoring

```bash
# Create factory structure
cps create --meta '{"type":"factory"}'
cps create --parent 0 --meta '{"type":"line","name":"Assembly Line 1"}'
cps create --parent 1 --meta '{"type":"machine","id":"CNC-001"}'

# Monitor machine data with encryption
cps mqtt subscribe "machines/cnc001/telemetry" 2 --receiver-public <RECEIVER_ADDRESS>
```

## 🛠️ Development

### Project Structure

```
tools/cps/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs           # CLI entry point
    ├── types.rs          # CPS pallet types
    ├── commands/         # Command implementations
    │   ├── mod.rs
    │   ├── show.rs
    │   ├── create.rs
    │   ├── set_meta.rs
    │   ├── set_payload.rs
    │   ├── move_node.rs
    │   ├── remove.rs
    │   └── mqtt.rs
    ├── crypto/           # Encryption utilities
    │   ├── mod.rs
    │   └── xchacha20.rs
    ├── blockchain/       # Blockchain client
    │   ├── mod.rs
    │   └── client.rs
    ├── mqtt/             # MQTT client
    │   ├── mod.rs
    │   └── bridge.rs
    └── display/          # Pretty output
        ├── mod.rs
        └── tree.rs
```

### Building

```bash
cargo build --package robonomics-cps-cli
```

### Testing

```bash
cargo test --package robonomics-cps-cli
```

### Generating Blockchain Metadata

When connected to a running Robonomics node:

```bash
# Install subxt-cli
cargo install subxt-cli

# Generate metadata
subxt metadata --url ws://localhost:9944 > metadata.scale

# Generate Rust code
subxt codegen --file metadata.scale > src/robonomics_runtime.rs
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

Apache-2.0

## 🔗 Links

- [Robonomics Network](https://robonomics.network)
- [Documentation](https://wiki.robonomics.network)
- [GitHub](https://github.com/airalab/robonomics)

## 💡 Tips

- Use `//Alice`, `//Bob`, etc. for development accounts
- Always backup your seed phrase in production
- Test encryption with development keys first
- Monitor MQTT bridge logs for debugging
- Use `--help` on any command for more details

## 🐛 Troubleshooting

### Connection Failed

```bash
# Check if node is running
curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' http://localhost:9944

# Try default WebSocket URL
cps --ws-url ws://127.0.0.1:9944 show 0
```

### Account Not Found

```bash
# Make sure SURI is set
export ROBONOMICS_SURI=//Alice

# Or pass it directly
cps --suri //Alice create --meta '{"test":true}'
```

### MQTT Connection Issues

```bash
# Test MQTT broker
mosquitto_pub -h localhost -t test -m "hello"

# Check broker URL format
export ROBONOMICS_MQTT_BROKER=mqtt://localhost:1883
```

---

Made with ❤️ by the Robonomics Team
