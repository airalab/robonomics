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

### Custom Encryption Algorithms

The pallet supports extensible encryption algorithms through a configurable associated type. You can define custom encryption algorithms by implementing your own enum:

```rust
use parity_scale_codec::{Encode, Decode};
use scale_info::TypeInfo;
use frame_support::pallet_prelude::MaxEncodedLen;

#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MyCryptoAlgorithm {
    XChaCha20Poly1305,
    AesGcm256,
    ChaCha20,
}

impl pallet_robonomics_cps::Config for Runtime {
    type CryptoAlgorithm = MyCryptoAlgorithm;
    // ... other config
}
```

**Default Implementation:**

The pallet provides `DefaultCryptoAlgorithm` with XChaCha20-Poly1305 support out of the box:

```rust
impl pallet_robonomics_cps::Config for Runtime {
    type CryptoAlgorithm = pallet_robonomics_cps::DefaultCryptoAlgorithm;
    // ... other config
}
```

**Benefits of Configurable Algorithms:**
- Add new algorithms without modifying pallet code
- Different runtimes can support different encryption schemes
- Easy testing with mock crypto types
- Future-proof for emerging cryptographic standards

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
       type CryptoAlgorithm = pallet_robonomics_cps::DefaultCryptoAlgorithm;
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
- ✅ Extensible encryption algorithms via Config trait

**Planned (v2):**
- 🔮 Multi-owner nodes with role-based permissions
- 🔮 Node templates for rapid deployment
- 🔮 Batch operations for bulk updates
- 🔮 Built-in implementations for additional encryption algorithms (AES-GCM, ChaCha20)
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
