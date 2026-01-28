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
- 📡 **MQTT bridge** for IoT device integration (optional feature)
- 🌲 **Hierarchical tree visualization** of CPS nodes (CLI)
- ⚙️ **Flexible configuration** via environment variables or CLI args
- 🔒 **Secure by design** with proper key management and ECDH key agreement
- 📚 **Comprehensive documentation** for library API
- 🔧 **Type-safe blockchain integration** via subxt
- 🎛️ **Feature flags** for flexible dependency management

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                    libcps CLI                       │
├─────────────────────────────────────────────────────┤
│  Commands │ Display │ Crypto │ Blockchain │ MQTT   │
└─────────────────────────────────────────────────────┘
      ↓           ↓         ↓         ↓          ↓
┌─────────────────────────────────────────────────────┐
│                 libcps Library                      │
├──────────────┬──────────────┬──────────────────────┤
│   Cipher     │  Types       │  Generated Runtime   │
│   - SR25519  │  - NodeData  │  - subxt codegen     │
│   - ED25519  │  - NodeId    │  - CPS pallet API    │
└──────────────┴──────────────┴──────────────────────┘
      ↓                              ↓
┌─────────────────────┐    ┌─────────────────────────┐
│  Substrate Node     │    │    MQTT Broker          │
│  - CPS Pallet       │    │  - rumqttc client       │
└─────────────────────┘    └─────────────────────────┘
```

## 📦 Installation

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
libcps = "0.1.0"
```

#### Feature Flags

The library supports optional feature flags for flexible dependency management:

- **`mqtt`** - Enables MQTT bridge functionality (enabled by default)
- **`cli`** - Enables CLI binary with colored output (enabled by default)

```toml
# Default: all features enabled
[dependencies]
libcps = "0.1.0"

# Library only, without MQTT
[dependencies]
libcps = { version = "0.1.0", default-features = false }

# Library with MQTT only (no CLI)
[dependencies]
libcps = { version = "0.1.0", default-features = false, features = ["mqtt"] }
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
[*] CPS Node ID: 0

|--  [O] Owner: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
|--  [M] Meta: {
  "type": "building",
  "name": "HQ"
}
`--  [P] Payload: {
  "status": "online"
}

    [C] Children: (1 nodes)
      `-- Node: 1
```

## 📚 Library Usage

### Quick Start

This example shows the core node-oriented operations: creating nodes, setting metadata and payload, and visualizing the tree structure.

```rust
use libcps::{Client, Config, node::Node, types::NodeData};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to blockchain
    let config = Config {
        ws_url: "ws://localhost:9944".to_string(),
        suri: Some("//Alice".to_string()),
    };
    let client = Client::new(&config).await?;
    
    // Create a root node with metadata and payload
    let meta: NodeData = r#"{"type":"building","name":"HQ"}"#.into();
    let payload: NodeData = r#"{"status":"online"}"#.into();
    let root_node = Node::create(&client, None, Some(meta), Some(payload)).await?;
    println!("Created root node: {}", root_node.id());
    
    // Create a child node
    let child_meta: NodeData = r#"{"type":"room","name":"Server Room"}"#.into();
    let child_payload: NodeData = r#"{"temp":"22C"}"#.into();
    let child_node = Node::create(&client, Some(root_node.id()), Some(child_meta), Some(child_payload)).await?;
    println!("Created child node: {}", child_node.id());
    
    // Update node metadata
    let new_meta: NodeData = r#"{"type":"room","name":"Server Room","updated":true}"#.into();
    child_node.set_meta(Some(new_meta)).await?;
    
    // Update node payload
    let new_payload: NodeData = r#"{"temp":"23.5C"}"#.into();
    child_node.set_payload(Some(new_payload)).await?;
    
    // Query and display node information
    let info = root_node.query().await?;
    println!("Node {} has {} children", info.id, info.children.len());
    
    Ok(())
}
```

**Tree Visualization Output:**

When you query a node, the tree is displayed with the new visual format:

```
[*] CPS Node ID: 0

|--  [O] Owner: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
|--  [M] Meta: {
  "type": "building",
  "name": "HQ"
}
`--  [P] Payload: {
  "status": "online"
}

    [C] Children: (1 nodes)
      `-- Node: 1
```

### Data Types

```rust
use libcps::types::{NodeData, NodeId};

// Create plain data (unencrypted)
let meta = NodeData::from("sensor config");
let meta_bytes = NodeData::from(vec![1, 2, 3]);

// Create encrypted data from cipher output
let encrypted_msg = cipher.encrypt(plaintext, &receiver_public, EncryptionAlgorithm::XChaCha20Poly1305)?;
let encrypted_bytes = encrypted_msg.encode();
let payload = NodeData::aead_from(encrypted_bytes);
```

### Encryption Examples

#### SR25519 Encryption (Substrate Native)

```rust
use libcps::crypto::{Cipher, EncryptionAlgorithm, CryptoScheme};

fn encrypt_sr25519_example() -> anyhow::Result<()> {
    // Create ciphers for Alice and Bob
    let alice = Cipher::new(
        "//Alice".to_string(),
        CryptoScheme::Sr25519,
    )?;

    let bob = Cipher::new(
        "//Bob".to_string(),
        CryptoScheme::Sr25519,
    )?;

    // Encrypt from Alice to Bob
    let plaintext = b"secret message";
    let encrypted = alice.encrypt(plaintext, &bob.public_key(), EncryptionAlgorithm::XChaCha20Poly1305)?;

    // Decrypt with sender verification (recommended)
    let decrypted = bob.decrypt(&encrypted, Some(&alice.public_key()))?;
    assert_eq!(plaintext, &decrypted[..]);

    // Decrypt without sender verification (accepts from any sender)
    let decrypted_any = bob.decrypt(&encrypted, None)?;
    assert_eq!(plaintext, &decrypted_any[..]);

    Ok(())
}
```

#### ED25519 Encryption (IoT / Home Assistant Compatible)

```rust
use libcps::crypto::{Cipher, EncryptionAlgorithm, CryptoScheme};

fn encrypt_ed25519_example() -> anyhow::Result<()> {
    // Create ciphers with ED25519 scheme
    let alice = Cipher::new(
        "//Alice".to_string(),
        CryptoScheme::Ed25519,
    )?;

    let bob = Cipher::new(
        "//Bob".to_string(),
        CryptoScheme::Ed25519,
    )?;

    // Encrypt from Alice to Bob
    let plaintext = b"secret message for home assistant";
    let encrypted = alice.encrypt(plaintext, &bob.public_key(), EncryptionAlgorithm::AesGcm256)?;

    // Decrypt with sender verification
    let decrypted = bob.decrypt(&encrypted, Some(&alice.public_key()))?;
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

### MQTT Bridge (Library Usage)

The MQTT bridge can be used directly from library code:

```rust
use libcps::{mqtt, Config as BlockchainConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let blockchain_config = BlockchainConfig {
        ws_url: "ws://localhost:9944".to_string(),
        suri: Some("//Alice".to_string()),
    };

    let mqtt_config = mqtt::Config {
        broker: "mqtt://localhost:1883".to_string(),
        username: None,
        password: None,
        client_id: Some("cps-subscriber".to_string()),
        blockchain: None,
        subscribe: Vec::new(),
        publish: Vec::new(),
    };

    // Subscribe to MQTT and update blockchain using Config method
    mqtt_config.subscribe(
        &blockchain_config,
        None,  // No encryption
        "sensors/temperature",
        42,    // node_id
        None,  // No receiver public key
        None,  // No custom message handler
    ).await?;

    Ok(())
}
```

For detailed MQTT examples, see [`examples/mqtt_bridge.rs`](examples/mqtt_bridge.rs).

### Debugging and Logging

The library uses the [`log`](https://docs.rs/log) crate for comprehensive logging throughout all operations. This makes debugging easy and allows you to trace every step of encryption, blockchain interaction, and MQTT communication.

#### Logging Levels

The library uses these log levels:
- **`trace`**: Very detailed logs (function entry/exit, data hex dumps, step-by-step operations)
- **`debug`**: Operation details (algorithms chosen, data sizes, IDs, success messages)
- **`warn`**: Recoverable issues (fallback to plaintext, retries)
- **`error`**: Failures (encryption errors, connection failures)

#### Enabling Logging

Use any log implementation like `env_logger` or `tracing`:

```rust
use libcps::{Client, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger (example with env_logger)
    env_logger::init();
    
    let config = Config {
        ws_url: "ws://localhost:9944".to_string(),
        suri: Some("//Alice".to_string()),
    };
    
    let client = Client::new(&config).await?;
    // Now you'll see debug logs!
    
    Ok(())
}
```

#### Set Log Level via Environment

```bash
# Show all debug and trace logs
RUST_LOG=trace cargo run

# Show only libcps logs at debug level
RUST_LOG=libcps=debug cargo run

# Show only crypto operations
RUST_LOG=libcps::crypto=trace cargo run

# Show specific modules
RUST_LOG=libcps::mqtt=debug,libcps::crypto=trace cargo run
```

#### Example Trace Output

```
TRACE libcps::blockchain: Connecting to blockchain at ws://localhost:9944
DEBUG libcps::blockchain: Successfully connected to blockchain
TRACE libcps::crypto: Creating new Cipher with scheme: Sr25519
DEBUG libcps::crypto: SR25519 keypair created successfully
DEBUG libcps::crypto: Encrypting 42 bytes with XChaCha20Poly1305 using Sr25519 scheme
TRACE libcps::crypto: Deriving shared secret via ECDH
TRACE libcps::crypto: Encryption key derived
TRACE libcps::crypto: Generated XChaCha20 nonce: 24 bytes
DEBUG libcps::crypto: Encryption complete: 42 bytes plaintext -> 58 bytes ciphertext (+ 16 bytes overhead)
DEBUG libcps::mqtt: Starting MQTT subscribe bridge: topic='sensors/temp', node=5
TRACE libcps::mqtt: Received MQTT message on topic 'sensors/temp': 42 bytes
DEBUG libcps::node: Setting payload for node 5: has_data=true
TRACE libcps::node: Building set_payload transaction
DEBUG libcps::node: Payload updated successfully for node 5
```

This comprehensive logging makes it easy to:
- Debug encryption/decryption operations
- Monitor performance (data sizes at each step)
- Audit security operations
- Troubleshoot MQTT connectivity
- Track blockchain transactions

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

Monitor node payload and publish changes to MQTT topic using event-driven architecture.

```bash
# Publish node changes
cps mqtt publish "actuators/valve01" 10
```

**Behavior:**
- Event-driven monitoring (subscribes to blockchain events)
- Only queries and publishes when payload actually changes
- Automatically decrypts encrypted payloads

See [MQTT Bridge](#-mqtt-bridge) section for detailed technical implementation.

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
- [Encryption section](#encryption) for algorithm details

## 🔐 Encryption

The library supports multiple cryptographic schemes and AEAD encryption algorithms with robust key derivation and self-describing message format.

### Cryptographic Schemes

Two cryptographic schemes are supported for ECDH key agreement:

| Feature | SR25519 | ED25519 |
|---------|---------|---------|
| **Curve** | Ristretto255 | Curve25519 (via X25519) |
| **ECDH** | Ristretto255 scalar multiplication | ED25519 → X25519 |
| **Best For** | Substrate blockchain operations | IoT devices, Home Assistant |
| **Compatibility** | Native to Polkadot ecosystem | Standard ED25519 implementations |
| **Key Agreement** | `scalar * point` on Ristretto255 | ED25519 → Curve25519 → X25519 |

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

3. **Self-Describing Message Format**
   
   The encrypted message uses SCALE codec for efficient binary serialization on the blockchain.
   The message format is defined as a versioned Rust enum:
   
   ```rust
   pub enum EncryptedMessage {
       V1 {
           algorithm: EncryptionAlgorithm,  // XChaCha20Poly1305, AesGcm256, or ChaCha20Poly1305
           from: [u8; 32],                  // Sender's 32-byte public key
           nonce: Vec<u8>,                  // 24 bytes for XChaCha20, 12 for AES-GCM/ChaCha20
           ciphertext: Vec<u8>,             // Encrypted data with authentication tag
       }
   }
   ```
   
   The message is serialized using **SCALE codec** (Simple Concatenated Aggregate Little-Endian),
   the native encoding format for Substrate blockchains, providing:
   
   - **Blockchain efficiency**: Compact binary format minimizes on-chain storage costs
   - **Automatic algorithm detection**: Receiver knows which cipher to use from the enum
   - **Sender identification**: The `from` field contains sender's raw 32-byte public key
   - **Version compatibility**: Enum variants enable future protocol upgrades
   - **Type safety**: Compile-time guarantee of message structure validity with Encode/Decode derives
   - **Future-proof**: New versions can be added as additional enum variants (e.g., `V2 { ... }`)
   - **Native integration**: SCALE codec is the standard for all Substrate/Polkadot data


### Key Derivation (HKDF-SHA256)

The encryption scheme uses HKDF (RFC 5869) for deriving encryption keys from shared secrets:

#### Process:

1. **ECDH Key Agreement**
   - SR25519: Ristretto255 scalar multiplication
   - ED25519: X25519 (ED25519 → Curve25519 → X25519)
   - Result: 32-byte shared secret

2. **HKDF Extract**
   ```
   salt = "robonomics-network"  (constant, for domain separation)
   PRK = HMAC-SHA256(salt, shared_secret)
   ```

3. **HKDF Expand**
   ```
   info = algorithm-specific string:
     - "robonomics-cps-xchacha20poly1305"
     - "robonomics-cps-aesgcm256"
     - "robonomics-cps-chacha20poly1305"
   
   OKM = HMAC-SHA256(PRK, info)[0..32]
   ```

#### Security Properties:
- **Domain Separation**: Keys bound to Robonomics network context
- **Algorithm Binding**: Different algorithms produce independent keys
- **Key Independence**: Each (shared_secret, algorithm) pair → unique key
- **Security Enhancement**: Constant salt strengthens key derivation even with low-entropy secrets

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

### CLI Address Formats

The `--receiver-public` parameter accepts both:

1. **SS58 Address** (recommended):
   ```bash
   cps create --payload 'data' --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
   ```

2. **Hex-encoded public key** (32 bytes, optional `0x` prefix):
   ```bash
   cps create --payload 'data' --receiver-public 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
   cps create --payload 'data' --receiver-public d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
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

The MQTT bridge enables seamless IoT integration with real-time, event-driven synchronization. The bridge functionality is available both as a CLI command and as a library API.

### Library API

The MQTT bridge can be used programmatically from your Rust applications:

```rust
use libcps::{mqtt, Config as BlockchainConfig};

// Subscribe Bridge: MQTT → Blockchain
// Using Config method API
mqtt_config.subscribe(
    &blockchain_config,
    None,              // Optional encryption cipher
    "sensors/temp",    // MQTT topic
    1,                 // Node ID
    None,              // Optional receiver public key
    None,              // Optional message handler callback
).await?;

// Publish Bridge: Blockchain → MQTT
// Using Config method API
mqtt_config.publish(
    &blockchain_config,
    None,               // Optional cipher for decryption
    "actuators/status", // MQTT topic
    1,                  // Node ID
    None,               // Optional publish handler callback
).await?;
```

**Library Features:**
- ✅ Reusable API for custom applications
- ✅ Optional callbacks for message handling and logging
- ✅ Resilient error handling (continues on transient failures)
- ✅ Encryption support (SR25519/ED25519)
- ✅ Auto-reconnection on connection failures

See [`examples/mqtt_bridge.rs`](examples/mqtt_bridge.rs) for a complete working example.

### Configuration File

You can manage multiple bridges using a TOML configuration file. This is ideal for running multiple subscribe and publish bridges concurrently.

#### CLI Usage

```bash
# Start all bridges from config file
cps mqtt start -c mqtt_config.toml

# With custom config path
cps mqtt start --config /etc/cps/mqtt-bridge.toml
```

#### Configuration File Format

```toml
# MQTT Broker Configuration
broker = "mqtt://localhost:1883"
username = "myuser"  # Optional
password = "mypass"  # Optional
client_id = "cps-bridge"  # Optional

# Blockchain Configuration
[blockchain]
ws_url = "ws://localhost:9944"
suri = "//Alice"

# Subscribe Topics (MQTT → Blockchain)
[[subscribe]]
topic = "sensors/temperature"
node_id = 5

[[subscribe]]
topic = "sensors/humidity"
node_id = 6
receiver_public = "5GrwvaEF..."  # Optional encryption
cipher = "xchacha20"  # Optional
scheme = "sr25519"  # Optional

# Publish Topics (Blockchain → MQTT)
[[publish]]
topic = "actuators/valve01"
node_id = 10

[[publish]]
topic = "actuators/fan"
node_id = 11

# Publish with decryption (reads encrypted blockchain data, publishes decrypted to MQTT)
# Algorithm and scheme are auto-detected from the encrypted data
[[publish]]
topic = "decrypted/sensor/data"
node_id = 13
decrypt = true
```

See [`examples/mqtt_config.toml`](examples/mqtt_config.toml) for a complete example.

#### Library Usage

```rust
use libcps::mqtt::Config;

// Load config from file
let config = Config::from_file("mqtt_config.toml")?;

// Start all bridges
config.start().await?;
```

**Features:**
- ✅ Manage multiple bridges from a single file
- ✅ Concurrent execution of all bridges
- ✅ Per-topic encryption configuration
- ✅ Easy deployment and version control
- ✅ No need to manage multiple CLI processes

### Subscribe: MQTT → Blockchain

Subscribe to MQTT topics and automatically update blockchain node payload with received messages.

#### CLI Usage

```bash
# Basic subscription
cps mqtt subscribe "sensors/temperature" 5

# With SR25519 encryption (default)
cps mqtt subscribe "sensors/temperature" 5 \
    --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

# With ED25519 encryption (Home Assistant compatible)
cps mqtt subscribe "homeassistant/sensor/temperature" 5 \
    --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \
    --scheme ed25519

# With AES-GCM cipher
cps mqtt subscribe "sensors/temperature" 5 \
    --receiver-public 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \
    --cipher aesgcm256
```

#### Library Usage

```rust
use libcps::{mqtt, Config as BlockchainConfig};

// Create a custom message handler for logging
let handler = Box::new(|topic: &str, payload: &[u8]| {
    println!("📥 Received on {}: {:?}", topic, payload);
});

// Using Config method API
mqtt_config.subscribe(
    &blockchain_config,
    None,              // No encryption
    "sensors/temp",
    1,                 // node_id
    None,              // No receiver public key
    Some(handler),     // Custom message handler
).await?;
```

**Features:**
- ✅ Real-time message processing
- ✅ Auto-reconnect on connection failures
- ✅ Optional encryption support (SR25519/ED25519)
- ✅ Colorful console output with timestamps (CLI)
- ✅ Custom message handlers for library usage
- ✅ Multiple cipher algorithm support
- ✅ Resilient error handling (continues on transient failures)

**Flow:**
```
MQTT Topic → CPS CLI → Blockchain Node
    ↓             ↓            ↓
"22.5C"      Receive      Update Payload
                         (encrypted if configured)
```

### Publish: Blockchain → MQTT

Monitor blockchain node for payload changes and publish to MQTT topic in real-time using event-driven architecture.

#### CLI Usage

```bash
# Basic publishing
cps mqtt publish "actuators/valve" 10

# With decryption (auto-detects algorithm from encrypted data)
cps mqtt publish "sensors/encrypted" 10 --decrypt

# With custom broker configuration
cps mqtt publish "actuators/valve" 10 \
    --mqtt-broker mqtt://broker.example.com:1883 \
    --mqtt-username myuser \
    --mqtt-password mypass
```

#### Library Usage

```rust
use libcps::{mqtt, Config as BlockchainConfig, crypto::Cipher};

// Create cipher for decryption (optional)
let cipher = Cipher::new(
    "//Alice".to_string(),
    crypto::CryptoScheme::Sr25519
)?;

// Create a custom publish handler for logging
let handler = Box::new(|topic: &str, block_num: u32, data: &str| {
    println!("📤 Published to {} at block #{}: {}", topic, block_num, data);
});

// Using Config method API with decryption
mqtt_config.publish(
    &blockchain_config,
    Some(&cipher),     // Optional cipher for decryption
    "actuators/status",
    1,                 // node_id
    Some(handler),     // Custom publish handler
).await?;
```

**Technical Implementation:**
- Subscribes to finalized blockchain blocks
- Monitors `PayloadSet` events for target node
- Only queries and publishes when payload actually changes (event-driven)
- No polling overhead - reacts to blockchain events in real-time
- Publishes to MQTT with QoS 0 (At Most Once)
- Background event loop for MQTT auto-reconnection

**Flow:**
```text
Blockchain PayloadSet Event → Detect Change → Query Node → Publish to MQTT
          ↓                         ↓              ↓              ↓
   (detected via event)      (node_id match)  (at block #)  (changed data)
```

**Features:**
- ✅ Event-driven monitoring (no polling overhead)
- ✅ Monitors `PayloadSet` events in finalized blocks
- ✅ Only queries and publishes when payload actually changes
- ✅ Automatic decryption of encrypted payloads
- ✅ Auto-reconnect on connection failures
- ✅ Graceful shutdown handling
- ✅ Block number tracking in logs
- ✅ Custom publish handlers for library usage
- ✅ Resilient error handling (continues on transient failures)

### Example Output

**Subscribe Command:**
```
[~] Connecting to MQTT broker...
[+] Connected to mqtt://localhost:1883
[i] Subscribed to topic: sensors/temp01
[~] Listening for messages...

[2025-12-04 10:30:15] Received: 22.5C
[i] Encrypting with XChaCha20-Poly1305 using SR25519
[+] Updated node 5 payload

[2025-12-04 10:30:45] Received: 23.1C
[i] Encrypting with XChaCha20-Poly1305 using SR25519
[+] Updated node 5 payload
```

**Publish Command:**
```
[~] Connecting to blockchain...
[+] Connected to ws://localhost:9944
[~] Connecting to MQTT broker localhost:1883...
[+] Connected to mqtt://localhost:1883
[i] Monitoring node 10 payload on each block...

[2025-12-04 10:31:20] Published to actuators/valve01 at block #1234: open
[2025-12-04 10:31:50] Published to actuators/valve01 at block #1240: closed
```

### Authentication

Configure MQTT credentials via environment variables or CLI flags:

```bash
# Environment variables
export ROBONOMICS_MQTT_BROKER=mqtt://broker.example.com:1883
export ROBONOMICS_MQTT_USERNAME=myuser
export ROBONOMICS_MQTT_PASSWORD=mypassword
export ROBONOMICS_MQTT_CLIENT_ID=cps-client-01

# Or via CLI flags
cps mqtt subscribe "topic" 5 \
    --mqtt-broker mqtt://broker.example.com:1883 \
    --mqtt-username myuser \
    --mqtt-password mypassword
```

### Integration Examples

#### Home Assistant Integration

```bash
# Subscribe to Home Assistant sensor (ED25519 compatible)
cps mqtt subscribe "homeassistant/sensor/living_room/temperature" 100 \
    --receiver-public <HOME_ASSISTANT_PUBLIC_KEY> \
    --scheme ed25519 \
    --cipher aesgcm256

# Publish to Home Assistant actuator
cps mqtt publish "homeassistant/switch/kitchen/light" 101
```

#### Industrial IoT

```bash
# Monitor encrypted machine telemetry
cps mqtt subscribe "factory/line1/cnc001/telemetry" 200 \
    --receiver-public <MACHINE_PUBLIC_KEY> \
    --cipher xchacha20

# Publish control commands
cps mqtt publish "factory/line1/controller/commands" 201
```

#### Smart Building

```bash
# Create building hierarchy
cps create --meta '{"type":"building","name":"HQ"}'               # Node 0
cps create --parent 0 --meta '{"type":"floor","number":1}'       # Node 1
cps create --parent 1 --meta '{"type":"room","name":"Server"}'   # Node 2

# Bridge temperature sensor
cps mqtt subscribe "building/floor1/server-room/temp" 2

# Monitor and publish HVAC status
cps mqtt publish "building/floor1/server-room/hvac" 2
```

### Error Handling

The MQTT bridge handles various error scenarios gracefully:

- **Connection Failures**: Auto-reconnect with 5-second delay
- **Invalid Messages**: Logged and skipped
- **Blockchain Errors**: Logged with timestamps
- **Encryption Errors**: Descriptive error messages
- **Graceful Shutdown**: Background tasks cleaned up on exit

### Performance Considerations

- **Event-Driven**: No unnecessary blockchain queries
- **Efficient**: Only processes blocks with relevant events  
- **Low Latency**: Real-time event detection
- **Resource Efficient**: Minimal memory footprint
- **Scalable**: Multiple instances can run simultaneously

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
