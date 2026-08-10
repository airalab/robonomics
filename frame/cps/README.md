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
Smart Factory (Root)
├── Production Line A
│   ├── Robot Arm 1
│   │   ├── Gripper
│   │   └── Vision Sensor
│   └── Conveyor Belt
└── Production Line B
    ├── Robot Arm 2
    └── Quality Control Station
```

This pallet provides a decentralized, tamper-proof registry for such systems, enabling:
- **Transparent ownership** of physical assets
- **Verifiable system topology** for audits and compliance
- **Secure data storage** with encryption support
- **Immutable audit trails** of system changes

## Core Concepts

### Hierarchical Tree Structure

Nodes are organized in a parent-child tree where each child inherits its parent's owner:

```
         [Factory]
         /        \
    [Line A]    [Line B]
      /   \        |
  [Robot] [Belt] [Robot]
```

**Benefits:**
- **Access Control**: Owning a parent grants control over its entire subtree
- **Logical Grouping**: Related systems stay together
- **Efficient Queries**: Find all components of a system in O(1) time

### Data Privacy Model

Each node can store two types of data:

1. **Metadata**: System configuration, capabilities, specifications
2. **Payload**: Operational data, sensor readings, telemetry

Both can be stored as:
- **Plain text**: Public information visible to all
- **Encrypted**: Private data readable only by authorized parties

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

### Use Case 2: Smart City Infrastructure

A city manages its IoT infrastructure:

```
City Dashboard
├── District North
│   ├── Traffic Light Controller #1
│   │   └── Status (plain) + Maintenance Log (encrypted)
│   └── Parking Sensor Grid
│       └── Occupancy Data (plain)
└── District South
    └── ...
```

**Benefits**: Decentralized control, verifiable maintenance records, public data transparency

### Use Case 3: Medical Device Network

A hospital organizes connected medical equipment:

```
Operating Room 3
├── Anesthesia Machine
│   └── Patient Data (encrypted) + Calibration (plain)
├── Vital Signs Monitor
│   └── Real-time Readings (encrypted)
└── Surgical Robot
    └── Procedure Log (encrypted)
```

**Benefits**: HIPAA-compliant encryption, immutable audit trail, emergency access control

## How It Works

### Creating a System Hierarchy

1. **Start with a root node** representing your top-level system
2. **Add child nodes** for subsystems and components
3. **Store data** as plain text (public) or encrypted (private)
4. **Reorganize** by moving nodes to different parents as systems evolve

```
Step 1: Create Root          Step 2: Add Children        Step 3: Add Details
    [Factory]         →           [Factory]          →        [Factory]
                                  /         \                 /         \
                            [Line A]    [Line B]        [Line A]    [Line B]
                                                         /    \
                                                    [Robot] [Belt]
```

### Tree Integrity Guarantees

The pallet enforces several invariants:

- **No Cycles**: Cannot move a parent under its own descendant
- **Ownership Consistency**: Children always have the same owner as their parent
- **Depth Limits**: Trees cannot exceed configured maximum depth
- **Deletion Safety**: Nodes with children cannot be deleted

**Visual Example of Cycle Prevention:**

```
BEFORE MOVE:          ATTEMPTED MOVE:         RESULT:
    [A]                   [A]                 ❌ ERROR
     |                     ↑                  CycleDetected
    [B]          →        [B]
     |                     ↑
    [C]                   [C]
```

### Performance: O(1) Operations

Traditional tree implementations require recursive traversal. This pallet stores the complete ancestor path in each node:

```
Node C stores: parent=[B], path=[A, B]
```

**Operations become instant:**
- ✅ **Cycle check**: `is node_id in target.path?` → O(1)
- ✅ **Depth check**: `target.path.len() < MAX_DEPTH?` → O(1)
- ✅ **Find ancestors**: Already stored in `path` field → O(1)

**Trade-off**: Slightly more storage per node, but predictable gas costs regardless of tree depth.

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

### 🔀 Move Node

Reorganize your hierarchy by moving nodes to different parents:

```
move_node(node_id, new_parent_id)
```

**Example**: Relocating a robot from Line A to Line B:
```
BEFORE:                      AFTER:
Factory                      Factory
├── Line A                  ├── Line A
│   └── Robot #5    →       └── Line B
└── Line B                      └── Robot #5
```

All descendants move with the node automatically!

### 🗑️ Delete Node

Remove a leaf node (must have no children):

```
delete_node(node_id)
```

**Safety**: Cannot delete nodes with children to prevent orphaned subtrees.

## Callbacks

### OnPayloadSet Trait

The CPS pallet provides a comprehensive callback system through the `OnPayloadSet` trait, enabling runtime-level hooks when node payloads are updated. This allows you to extend the pallet's functionality without modifying its core logic.

**When Callbacks Trigger:**
- After a payload is successfully set via `set_payload()` extrinsic
- Only after the storage write has completed
- Before the transaction finalizes

### Trait Definition

```rust
pub trait OnPayloadSet<AccountId> {
    fn on_payload_set(
        node_id: NodeId,
        meta: Option<NodeData>,
        payload: Option<NodeData>,
    );
}
```

### Implementation Pattern

Create a handler struct and implement the trait:

```rust
use pallet_robonomics_cps::{OnPayloadSet, NodeId, NodeData};

pub struct PayloadIndexer;

impl<AccountId, EncryptedData> OnPayloadSet<AccountId> 
    for PayloadIndexer 
where
    EncryptedData: MaxEncodedLen,
{
    fn on_payload_set(
        node_id: NodeId,
        meta: Option<NodeData>,
        payload: Option<NodeData>
    ) {
        // Your custom logic here
        log::info!("Payload updated on node {:?}", node_id);
        
        // Example: Trigger an event, update an index, etc.
        // Self::update_search_index(node_id, &payload);
    }
}
```

### Runtime Configuration

Configure the callback in your runtime's `Config` implementation:

```rust
impl pallet_robonomics_cps::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxTreeDepth = ConstU32<32>;
    type MaxChildrenPerNode = ConstU32<100>;
    type MaxRootNodes = ConstU32<100>;
    
    // Single handler
    type OnPayloadSet = PayloadIndexer;
    
    // Or disable callbacks with ()
    // type OnPayloadSet = ();
    
    type WeightInfo = ();
}
```

### Multiple Handlers

Combine multiple callback handlers using tuples:

```rust
// Define multiple handlers
pub struct PayloadLogger;
impl<AccountId, EncryptedData: MaxEncodedLen> OnPayloadSet<AccountId> 
    for PayloadLogger 
{
    fn on_payload_set(node_id: NodeId, meta: Option<_>, payload: Option<_>) {
        log::info!("Node {} payload changed", node_id);
    }
}

pub struct MetricsCollector;
impl<AccountId, EncryptedData: MaxEncodedLen> OnPayloadSet<AccountId> 
    for MetricsCollector 
{
    fn on_payload_set(node_id: NodeId, meta: Option<_>, payload: Option<_>) {
        // Update metrics
        // Example: increment counter in your metrics storage
        log::info!("Metrics: Node {:?} updated", node_id);
    }
}

// Configure multiple handlers in runtime
impl pallet_robonomics_cps::Config for Runtime {
    // ... other config ...
    type OnPayloadSet = (PayloadLogger, MetricsCollector);
}
```

### Use Cases

#### 1. Indexing and Search

Build searchable indexes of node payloads for efficient querying:

```rust
impl PayloadIndexer {
    fn update_search_index(node_id: NodeId, payload: &Option<NodeData<_>>) {
        if let Some(NodeData::Plain(data)) = payload {
            // Extract searchable terms and update your search index
            // Example: parse data and store in an off-chain storage or database
        }
    }
}
```

#### 2. External System Notifications

Push updates to off-chain systems or other chains:

```rust
pub struct WebhookNotifier;

impl<AccountId, EncryptedData: MaxEncodedLen> OnPayloadSet<AccountId> 
    for WebhookNotifier 
{
    fn on_payload_set(node_id: NodeId, _meta: Option<_>, payload: Option<_>) {
        // Queue notification to off-chain worker
        // Example: send event to external system via off-chain worker
        log::info!("Payload changed on node {:?}, notify external systems", node_id);
    }
}
```

#### 3. Analytics and Metrics

Track payload update patterns and system usage:

```rust
pub struct AnalyticsCollector;

impl<AccountId, EncryptedData: MaxEncodedLen> OnPayloadSet<AccountId> 
    for AnalyticsCollector 
{
    fn on_payload_set(node_id: NodeId, _meta: Option<_>, payload: Option<_>) {
        // Update metrics storage
        // Example: track update frequency, payload sizes, etc.
        log::info!("Analytics: Node {:?} updated, payload present: {}", 
                   node_id, payload.is_some());
    }
}
```

#### 4. Automated Actions

Trigger automated responses based on payload changes:

```rust
pub struct AutomationTrigger;

impl<AccountId, EncryptedData: MaxEncodedLen> OnPayloadSet<AccountId> 
    for AutomationTrigger 
{
    fn on_payload_set(node_id: NodeId, _meta: Option<_>, payload: Option<_>) {
        if let Some(NodeData::Plain(data)) = payload {
            // Example: Parse sensor reading and trigger alerts
            // In practice, implement parse_sensor_data and trigger_alert for your use case
            log::info!("Checking node {:?} payload for alert conditions", node_id);
        }
    }
}
```

#### 5. Audit Trail Maintenance

Maintain comprehensive logs of all payload changes:

```rust
pub struct AuditLogger;

impl<AccountId, EncryptedData: MaxEncodedLen> OnPayloadSet<AccountId> 
    for AuditLogger 
{
    fn on_payload_set(node_id: NodeId, meta: Option<_>, payload: Option<_>) {
        // Append to audit log storage
        // Example: record the change in a separate storage item or event
        log::info!("Audit: Node {:?} payload updated", node_id);
    }
}
```

### Performance Considerations

- **Keep it Fast**: Callbacks execute in the transaction context and affect gas costs
- **Avoid Heavy Computation**: Defer expensive operations to off-chain workers
- **No Panics**: Ensure your callback never panics, as it would fail the entire transaction
- **Weight Accounting**: Complex callbacks may require custom weight calculations

### Best Practices

✅ **Do:**
- Use callbacks for lightweight hooks and event triggers
- Queue heavy work for off-chain workers
- Handle errors gracefully without panicking
- Document callback behavior for runtime integrators

❌ **Don't:**
- Perform expensive computations in callbacks
- Make external network calls
- Modify storage extensively (affects weights)
- Assume callback execution order with multiple handlers

## Access Control

### Proxy-Based Delegation

The CPS pallet integrates seamlessly with Substrate's `pallet-proxy` to enable delegated access control. Node owners can grant specific accounts proxy permissions to perform operations on their behalf, without transferring ownership or revealing private keys.

**Key Benefits:**
- 🔐 **Restricted Permissions**: Grant only CPS operations, not full account access
- 🎯 **Node-Level Granularity**: Limit access to specific nodes and their descendants
- ⏰ **Time-Delayed Security**: Add delay periods for security-critical operations
- 🔄 **Revocable**: Owners can revoke proxy access at any time
- 📝 **Auditable**: All proxy actions are recorded in blockchain events

### Setting Up ProxyType

Define a `ProxyType` enum in your runtime that implements `InstanceFilter`:

```rust
use frame_support::traits::InstanceFilter;
use parity_scale_codec::{Decode, Encode};

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProxyType {
    Any,  // Allows all operations
    
    /// CPS write access with optional node restriction
    /// - `CpsWrite(None)`: Access to all CPS nodes owned by the proxied account
    /// - `CpsWrite(Some(node_id))`: Access only to specific node and its descendants
    CpsWrite(Option<NodeId>),
}

impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::CpsWrite(allowed_node) => {
                // Check if it's a CPS call
                let is_cps_call = matches!(
                    c,
                    RuntimeCall::Cps(pallet_robonomics_cps::Call::set_meta { .. })
                        | RuntimeCall::Cps(pallet_robonomics_cps::Call::set_payload { .. })
                        | RuntimeCall::Cps(pallet_robonomics_cps::Call::move_node { .. })
                        | RuntimeCall::Cps(pallet_robonomics_cps::Call::delete_node { .. })
                        | RuntimeCall::Cps(pallet_robonomics_cps::Call::create_node { .. })
                );
                
                if !is_cps_call {
                    return false;
                }
                
                // If no specific node restriction, allow all CPS calls
                if allowed_node.is_none() {
                    return true;
                }
                
                // Check if call targets the allowed node
                match c {
                    RuntimeCall::Cps(pallet_robonomics_cps::Call::set_meta { node_id, .. }) |
                    RuntimeCall::Cps(pallet_robonomics_cps::Call::set_payload { node_id, .. }) |
                    RuntimeCall::Cps(pallet_robonomics_cps::Call::move_node { node_id, .. }) |
                    RuntimeCall::Cps(pallet_robonomics_cps::Call::delete_node { node_id, .. }) => {
                        Some(node_id) == allowed_node.as_ref()
                    }
                    RuntimeCall::Cps(pallet_robonomics_cps::Call::create_node { parent_id, .. }) => {
                        parent_id.as_ref() == allowed_node.as_ref()
                    }
                    _ => false,
                }
            }
        }
    }
    
    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (ProxyType::Any, _) => true,
            (_, ProxyType::Any) => false,
            (ProxyType::CpsWrite(None), ProxyType::CpsWrite(_)) => true,
            (ProxyType::CpsWrite(Some(a)), ProxyType::CpsWrite(Some(b))) => a == b,
            _ => false,
        }
    }
}
```

### Complete Example: IoT Sensor Management

**Scenario**: Alice owns a network of temperature sensors represented as CPS nodes. She wants to allow her IoT gateway device to update sensor readings without giving it full account access.

**Note**: This example uses simplified syntax for clarity. In production, adapt the types and error handling to match your runtime configuration.

```rust
// Step 1: Alice (owner) creates the sensor node hierarchy
let alice = AccountId::from([1u8; 32]);
let gateway = AccountId::from([2u8; 32]);

// Create root node for sensor network
Cps::create_node(
    RuntimeOrigin::signed(alice.clone()),
    None,  // root node
    Some(NodeData::Plain(b"Building_A_Sensors".to_vec().try_into()?)),
    None,
)?;
let network_id = NodeId(0);

// Create individual sensor nodes
Cps::create_node(
    RuntimeOrigin::signed(alice.clone()),
    Some(network_id),
    Some(NodeData::Plain(b"Room_101_Temperature".to_vec().try_into()?)),
    Some(NodeData::Plain(b"22.5C".to_vec().try_into()?)),
)?;
let sensor_id = NodeId(1);

// Step 2: Alice grants the gateway proxy access for CPS operations only
Proxy::add_proxy(
    RuntimeOrigin::signed(alice.clone()),
    gateway.clone(),
    ProxyType::CpsWrite(None),  // Restricts gateway to CPS operations only
    0  // No delay - proxy is immediately active
)?;

// Step 3: Gateway updates sensor reading on Alice's behalf
let new_reading = NodeData::Plain(b"23.1C".to_vec().try_into()?);
Proxy::proxy(
    RuntimeOrigin::signed(gateway.clone()),
    alice.clone(),
    None,
    Box::new(RuntimeCall::Cps(Call::set_payload {
        node_id: sensor_id,
        payload: Some(new_reading),
    }))
)?;

// Step 4: Alice can verify the update
// (In practice, query via RPC or check events)
let node = Cps::nodes(sensor_id).unwrap();
assert_eq!(node.owner, alice);  // Ownership unchanged

// Step 5: When gateway is decommissioned, Alice revokes access
Proxy::remove_proxy(
    RuntimeOrigin::signed(alice),
    gateway,
    ProxyType::CpsWrite(None),
    0
)?;
```

### Usage Patterns

#### 1. Time-Delayed Proxy for Security

Add a delay period for security-critical operations, giving the owner time to review and potentially cancel:

```rust
// Grant proxy access with 100-block delay
Proxy::add_proxy(
    RuntimeOrigin::signed(owner),
    proxy_account,
    ProxyType::CpsWrite(None),
    100  // Proxy activates after 100 blocks
)?;

// Owner has 100 blocks to review and potentially cancel before it activates
// This prevents immediate malicious actions by compromised proxy accounts
```

#### 2. Multi-Signature Workflows

Distribute node management across team members for collaborative operations:

```rust
// Team lead grants proxy access to multiple team members
Proxy::add_proxy(
    RuntimeOrigin::signed(team_lead),
    engineer_alice,
    ProxyType::CpsWrite(None),
    0
)?;

Proxy::add_proxy(
    RuntimeOrigin::signed(team_lead),
    engineer_bob,
    ProxyType::CpsWrite(None),
    0
)?;

// Engineer Alice reorganizes node hierarchy for her department
Proxy::proxy(
    RuntimeOrigin::signed(engineer_alice),
    team_lead,
    None,
    Box::new(RuntimeCall::Cps(Call::move_node {
        node_id: NodeId(5),
        new_parent_id: NodeId(3),
    }))
)?;
```

#### 3. Node-Specific Restrictions

Grant proxy access to only a specific node and its descendants:

```rust
// Grant proxy access to only node 5 and its children
// Useful for delegating management of a specific subtree
Proxy::add_proxy(
    RuntimeOrigin::signed(owner),
    contractor_account,
    ProxyType::CpsWrite(Some(NodeId(5))),  // Only node 5
    0
)?;

// Contractor can update node 5
Proxy::proxy(
    RuntimeOrigin::signed(contractor_account),
    owner,
    None,
    Box::new(RuntimeCall::Cps(Call::set_payload {
        node_id: NodeId(5),
        payload: Some(NodeData::Plain(b"updated".to_vec().try_into()?)),
    }))
)?;

// Contractor can create children under node 5
Proxy::proxy(
    RuntimeOrigin::signed(contractor_account),
    owner,
    None,
    Box::new(RuntimeCall::Cps(Call::create_node {
        parent_id: Some(NodeId(5)),
        meta: Some(NodeData::Plain(b"child_node".to_vec().try_into()?)),
        payload: None,
    }))
)?;

// But contractor CANNOT update other nodes (e.g., node 3)
// This call would fail with NotProxy error
```

#### 4. Automated Bot Access

Allow automation bots to update node state while restricting them from other account operations:

```rust
// Automation bot updates node data based on external events
// ProxyType::CpsWrite(None) ensures it can only manage CPS nodes
Proxy::proxy(
    RuntimeOrigin::signed(monitoring_bot),
    system_owner,
    None,
    Box::new(RuntimeCall::Cps(Call::set_payload {
        node_id: NodeId(10),
        payload: Some(NodeData::Plain(b"alert: threshold exceeded".to_vec().try_into()?)),
    }))
)?;

// The bot CANNOT:
// - Transfer funds from the owner's account
// - Change account settings
// - Execute non-CPS operations
```

### Security Considerations

**Type Safety:**
- `ProxyType::CpsWrite` restricts proxies to CPS operations only
- Proxies cannot execute balance transfers, governance votes, or other operations
- Type system enforces these restrictions at compile time

**Node-Level Granularity:**
- `CpsWrite(Some(node_id))` enables fine-grained access control
- Limits proxy to a specific subtree of the node hierarchy
- Useful for contractor or temporary access scenarios

**Ownership Preserved:**
- All operations maintain original ownership semantics
- Nodes remain owned by the original account
- Proxies act on behalf of the owner, not as the owner

**Revocable:**
- Owners can revoke proxy access at any time
- Immediate effect - no delay required for revocation
- Multiple proxies can be managed independently

**Auditable:**
- All proxy actions are recorded in blockchain events
- Full transparency of who did what on whose behalf
- Essential for compliance and security audits

**No Privilege Escalation:**
- Proxies cannot grant permissions to other accounts
- Cannot create new proxies on behalf of the owner
- Strictly limited to configured operations

### Best Practices

✅ **Do:**
- Use `CpsWrite(None)` for trusted automation systems needing broad access
- Use `CpsWrite(Some(node_id))` for contractors or limited-scope access
- Add time delays for high-value or security-critical operations
- Regularly audit active proxies and revoke unused ones
- Document proxy relationships for team coordination

❌ **Don't:**
- Grant `ProxyType::Any` unless absolutely necessary
- Leave temporary proxies active after their purpose is fulfilled
- Use proxies as a substitute for proper multi-sig governance
- Share proxy account keys - create separate proxies per entity

### Use Cases Summary

1. **IoT Device Management**: Grant IoT gateways write access to update sensor data without exposing account keys
2. **Multi-Signature Workflows**: Distribute node management responsibilities across team members
3. **Automated Systems**: Allow bots to update node state based on external triggers with limited permissions
4. **Temporary Access**: Grant time-limited access for maintenance, audits, or contractor work
5. **Hierarchical Management**: Delegate specific subtree management to department leads or sub-teams

## Storage Efficiency

### Compact Encoding

Node IDs use SCALE compact encoding for efficient storage:

| Node ID Value | Standard Size | Compact Size | Savings |
|---------------|---------------|--------------|---------|
| 0-63          | 8 bytes       | 1 byte       | 87%     |
| 64-16,383     | 8 bytes       | 2 bytes      | 75%     |
| 16,384+       | 8 bytes       | 3+ bytes     | 62%+    |

**Real-world impact:**
- Path with 5 small IDs: **5 bytes** vs 40 bytes (87% reduction)
- Typical tree depth of 3-4 levels benefits significantly
- No performance penalty—still O(1) operations

### Visual Example

```
Standard encoding [0, 1, 2]:     |████████|████████|████████|  (24 bytes)
Compact encoding [0, 1, 2]:      |█|█|█|                        (3 bytes)
```

## Configuration

Customize the pallet for your use case:

| Parameter | Default | Description | Example Use Case |
|-----------|---------|-------------|------------------|
| `MaxDataSize` | 2048 bytes | Size limit for meta/payload | Sensor readings, configs |
| `MaxTreeDepth` | 32 levels | Maximum hierarchy depth | Nested organizations |
| `MaxChildrenPerNode` | 100 | Maximum child nodes | Factory with 50 machines |
| `MaxRootNodes` | 100 | Maximum top-level systems | Multi-site deployments |

**Tuning Guidelines:**
- **Small IoT deployments**: Keep defaults
- **Large industrial systems**: Increase MaxChildrenPerNode to 1000+
- **Shallow hierarchies**: Reduce MaxTreeDepth to 10-15
- **Enterprise multi-site**: Increase MaxRootNodes to 1000+

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
let node = Cps::nodes(node_id)?;
let encrypted_bytes = node.meta?;

// 2. Deserialize message
let message: Message = serde_json::from_slice(&encrypted_bytes)?;

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
api.tx.cps.set_payload(sensor_node_id, Some(data)).sign_and_send(sensor_account)?;

// Server retrieves and decrypts
let node = api.query.cps.nodes(sensor_node_id).await?;
let decrypted = client.decrypt(&node.payload?, &server_private_key)?;
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

✅ **Ownership Verification**: Only owners can modify their nodes  
✅ **Tree Integrity**: Impossible to create cycles or orphaned nodes  
✅ **Data Encryption**: Private data protected with XChaCha20-Poly1305  
✅ **Immutable History**: All changes recorded in blockchain events  
✅ **DoS Protection**: Bounded collections prevent resource exhaustion  

### What's NOT Protected

⚠️ **Encryption Key Management**: Users must manage encryption keys externally  
⚠️ **Node Structure Privacy**: Tree topology is publicly visible  
⚠️ **Access Control Beyond Ownership**: Only owner-based permissions supported  

### Threat Model

**Prevents:**
- Unauthorized modification of nodes
- Tree corruption via cycles
- Resource exhaustion attacks
- Replay attacks (via nonces)

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
       type MaxTreeDepth = ConstU32<32>;
       type MaxChildrenPerNode = ConstU32<100>;
       type MaxRootNodes = ConstU32<100>;
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
// Get a node
const node = await api.query.cps.nodes(nodeId);

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

// Move a node
await api.tx.cps.moveNode(nodeId, newParentId).signAndSend(account);
```

## Comparison with Alternatives

| Approach | Pros | Cons | Best For |
|----------|------|------|----------|
| **CPS Pallet** | Decentralized, immutable, efficient | Requires blockchain | Trustless multi-party systems |
| **Traditional DB** | Fast, flexible queries | Centralized, mutable | Single organization |
| **IPFS + DB** | Decentralized storage | No ownership enforcement | Content distribution |
| **ERC-721 NFTs** | Standard, composable | Gas-expensive, limited structure | Digital collectibles |

## Roadmap

**Current (v1):**
- ✅ Hierarchical tree with cycle prevention
- ✅ Plain and encrypted data storage
- ✅ O(1) operations via path storage
- ✅ Compact encoding for efficiency

**Planned (v2):**
- 🔮 Multi-owner nodes with role-based permissions
- 🔮 Node templates for rapid deployment
- 🔮 Batch operations for bulk updates
- 🔮 Additional encryption algorithms (AES-GCM, ChaCha20)
- 🔮 Off-chain worker integration for automated maintenance

## Technical Documentation

For detailed implementation information, see the [inline code documentation](src/lib.rs) which includes:
- Type definitions and trait implementations
- Storage layout and indexes
- Extrinsic signatures and validation logic
- Comprehensive test suite
- Benchmarking results

## License

Apache License 2.0 - See [LICENSE](../../LICENSE) for details.

---

**Questions?** Check the [Robonomics Wiki](https://wiki.robonomics.network) or join our [Discord](https://discord.gg/robonomics).
