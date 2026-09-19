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
- **Verifiable authority boundaries** for physical assets and sub-systems
- **Verifiable system topology** for audits and compliance
- **Secure data storage** with client-side encryption support
- **Immutable audit trails** of system changes
- **Fine-grained delegation** of node-state mutation without transferring ownership

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

### Scope / Access Model

Authority is **not** stored on every node. Instead, a `Scope` marks only the
nodes that start a new administrative and economic boundary; every other node
resolves to the nearest ancestor `Scope`:

```
Global
Scope #1 / owner=A
|
`-- Japan
    Scope #7 / owner=B
    |
    `-- University
```

`University` resolves to `Scope #7` (owner `B`); `Japan`'s Scope has no
implicit rights over `Global`'s other, independently owned children, and vice
versa. A nested Scope is always a hard boundary: it stops inheritance of
authority, `Access`, and resource limits, even when parent and child Scope
owners are the same account.

`Pallet::resolve_scope(node_id)` is the single canonical resolver: it walks
`parent` links one hop at a time until it finds an active Scope, and is used
by every authorization check in this pallet.

### Capabilities and Access

A [`Capability`] is a delegable authority. Two are defined today:

- **`Write`** - mutate a node's `Meta` / `Payload` (covers both `set_meta`
  and `set_payload`).
- **`CreateScope`** - create/replace a Scope at the exact Scope root that
  grants it (the sole mechanism for handing over control of a Scope).

`Pallet::grant_access` / `Pallet::revoke_access` let a Scope owner delegate a
`Capability` to another account at a specific `NodeId`, either for that exact
node (`inherited = false`) or for the node and all its descendants within the
same Scope (`inherited = true`). Access never crosses a nested Scope
boundary. The Scope owner always has implicit authority over their whole
Scope and does not need explicit `Access` entries.

`CreateScope` is special-cased: it is never inherited, even if granted with
`inherited = true` - it only ever applies to the exact node it targets.

#### Example: delegating `Write`

```
Factory
Scope #10 / owner=A
|
+-- Robot
|
`-- Laboratory
    Scope #20 / owner=B
    |
    `-- Sensor
```

If `A` grants `Access(#10, Factory, Gateway, Write, inherited = true)`,
`Gateway` may `set_meta`/`set_payload` on `Factory` and `Robot` (both resolve
to Scope #10), but **not** on `Laboratory` or `Sensor` - those resolve to the
independent `Scope #20`, so `A`'s Scope-#10 Access never applies there, even
though `Laboratory` is a descendant of `Factory` in the tree.

Access entries become invalid the moment their Scope is replaced or deleted,
without requiring any rewrite: authorization always starts by resolving the
*current* Scope for the target node.

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
managing their own equipment, and delegates day-to-day sensor updates to a
gateway device without handing over Scope ownership:

```
Building-A
Scope #1 / owner=PropertyManager
├── Floor-1 (shared building systems)
│   ├── Fire Suppression Controller
│   └── Elevator Bank
└── Floor-3
    Scope #7 / owner=TenantCorp
    ├── HVAC-Unit-07
    │   └── Thermostat-142 (occupancy data, encrypted)
    │       Access(#7, Thermostat-142, Gateway, Write, inherited=false)
    └── Access Control Panel (badge logs, encrypted)
```

**Benefits**: `TenantCorp` manages Floor-3's equipment independently and can
delegate `Write` on individual nodes (like `Thermostat-142`) to a `Gateway`
device, without granting it any administrative rights over the Scope;
`PropertyManager` retains full control of shared building systems and other
floors without either party having implicit access to the other's boundary.

## How It Works

### Creating a System Hierarchy

1. **Start with a root node** representing your top-level system - the creator becomes the owner of a freshly allocated Scope
2. **Add child nodes** for subsystems and components - requires `Write` authority (Scope owner, or matching `Access`) over the parent's resolved Scope
3. **Store data** as plain text (public) or client-side encrypted (private)
4. **Establish nested boundaries** with `create_scope` when a sub-tree needs independent administration
5. **Delegate `Write`** with `grant_access` when another account should update node state without administering the Scope

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
- **Scope Resolution**: Every active node resolves to exactly one Scope
- **Scope Boundaries**: An active `Scope` entry stops inheritance from ancestors
- **Depth Limits**: Trees cannot exceed `MaxTreeDepth`
- **Deletion Safety**: Nodes with children cannot be deleted

### Scope Resolution: O(depth)

There is no cached ancestor list. `resolve_scope` walks the single `parent`
link one hop at a time until it finds an active Scope, bounded by
`MaxTreeDepth`:

```
Node C: parent = Some(B)  ─┐
Node B: parent = Some(A)   ├─ walked one hop at a time
Node A: parent = None      ┘  (root - always has an active Scope entry)
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

Creating a root node (`parent: None`) allocates a fresh Scope, owned by the
caller. Creating a child node requires `Write` authority over the parent's
resolved Scope; the child does not get its own Scope.

### ✏️ Update Data

Modify metadata or payload without changing the hierarchy. Both require
`Write` authority over the node's resolved Scope:

```
set_meta(node_id, new_metadata)    // Update configuration
set_payload(node_id, new_payload)  // Update operational data
```

**Example**: Sensor recalibration:
```
set_meta(sensor_id, {"type": "temperature", "model": "DHT22", "calibrated": "2025-01-15"})
```

### 🔒 Create / Replace a Scope

Establish a new, independent administrative and economic boundary on a node,
or replace an existing Scope rooted at the caller's own node:

```
create_scope(node_id)
```

The Scope owner may do this on any node within their Scope. A non-owner may
only replace a Scope at its own root, and only via a non-inherited
`CreateScope` grant on that exact node.

**Example**: A property manager carves out an independent boundary for a new tenant:
```
create_scope(floor_3_id)   // signed by the current Scope owner
```

Replacing a Scope allocates a brand-new `ScopeId` - the previous Scope's
`Access` entries become immediately inactive without requiring any
descendant rewrite.

### 🗝️ Delete a Scope

Remove an administrative/economic boundary from a node without deleting the
node or its descendants (they fall back to resolving the nearest remaining
ancestor Scope):

```
delete_scope(node_id)
```

Only the Scope's owner may delete it (never through `Access`, even a full
`Write` grant). A CPS root's Scope can never be deleted, since every node
must resolve to exactly one Scope.

### 🔑 Grant / Revoke Access

Delegate (or withdraw) a `Capability` to another account at a specific node:

```
grant_access(node_id, principal, capability, inherited)
revoke_access(node_id, principal, capability)
```

Only the Scope owner may grant or revoke Access. `inherited = true`
propagates the grant to descendants that still resolve to the same Scope;
`inherited = false` applies only to the exact node.

**Example**: A tenant delegates `Write` on a single thermostat to a gateway device:
```
grant_access(thermostat_id, gateway_account, Capability::Write, inherited: false)
```

### 🗑️ Delete Node

Remove a leaf node (must have no children). Requires `Write` authority over
the node's resolved Scope:

```
delete_node(node_id)
```

**Safety**: Cannot delete nodes with children to prevent orphaned subtrees.
Deleting a node also clears any `Meta` / `Payload` / active Scope attached to it.

## Runtime API

`pallet-robonomics-cps-runtime-api` exposes read-only queries to off-chain
clients (e.g. Subxt-based tooling) without reimplementing Scope-resolution or
Access-traversal logic client-side:

- `resolve_scope(node) -> Option<ScopeId>` - the `ScopeId` currently active
  for `node`.
- `has_capability(node_id, account_id, capability_id) -> bool` - whether
  `account_id` currently holds `capability_id` at `node_id`, reusing the same
  authorization logic enforced by `set_meta`/`set_payload`/`create_scope`.

`capability_id` is a stable [`CapabilityId`], decoupled from `Capability`'s
internal SCALE representation, so the Runtime API's wire format does not
change as new capabilities are added. Convert with
`CapabilityId::from(capability)` / `Capability::try_from(capability_id)`.

## Storage Efficiency

### Compact Encoding

Node IDs use SCALE compact encoding for efficient storage:

| Node ID Value | Standard Size | Compact Size | Savings |
|---------------|---------------|--------------|---------|
| 0-63          | 8 bytes       | 1 byte       | 87%     |
| 64-16,383     | 8 bytes       | 2 bytes      | 75%     |
| 16,384+       | 8 bytes       | 3+ bytes     | 62%+    |

### Per-Field Storage

Each node's attributes live in their own storage map (`Parents`, `Meta`,
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

✅ **Authorization Verification**: Only the Scope owner or an explicit `Access` grant can mutate a node
✅ **Boundary Isolation**: Nested Scopes stop implicit ancestor rights and Access inheritance
✅ **Tree Integrity**: `parent` is immutable, so cycles and rewiring are impossible
✅ **Data Encryption**: Client-side encryption fully supported for private data
✅ **Immutable History**: All changes recorded in blockchain events
✅ **DoS Protection**: Bounded collections prevent resource exhaustion
✅ **Least Privilege Delegation**: `Write` delegates data mutation only - never Scope administration

### What's NOT Protected

⚠️ **Encryption Key Management**: Users must manage encryption keys externally
⚠️ **Node Structure Privacy**: Tree topology is publicly visible
⚠️ **Access Control Beyond Scope/Access**: Only Scope-owner and `Access`-based permissions supported

### Threat Model

**Prevents:**
- Unauthorized modification of nodes
- Tree corruption via cycles (impossible - `parent` is immutable)
- Resource exhaustion attacks
- Cross-boundary privilege escalation (nested Scopes are a hard boundary)
- Privilege escalation from data mutation to Scope administration (`Write` never authorizes `create_scope`/`delete_scope`/`grant_access`/`revoke_access`)

**Does Not Prevent:**
- Analysis of tree structure
- Brute-force attacks on weak encryption keys
- Side-channel attacks on encrypted data size

## Integration Guide

### For Runtime Developers

1. Add to `Cargo.toml`:
   ```toml
   pallet-robonomics-cps = { default-features = false, path = "../frame/cps" }
   pallet-robonomics-cps-runtime-api = { default-features = false, path = "../frame/cps-runtime-api" }
   ```

2. Configure in runtime:
   ```rust
   impl pallet_robonomics_cps::Config for Runtime {
       type RuntimeEvent = RuntimeEvent;
       type WeightInfo = ();
   }
   ```

3. Add to `construct_runtime!`:
   ```rust
   Cps: pallet_robonomics_cps,
   ```

4. Implement the Runtime API:
   ```rust
   impl pallet_robonomics_cps_runtime_api::CpsApi<Block, AccountId> for Runtime {
       fn resolve_scope(node: NodeId) -> Option<ScopeId> {
           Cps::resolve_scope(node).ok()
       }

       fn has_capability(node_id: NodeId, account_id: AccountId, capability_id: CapabilityId) -> bool {
           match Capability::try_from(capability_id) {
               Ok(capability) => Cps::has_capability(node_id, &account_id, capability),
               Err(()) => false,
           }
       }
   }
   ```

### For dApp Developers

Query the chain to discover system hierarchies:

```javascript
// Get a node's parent (also tells you whether the node exists)
const parent = await api.query.cps.parents(nodeId);

// Get metadata / payload
const meta = await api.query.cps.meta(nodeId);
const payload = await api.query.cps.payload(nodeId);

// Get all children of a node
const children = await api.query.cps.nodesByParent(parentId);

// Get all root nodes
const roots = await api.query.cps.rootNodes();

// Resolve the active Scope and check a capability via the Runtime API
const scopeId = await api.call.cpsApi.resolveScope(nodeId);
const canWrite = await api.call.cpsApi.hasCapability(nodeId, accountId, writeCapabilityId);
```

Create and manage hierarchies:

```javascript
// Create a root node
await api.tx.cps.createNode(null, metadata, payload).signAndSend(account);

// Add a child
await api.tx.cps.createNode(parentId, metadata, payload).signAndSend(account);

// Establish a new Scope boundary on an existing node
await api.tx.cps.createScope(nodeId).signAndSend(currentOwner);

// Delegate Write to another account for a single node
await api.tx.cps.grantAccess(nodeId, principal, 'Write', false).signAndSend(scopeOwner);
```

## Comparison with Alternatives

| Approach | Pros | Cons | Best For |
|----------|------|------|----------|
| **CPS Pallet** | Decentralized, immutable, hierarchical Scope/Access authority | Requires blockchain | Trustless multi-party systems |
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
