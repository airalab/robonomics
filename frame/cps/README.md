# Pallet CPS

**On-chain hierarchical organization for Cyber-Physical Systems**

## What is a Cyber-Physical System?

A Cyber-Physical System (CPS) bridges the digital and physical worlds by integrating computation, networking, and physical processes. Examples include:

- **Smart Manufacturing**: Robotic assembly lines coordinating production
- **Autonomous Vehicles**: Self-driving cars communicating with infrastructure
- **Smart Buildings**: HVAC, lighting, and security systems working together
- **Industrial IoT**: Sensor networks monitoring and optimizing processes
- **Medical Devices**: Connected healthcare equipment in hospitals

## Why Hierarchical Organization?

Real-world CPS naturally form hierarchies:

```
Smart Building (Root)
├── Floor 1
│   ├── HVAC Unit
│   │   ├── Compressor
│   │   └── Thermostat
│   └── Lighting Controller
└── Floor 2
    ├── HVAC Unit
    └── Security Camera
```

This pallet provides a decentralized, tamper-proof registry for such systems, enabling:
- **Verifiable ownership boundaries** for physical assets and sub-systems
- **Verifiable system topology** for audits and compliance
- **Secure data storage** with client-side encryption support
- **Immutable audit trails** of system changes

## Core Concepts

### Hierarchical Tree Structure

Nodes are organized in a parent-child tree. A node's `parent` is fixed at
creation time and never changes - there is no operation to relocate a node:

```
         [Building-A]
         /          \
    [Floor-1]    [Floor-2]
      /    \          |
[HVAC-01] [Lights] [HVAC-02]
```

**Benefits:**
- **Logical Grouping**: Related systems stay together
- **Efficient Queries**: Find all children of a node in O(1) time via `NodesByParent`
- **Structural Immutability**: The tree shape can only grow, never be rewired

### Ownership Model

Ownership is **not** stored on every node. Instead, an explicit `Ownership`
entry marks only the nodes that start a new administrative and
resource-accounting scope (an "Ownership boundary"); every other node
inherits ownership from the nearest ancestor boundary:

```
Building-A
Ownership(PropertyManager)
|
`-- Floor-3
    Ownership(TenantCorp)
    |
    `-- HVAC-Unit-07
        |
        `-- Thermostat-142
```

`HVAC-Unit-07` and `Thermostat-142` inherit ownership from `Floor-3`'s
`Ownership(TenantCorp)` boundary. `TenantCorp` has no implicit rights over
`Building-A`'s other independently owned floors, and `PropertyManager` has
no implicit rights inside `TenantCorp`'s boundary.

New boundaries are established purely through `transfer_ownership` (propose)
and `accept_ownership` (accept) - the same two-step flow used to hand
ownership of an existing boundary to another account. A "self-transfer"
(proposing and accepting to the same account) simply carves out a new,
independent boundary without changing the effective owner.

`Pallet::resolve_ownership(node_id)` is the single canonical resolver: it
returns the boundary root `NodeId` and the effective owner `AccountId` for
any node, and is used by every authorization check in this pallet.

### Data Model

Each node can independently carry two pieces of data, each stored in its own
map so unset fields cost no storage:

1. **Metadata** (`Meta`): System configuration, capabilities, specifications
2. **Payload** (`Payload`): Operational data, sensor readings, telemetry

Both are stored as plain bytes; for private data, encryption should happen
client-side before submission (see [Client-Side Encryption](#-client-side-encryption)).

```
Node: "Temperature Sensor"
├── Meta (plain): {"type": "thermocouple", "range": "-50 to 400°C"}
└── Payload (encrypted): Current reading + calibration data
```

## Real-World Use Cases

### Use Case 1: Supply Chain Tracking

A manufacturer tracks components through production:

```
Product Batch #12345
├── Component A (Supplier: ACME Corp)
│   └── Raw Material Certificate (encrypted)
├── Component B (Supplier: Beta LLC)
│   └── Quality Test Results (plain)
└── Assembly Record
    └── Worker ID + Timestamp (encrypted)
```

**Benefits**: Immutable provenance, encrypted sensitive data, transparent for auditors

### Use Case 2: Smart Building Management

A property manager leases floors to independent tenant companies, each
managing their own equipment:

```
Building-A
Ownership(PropertyManager)
├── Floor-1 (shared building systems)
│   ├── Fire Suppression Controller
│   └── Elevator Bank
└── Floor-3
    Ownership(TenantCorp)
    ├── HVAC-Unit-07
    │   └── Thermostat-142 (occupancy data, encrypted)
    └── Access Control Panel (badge logs, encrypted)
```

**Benefits**: `TenantCorp` manages Floor-3's equipment independently; `PropertyManager` retains full control of shared building systems and other floors without either party having implicit access to the other's boundary.

## How It Works

### Creating a System Hierarchy

1. **Start with a root node** representing your top-level system - the creator becomes its owner
2. **Add child nodes** for subsystems and components - the resolved owner of the parent authorizes creation
3. **Store data** as plain text (public) or client-side encrypted (private)
4. **Establish nested boundaries** with `transfer_ownership`/`accept_ownership` when a sub-tree needs independent administration

```
Step 1: Create Root          Step 2: Add Children        Step 3: Add Details
[Building-A]          →         [Building-A]        →       [Building-A]
                                /            \                /            \
                          [Floor-1]     [Floor-2]       [Floor-1]     [Floor-2]
                                                          /    \
                                                    [HVAC] [Lights]
```

### Tree Integrity Guarantees

The pallet enforces several invariants:

- **Structural Immutability**: `parent` never changes after creation, so cycles cannot be created
- **Ownership Resolution**: Every active node resolves to exactly one effective owner
- **Ownership Boundaries**: An explicit `Ownership` entry stops inheritance from ancestors
- **Depth Limits**: Trees cannot exceed `MaxTreeDepth`
- **Deletion Safety**: Nodes with children cannot be deleted

### Ownership Resolution: O(depth)

There is no cached ancestor list. `resolve_ownership` walks the single
`parent` link one hop at a time until it finds an explicit `Ownership` entry,
bounded by `MaxTreeDepth`:

```
Node C: parent = Some(B)  ─┐
Node B: parent = Some(A)   ├─ walked one hop at a time
Node A: parent = None      ┘  (root - always has an explicit Ownership entry)
```

**Trade-off**: No extra storage per node for ancestor tracking, at the cost
of O(depth) storage reads per authorization check (bounded and predictable,
since `depth < MaxTreeDepth`).

## Operations

### 🏗️ Create Node

Add a new node to your system hierarchy:

```
create_node(
  parent: Some(node_id),      // Link to parent (None for root)
  meta: Some(...),            // System configuration
  payload: Some(...)          // Operational data
)
```

**Example**: Adding a temperature sensor to a room:
```
parent: Room 101
meta: {"type": "temperature", "model": "DHT22"}
payload: {"reading": "22.5°C", "timestamp": "2025-01-15T10:30:00Z"}
```

Creating a root node (`parent: None`) makes the caller the explicit owner of
a new Ownership boundary. Creating a child node requires the caller to be the
resolved owner of the parent; the child inherits ownership and does not get
its own explicit entry.

### ✏️ Update Data

Modify metadata or payload without changing the hierarchy:

```
set_meta(node_id, new_metadata)    // Update configuration
set_payload(node_id, new_payload)  // Update operational data
```

**Example**: Sensor recalibration:
```
set_meta(sensor_id, {"type": "temperature", "model": "DHT22", "calibrated": "2025-01-15"})
```

### 🔑 Transfer Ownership

Hand off an Ownership boundary to another account, or carve out a brand-new
boundary on a node that currently inherits ownership:

```
transfer_ownership(node_id, new_owner)   // Propose (caller must be the resolved owner)
accept_ownership(node_id)                // Accept (caller must be `new_owner`)
```

**Example**: A property manager delegates Floor-3 to a tenant:
```
transfer_ownership(floor_3_id, tenant_corp)
accept_ownership(floor_3_id)   // signed by tenant_corp
```

A "self-transfer" (`new_owner == caller`) carves out a new, independent
boundary without changing the effective owner.

### 🗑️ Delete Node

Remove a leaf node (must have no children):

```
delete_node(node_id)
```

**Safety**: Cannot delete nodes with children to prevent orphaned subtrees.
Deleting a node also clears any `Ownership` / pending transfer attached to it.

## Storage Efficiency

### Compact Encoding

Node IDs use SCALE compact encoding for efficient storage:

| Node ID Value | Standard Size | Compact Size | Savings |
|---------------|---------------|--------------|---------|
| 0-63          | 8 bytes       | 1 byte       | 87%     |
| 64-16,383     | 8 bytes       | 2 bytes      | 75%     |
| 16,384+       | 8 bytes       | 3+ bytes     | 62%+    |

### Per-Field Storage

Each node's attributes live in their own storage map (`Parent`, `Meta`,
`Payload`), so a node with no metadata or payload set costs no storage for
those fields.

## Configuration

Customize the pallet for your use case:

| Constant | Default | Description | Example Use Case |
|-----------|---------|-------------|------------------|
| `MAX_DATA_SIZE` | 2048 bytes | Size limit for meta/payload | Sensor readings, configs |
| `MAX_TREE_DEPTH` | 32 levels | Maximum hierarchy depth | Nested organizations |
| `MAX_CHILDREN_PER_NODE` | 100 | Maximum child nodes | Factory with 50 machines |
| `MAX_ROOT_NODES` | 100 | Maximum top-level systems | Multi-site deployments |

## 🔐 Client-Side Encryption

### Overview

**The CPS pallet stores data as plain bytes**. For sensitive data, encryption must be handled at the **client level** before submitting to the blockchain. This design keeps the pallet simple and flexible, allowing clients to choose their preferred encryption schemes.

### Why Client-Side?

- **Flexibility**: Choose any encryption algorithm suitable for your use case
- **Simplicity**: Pallet remains lean without complex encryption logic
- **Upgradability**: Switch encryption schemes without pallet upgrades
- **Privacy Control**: Encryption keys never touch the blockchain

### Recommended Encryption: AEAD

For robust security, we recommend **AEAD (Authenticated Encryption with Associated Data)** ciphers:

✅ **Confidentiality** - Data is encrypted, unreadable without the key
✅ **Integrity** - Tampering is detected via authentication tag
✅ **Authentication** - Sender identity verified via ECDH key agreement

### Recommended Algorithms

| Algorithm | Nonce Size | Best For | Performance |
|-----------|------------|----------|-------------|
| **XChaCha20-Poly1305** (recommended) | 24 bytes | General purpose, large nonce space | ~680 MB/s (software) |
| **AES-256-GCM** | 12 bytes | Hardware acceleration | ~2-3 GB/s (with AES-NI) |
| **ChaCha20-Poly1305** | 12 bytes | Portable without hardware | ~600 MB/s (software) |

All algorithms should use:
- **256-bit keys** (derived via ECDH + HKDF-SHA256)
- **Authenticated encryption** (AEAD with authentication tag)
- **Sender verification** (optional during decryption)

### Self-Describing Encryption Format

We recommend storing encrypted data as **self-describing JSON** for forward compatibility:

```json
{
  "version": 1,
  "algorithm": "xchacha20",           // Auto-detected during decryption
  "from": "5GrwvaEF5zXb26Fz...",      // Sender's public key (bs58)
  "nonce": "Zm9vYmFy...",              // Random nonce (base64)
  "ciphertext": "ZW5jcnlwdGVk..."    // Encrypted data + auth tag (base64)
}
```

**Benefits**:
- Algorithm auto-detection during decryption
- Forward compatibility with new ciphers
- No version conflicts

### Client-Side Encryption Flow

```rust
// ===== ENCRYPTION (Before submitting to chain) =====

// 1. Key Agreement (ECDH)
let shared_secret = ecdh(sender_private, receiver_public);

// 2. Key Derivation (HKDF-SHA256)
let encryption_key = hkdf(shared_secret, "robonomics-cps-xchacha20");

// 3. AEAD Encryption
let nonce = random_bytes(24);  // XChaCha20 uses 24-byte nonce
let ciphertext = aead_encrypt(plaintext, encryption_key, nonce);

// 4. Build Self-Describing Message
let message = Message {
  version: 1,
  algorithm: "xchacha20",
  from: sender_public_key_bs58,
  nonce: base64(nonce),
  ciphertext: base64(ciphertext)
};

// 5. Serialize and Store
let encrypted_bytes = serde_json::to_vec(&message)?;
let data = BoundedVec::try_from(encrypted_bytes)?;
Cps::create_node(origin, parent_id, Some(data), None)?;

// ===== DECRYPTION (After retrieving from chain) =====

// 1. Retrieve node data
let meta = Cps::meta_of(node_id).ok_or(Error::NotFound)?;

// 2. Deserialize message
let message: Message = serde_json::from_slice(&meta)?;

// 3. Derive decryption key (same as encryption)
let shared_secret = ecdh(receiver_private, message.from);
let decryption_key = hkdf(shared_secret, format!("robonomics-cps-{}", message.algorithm));

// 4. Decrypt
let plaintext = aead_decrypt(
    base64::decode(message.ciphertext)?,
    decryption_key,
    base64::decode(message.nonce)?
)?;
```

### Example: IoT Sensor with Encrypted Telemetry

```rust
// Sensor encrypts reading before sending
let reading = b"temperature: 22.5C";
let encrypted = client.encrypt(reading, &receiver_public_key)?;
let data = BoundedVec::try_from(encrypted)?;

// Submit to chain
api.tx.cps.setPayload(sensorNodeId, data).signAndSend(sensorAccount);

// Server retrieves and decrypts
const payload = await api.query.cps.payload(sensorNodeId);
let decrypted = client.decrypt(&payload, &server_private_key)?;
println!("Reading: {}", String::from_utf8(decrypted)?);
```

### Security Considerations

**What's Protected (with client-side encryption)**:
- ✅ Data confidentiality (unreadable without keys)
- ✅ Data integrity (tampering detected)
- ✅ Sender authentication (via ECDH)

**What's NOT Protected**:
- ⚠️ Tree structure (always public)
- ⚠️ Encrypted data size (visible on-chain)
- ⚠️ Update frequency (transaction timestamps public)

### Key Management Best Practices

1. **Never store private keys on-chain**
2. **Use hardware wallets** for high-value keys
3. **Rotate keys periodically** for long-term deployments
4. **Use separate keys** for different security domains
5. **Implement key backup** and recovery procedures

### Encryption Libraries

- **Rust**: [chacha20poly1305](https://docs.rs/chacha20poly1305), [aes-gcm](https://docs.rs/aes-gcm)
- **JavaScript**: [libsodium-wrappers](https://github.com/jedisct1/libsodium.js), [tweetnacl](https://github.com/dchest/tweetnacl-js)
- **Python**: [cryptography](https://cryptography.io/), [pynacl](https://pynacl.readthedocs.io/)

## Security & Trust

### What's Protected

✅ **Ownership Verification**: Only the resolved owner can modify a node
✅ **Boundary Isolation**: Nested Ownership boundaries stop implicit ancestor rights
✅ **Tree Integrity**: `parent` is immutable, so cycles and rewiring are impossible
✅ **Data Encryption**: Client-side encryption fully supported for private data
✅ **Immutable History**: All changes recorded in blockchain events
✅ **DoS Protection**: Bounded collections prevent resource exhaustion

### What's NOT Protected

⚠️ **Encryption Key Management**: Users must manage encryption keys externally
⚠️ **Node Structure Privacy**: Tree topology is publicly visible
⚠️ **Access Control Beyond Ownership**: Only owner-based permissions supported

### Threat Model

**Prevents:**
- Unauthorized modification of nodes
- Tree corruption via cycles (impossible - `parent` is immutable)
- Resource exhaustion attacks
- Cross-boundary privilege escalation

**Does Not Prevent:**
- Analysis of tree structure
- Brute-force attacks on weak encryption keys
- Side-channel attacks on encrypted data size

## Integration Guide

### For Runtime Developers

1. Add to `Cargo.toml`:
   ```toml
   pallet-robonomics-cps = { default-features = false, path = "../frame/cps" }
   ```

2. Configure in runtime:
   ```rust
   impl pallet_robonomics_cps::Config for Runtime {
       type RuntimeEvent = RuntimeEvent;
       type OnPayloadSet = ();
       type WeightInfo = ();
   }
   ```

3. Add to `construct_runtime!`:
   ```rust
   Cps: pallet_robonomics_cps,
   ```

### For dApp Developers

Query the chain to discover system hierarchies:

```javascript
// Get a node's parent (also tells you whether the node exists)
const parent = await api.query.cps.parent(nodeId);

// Get metadata / payload
const meta = await api.query.cps.meta(nodeId);
const payload = await api.query.cps.payload(nodeId);

// Get all children of a node
const children = await api.query.cps.nodesByParent(parentId);

// Get all root nodes
const roots = await api.query.cps.rootNodes();
```

Create and manage hierarchies:

```javascript
// Create a root node
await api.tx.cps.createNode(null, metadata, payload).signAndSend(account);

// Add a child
await api.tx.cps.createNode(parentId, metadata, payload).signAndSend(account);

// Establish a new Ownership boundary on an existing node
await api.tx.cps.transferOwnership(nodeId, newOwner).signAndSend(currentOwner);
await api.tx.cps.acceptOwnership(nodeId).signAndSend(newOwner);
```

## Comparison with Alternatives

| Approach | Pros | Cons | Best For |
|----------|------|------|----------|
| **CPS Pallet** | Decentralized, immutable, hierarchical ownership | Requires blockchain | Trustless multi-party systems |
| **Traditional DB** | Fast, flexible queries | Centralized, mutable | Single organization |
| **IPFS + DB** | Decentralized storage | No ownership enforcement | Content distribution |
| **ERC-721 NFTs** | Standard, composable | Gas-expensive, limited structure | Digital collectibles |

## Technical Documentation

For detailed implementation information, see the [inline code documentation](src/lib.rs) which includes:
- Type definitions and trait implementations
- Storage layout and indexes
- Extrinsic signatures and validation logic
- Comprehensive test suite
- Benchmarking results

## License

Apache License 2.0 - See [LICENSE](../../LICENSE) for details.
