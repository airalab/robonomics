# 🌳 Robonomics CPS CLI

Beautiful, user-friendly command-line interface for managing hierarchical Cyber-Physical Systems on the Robonomics blockchain.

## ✨ Features

- 🎨 **Beautiful colored output** with emojis and ASCII art
- 🔐 **XChaCha20-Poly1305 encryption** with sr25519 key derivation
- 📡 **MQTT bridge** for IoT device integration
- 🌲 **Hierarchical tree visualization** of CPS nodes
- ⚙️ **Flexible configuration** via environment variables or CLI args
- 🔒 **Secure by design** with proper key management

## 📦 Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/airalab/robonomics
cd robonomics

# Build the CLI tool
cargo build --release --package robonomics-cps-cli

# The binary will be at: target/release/cps
```

### Add to PATH (optional)

```bash
sudo cp target/release/cps /usr/local/bin/
```

## 🚀 Quick Start

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

# Create with encryption
cps create --parent 0 --payload 'secret data' --encrypt
```

**Options:**
- `--parent <id>`: Parent node ID (omit for root node)
- `--meta <data>`: Metadata (configuration data)
- `--payload <data>`: Payload (operational data)
- `--encrypt`: Encrypt the data

### `set-meta <node_id> <data>`

Update node metadata.

```bash
# Update metadata
cps set-meta 5 '{"name":"Updated Sensor"}'

# Update with encryption
cps set-meta 5 'private config' --encrypt
```

### `set-payload <node_id> <data>`

Update node payload (operational data).

```bash
# Update temperature reading
cps set-payload 5 '23.1C'

# Update with encryption
cps set-payload 5 'encrypted telemetry' --encrypt
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

# Subscribe with encryption
cps mqtt subscribe "sensors/temp01" 5 --encrypt
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

## 🔐 Encryption

The CLI implements **sr25519 → XChaCha20-Poly1305** encryption:

### How it works

1. **Key Derivation (ECDH + HKDF)**
   - Derive shared secret using sr25519 ECDH
   - Apply HKDF-SHA256 with info string: `"robonomics-cps-xchacha20poly1305"`

2. **Encryption (XChaCha20-Poly1305)**
   - Encrypt data with derived 32-byte key
   - Generate random 24-byte nonce per message
   - Add authentication tag (AEAD)

3. **Message Format**
   ```json
   {
     "version": 1,
     "from": "5GrwvaEF...",
     "nonce": "base64-encoded-24-bytes",
     "ciphertext": "base64-encoded"
   }
   ```

### Usage

```bash
# Encrypt when creating node
cps create --payload 'secret data' --encrypt

# Decrypt when viewing
cps show 5 --decrypt
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
cps mqtt subscribe "machines/cnc001/telemetry" 2 --encrypt
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
