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
- **Data Privacy**: Support for both plain and encrypted data using XChaCha20-Poly1305 AEAD
- **Tree Integrity**: O(1) cycle detection via ancestor path tracking
- **Efficient Operations**: Path stored in each node eliminates recursive lookups
- **Efficient Indexing**: Multiple indexes for fast queries by parent and root nodes

## Storage

### Primary Storage

- `NextNodeId`: Counter for generating unique node IDs
- `Nodes`: Main storage for node data (uses Blake2_128Concat hasher for security)
  - Each node stores its complete ancestor path for O(1) cycle detection and depth checks

### Indexes

- `NodesByParent`: Index children by parent node (uses Blake2_128Concat hasher)
- `RootNodes`: List of all root nodes (nodes without parents)

**Note:** NodesByOwner index removed in favor of off-chain indexing for better scalability.

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

### `delete_node(node_id)`

Delete a node from the tree.

- `node_id`: Node to delete

**Requirements:**
- Node must exist
- Caller must be node owner
- Node must not have any children

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
- `MaxRootNodes`: Maximum root nodes (default: 100)

**Storage Version:** v1 - Initial implementation with Blake2_128Concat hashers

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

// Delete a leaf node (must not have children)
Cps::delete_node(origin, 2)?;
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

## Performance Optimizations

### Ancestor Path Storage

Each node stores its complete path from the root, providing several performance benefits:

1. **O(1) Cycle Detection**: Moving a node requires only checking if the target parent ID exists in the node's path array, avoiding recursive tree traversal
2. **O(1) Depth Validation**: Tree depth is immediately available from the path length without traversing parent links
3. **No Loop Instructions**: All validation operations use simple array operations instead of loops, making the pallet more efficient
4. **Predictable Gas Costs**: Path-based operations have constant time complexity regardless of tree depth

The path is automatically updated when nodes are moved, with recursive updates propagating to all descendants.

### Compact SCALE Encoding

Node paths use custom compact SCALE encoding for storage efficiency:

- Each `NodeId` (u64) in the path is encoded using SCALE compact format, reducing storage size for small node IDs (which are common in typical tree structures)
- Path length is also encoded as compact, optimizing for typical short paths
- This can reduce storage costs by 50-87% for paths with small node IDs (< 2^14)
- Example: A path with 3 small node IDs (< 16384) uses ~6 bytes instead of 24 bytes

## Security Considerations

- **Ownership**: Only node owners can modify their nodes
- **Cycle Prevention**: The `move_node` extrinsic includes cycle detection to maintain tree integrity
- **Bounded Collections**: All collections use bounded types to prevent DoS attacks
- **Tree Depth**: Configurable depth limit prevents stack overflow during traversal
- **Data Validation**: All data must fit within configured size limits

## License

Apache License 2.0
