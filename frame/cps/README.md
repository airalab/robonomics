# Pallet CPS

Cyber-Physical System hierarchical tree management pallet for Substrate-based blockchains.

## Overview

The CPS (Cyber-Physical System) pallet provides a hierarchical tree structure for organizing and managing cyber-physical systems on-chain. It allows users to:

- Create nodes in a tree structure with optional parent-child relationships
- Store metadata and payload data (plain or encrypted)
- Manage node ownership and access control
- Move nodes within the tree while maintaining integrity
- Create and use crypto profiles for encrypted data

## Key Features

- **Hierarchical Structure**: Organize nodes in a tree with configurable depth limits
- **Ownership Model**: Each node has an owner, children inherit parent's owner
- **Data Privacy**: Support for both plain and encrypted data with crypto profiles
- **Tree Integrity**: Cycle detection prevents invalid tree structures
- **Efficient Indexing**: Multiple indexes for fast queries by owner, parent, and root nodes

## Storage

### Primary Storage

- `NextNodeId`: Counter for generating unique node IDs
- `Nodes`: Main storage for node data

### Indexes

- `NodesByOwner`: Index nodes by their owner account
- `NodesByParent`: Index children by parent node
- `RootNodes`: List of all root nodes (nodes without parents)

## Extrinsics

### `create_node(parent_id, meta, payload)`

Create a new node in the tree.

- `parent_id`: Optional parent node ID (None for root nodes)
- `meta`: Optional metadata (plain or encrypted with XChaCha20-Poly1305)
- `payload`: Optional payload data (plain or encrypted with XChaCha20-Poly1305)

**Requirements:**
- If parent is specified, it must exist and caller must be its owner
- Tree depth limit must not be exceeded

### `set_meta(node_id, meta)`

Update node metadata.

- `node_id`: Target node ID
- `meta`: New metadata (None to clear)

**Requirements:**
- Node must exist
- Caller must be node owner

### `set_payload(node_id, payload)`

Update node payload.

- `node_id`: Target node ID
- `payload`: New payload (None to clear)

**Requirements:**
- Node must exist
- Caller must be node owner

### `move_node(node_id, new_parent_id)`

Move a node to a different parent.

- `node_id`: Node to move
- `new_parent_id`: New parent node ID

**Requirements:**
- Both nodes must exist
- Caller must own both nodes
- Cannot create cycles (moving ancestor under descendant)
- Tree depth limit must not be exceeded after move

## Events

- `NodeCreated(node_id, parent_id, owner)`: Node created
- `MetaSet(node_id, owner)`: Metadata updated
- `PayloadSet(node_id, owner)`: Payload updated
- `NodeMoved(node_id, old_parent, new_parent, owner)`: Node moved
- `NodeDeleted(node_id, owner)`: Node deleted

## Configuration

The pallet can be configured with the following constants:

- `MaxDataSize`: Maximum size for data fields (default: 2048 bytes)
- `MaxTreeDepth`: Maximum tree depth (default: 32 levels)
- `MaxChildrenPerNode`: Maximum children per node (default: 100)
- `MaxNodesPerOwner`: Maximum nodes per owner (default: 1000)
- `MaxRootNodes`: Maximum root nodes (default: 100)

## Usage Example

```rust
// Create a root node with plain metadata
let meta = Some(NodeData::Plain(vec![1, 2, 3].try_into().unwrap()));
Cps::create_node(origin, None, meta, None)?;

// Create a child node with encrypted payload using XChaCha20-Poly1305
let payload = Some(NodeData::Encrypted {
    algorithm: CryptoAlgorithm::XChaCha20Poly1305,
    ciphertext: vec![4, 5, 6].try_into().unwrap(),
});
Cps::create_node(origin, Some(0), None, payload)?;

// Create a node with both encrypted metadata and payload
let encrypted_meta = Some(NodeData::Encrypted {
    algorithm: CryptoAlgorithm::XChaCha20Poly1305,
    ciphertext: vec![10, 11, 12].try_into().unwrap(),
});
let encrypted_payload = Some(NodeData::Encrypted {
    algorithm: CryptoAlgorithm::XChaCha20Poly1305,
    ciphertext: vec![13, 14, 15].try_into().unwrap(),
});
Cps::create_node(origin, Some(0), encrypted_meta, encrypted_payload)?;

// Move the child to a different parent
Cps::move_node(origin, 1, 2)?;

// Update metadata
let new_meta = Some(NodeData::Plain(vec![7, 8, 9].try_into().unwrap()));
Cps::set_meta(origin, 1, new_meta)?;
```

## Privacy Model

The pallet supports two modes for data storage:

### Plain Data

Data stored directly on-chain, visible to everyone:

```rust
NodeData::Plain(BoundedVec::from(vec![1, 2, 3]))
```

### Encrypted Data

Data encrypted using XChaCha20-Poly1305 AEAD:

```rust
NodeData::Encrypted {
    algorithm: CryptoAlgorithm::XChaCha20Poly1305,
    ciphertext: BoundedVec::from(vec![...]),  // Encrypted data
}
```

The encryption algorithm is specified at the node level, allowing flexibility for future algorithm additions while maintaining backward compatibility.


## Security Considerations

- **Ownership**: Only node owners can modify their nodes
- **Cycle Prevention**: The `move_node` extrinsic includes cycle detection to maintain tree integrity
- **Bounded Collections**: All collections use bounded types to prevent DoS attacks
- **Tree Depth**: Configurable depth limit prevents stack overflow during traversal
- **Data Validation**: All data must fit within configured size limits

## License

Apache License 2.0
