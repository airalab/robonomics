# CPS Pallet

`pallet-robonomics-cps` keeps a registry of cyber-physical systems (buildings,
machines, sensors, and so on) on-chain as a tree of nodes. Each node can carry
metadata and payload bytes. Ownership is organized in **Scopes**, and a Scope
owner can delegate permissions on individual nodes or subtrees to other
accounts through **Access** entries.

## Concepts

### Node tree

```
Building-A
├── Floor-1
│   ├── HVAC-01
│   └── Lights
└── Floor-2
    └── HVAC-02
```

- Node IDs (`NodeId`) are allocated sequentially and never reused.
- A node's parent is set at creation and never changes; there is no call to
  move a node. To relocate an object, create a new node under the new parent
  and delete the old one.
- A node has at most `MAX_CHILDREN_PER_NODE` direct children.
- Only a node without children can be deleted.
- Each node may have **metadata** (`Meta`, up to 1 KiB) and a **payload**
  (`Payload`, up to 8 KiB). Both are opaque bytes stored as given, in
  separate storage maps; a missing entry means the field is unset.

### Scopes

A Scope is rooted at a node and has one owner. Every node resolves to exactly
one Scope: the Scope rooted at the node itself, otherwise the Scope rooted at
its nearest ancestor that roots one.

```
City                 root of Scope #1, owner A
`-- Smart Building   root of Scope #7, owner B
    `-- Floor 3      resolves to Scope #7
```

- Creating a root node creates a new Scope owned by the creator.
- `create_scope` on any other node makes it the root of a new nested Scope
  owned by the caller.
- A nested Scope is a boundary: the owner and Access entries of an enclosing
  Scope have no effect inside it, even when both Scopes have the same owner.
  In the example, `A` has no rights on `Smart Building` or `Floor 3`. The one
  exception is that `A`, as owner of the immediately enclosing Scope, may
  replace Scope #7 (see [Replacing a Scope](#replacing-a-scope)).
- Scope IDs (`ScopeId`) are allocated sequentially and never reused. A
  Scope's owner never changes; replacing the Scope is the only way to change
  who controls it.

**Scope-local depth.** The path from a node up to its Scope root, both ends
included, holds at most `MAX_SCOPE_DEPTH` nodes. A nested Scope root starts a
new count, so the depth of the whole tree is not limited, while Scope
resolution never visits more than `MAX_SCOPE_DEPTH` nodes.

### Capabilities and Access

The Scope owner may do everything on every node of the Scope. Other accounts
need an Access entry, which only the Scope owner can grant or revoke. An
Access entry gives an account a `Capability` at a node, in one of two modes:

- `GrantMode::Node` - the granted node only;
- `GrantMode::Subtree` - the granted node and its descendants in the same
  Scope. It does not reach into nested Scopes.

| Capability    | Allows                                                                                          |
|---------------|-------------------------------------------------------------------------------------------------|
| `Write`       | `set_meta`, `set_payload`                                                                       |
| `CreateScope` | `create_scope`: a new nested Scope at an ordinary node, or replacement of the Scope at its root |

Adding child nodes, deleting nodes, and granting or revoking Access require
the Scope owner and cannot be delegated.

Example:

```
Factory              root of Scope #10, owner A
├── Robot
└── Laboratory       root of Scope #20, owner B
    └── Sensor
```

If `A` grants `Gateway` the `Write` capability at `Factory` with
`GrantMode::Subtree`, `Gateway` can update `Factory` and `Robot`, but not
`Laboratory` or `Sensor`, which belong to Scope #20.

### Access entry limit

A Scope holds at most `MAX_ACCESS_ENTRIES_PER_SCOPE` Access entries, one per
distinct `(node, account)` pair. The first grant for a pair takes a slot;
adding another capability to the pair or changing its mode does not. Revoking
the last capability of a pair deletes the entry and frees its slot. When a
Scope is replaced or its root node deleted, all of its Access entries are
deleted in the same call.

An Access entry is stored under the Scope its node belonged to when it was
granted, while `revoke_access` works on the Scope the node belongs to at call
time. An entry at a node that was later deleted, or that has since become
part of a nested Scope, therefore no longer grants anything and can no longer
be revoked; it keeps occupying a slot until its Scope is replaced or removed.
The Scope owner can free these slots by replacing the Scope with
`create_scope` on its root, which deletes all of the Scope's entries.

### Replacing a Scope

`create_scope` on a node that already roots a Scope replaces that Scope with a
new one, with a new `ScopeId` and the caller as owner. Nodes of the old Scope
now belong to the new one, and the old Scope's Access entries are deleted.
The replacement may be made by:

- the owner of the Scope;
- an account holding `CreateScope` at the Scope root;
- the owner of the immediately enclosing Scope, i.e. the Scope that the
  root's parent belongs to. Owners of Scopes further up the tree and Access
  holders of the enclosing Scope may not.

### Removing a Scope

A Scope is removed only together with its root node: `delete_node` on a
Scope root without children deletes the node, the Scope, and its Access
entries.

## Calls

| Call                                                   | Allowed caller                                                               |
|--------------------------------------------------------|------------------------------------------------------------------------------|
| `create_node(parent_id: None, meta, payload)`          | any signed account; the caller becomes owner of the new root's Scope         |
| `create_node(parent_id: Some(parent), meta, payload)`  | owner of the parent's Scope                                                  |
| `set_meta(node_id, meta)`                              | holder of `Write` at the node                                                |
| `set_payload(node_id, payload)`                        | holder of `Write` at the node                                                |
| `delete_node(node_id)`                                 | owner of the node's Scope                                                    |
| `create_scope(node_id)`                                | holder of `CreateScope` at the node; for a nested Scope root, also the owner of the enclosing Scope |
| `grant_access(node_id, principal, capability, mode)`   | owner of the node's Scope                                                    |
| `revoke_access(node_id, principal, capability)`        | owner of the node's Scope                                                    |

A "holder of a capability at the node" is the Scope owner, an account granted
the capability at the node, or an account granted it with `GrantMode::Subtree`
at an ancestor in the same Scope. Notes:

- `set_meta` / `set_payload` with `None` remove the field.
- `create_node` fails with `MaxScopeDepthExceeded` if the new node would
  exceed `MAX_SCOPE_DEPTH`, and with `TooManyChildren` if the parent already
  has `MAX_CHILDREN_PER_NODE` children.
- `delete_node` also removes the node's metadata and payload, and, if the
  node roots a Scope, the Scope and its Access entries.
- `revoke_access` succeeds, without changing storage, if the capability was
  not granted.
- `delete_node` and `create_scope` are charged for deleting
  `MAX_ACCESS_ENTRIES_PER_SCOPE` Access entries and refund the difference to
  the number actually deleted.

## Events

| Event                                                         | Emitted by                                                    |
|---------------------------------------------------------------|---------------------------------------------------------------|
| `NodeCreated(node_id, parent_id, creator)`                    | `create_node`                                                 |
| `MetaSet(node_id, sender)`                                    | `set_meta`                                                    |
| `PayloadSet(node_id, sender)`                                 | `set_payload`                                                 |
| `NodeDeleted(node_id, sender)`                                | `delete_node`                                                 |
| `ScopeCreated(scope_id, root, owner)`                         | `create_node` for a root node, `create_scope`                 |
| `ScopeDeleted(scope_id, root)`                                | `delete_node` on a Scope root                                 |
| `AccessGranted(scope_id, node_id, principal, capability, mode)` | `grant_access`                                              |
| `AccessRevoked(scope_id, node_id, principal, capability)`     | `revoke_access`                                               |

When `create_scope` replaces a Scope, only `ScopeCreated` for the new Scope is
emitted.

## Storage

| Item          | Key                                | Value                                                        |
|---------------|------------------------------------|--------------------------------------------------------------|
| `NextNodeId`  | -                                  | next `NodeId`                                                |
| `Nodes`       | `NodeId`                           | `NodeInfo { parent, scope }`; `scope` is set on Scope roots only |
| `Meta`        | `NodeId`                           | metadata bytes                                               |
| `Payload`     | `NodeId`                           | payload bytes                                                |
| `Children`    | `NodeId`                           | direct children, in creation order                           |
| `NextScopeId` | -                                  | next `ScopeId`                                               |
| `Scopes`      | `ScopeId`                          | `ScopeInfo { owner, access_count }`                          |
| `Access`      | `ScopeId`, `(NodeId, AccountId)`   | capabilities and their modes                                 |

`NodeId`, `ScopeId`, and `access_count` use SCALE compact encoding: values
below 64 take 1 byte, below 16 384 take 2 bytes, below 2^30 take 4 bytes.

## Constants

The limits are crate constants; changing them requires a runtime upgrade.

| Constant                       | Value | Meaning                                                            |
|--------------------------------|-------|--------------------------------------------------------------------|
| `MAX_META_SIZE`                | 1024  | maximum metadata size, in bytes                                    |
| `MAX_PAYLOAD_SIZE`             | 8192  | maximum payload size, in bytes                                     |
| `MAX_SCOPE_DEPTH`              | 32    | maximum nodes on the path from a node to its Scope root, inclusive |
| `MAX_CHILDREN_PER_NODE`        | 100   | maximum direct children of a node                                  |
| `MAX_ACCESS_ENTRIES_PER_SCOPE` | 32    | maximum Access entries (distinct `(node, account)` pairs) per Scope |

## Runtime API

`pallet-robonomics-cps-runtime-api` defines `CpsApi` with two read-only
queries, implemented with the pallet's own functions:

- `resolve_scope(node) -> Option<ResolvedScope<AccountId>>` - the Scope's
  `id`, `root` node, `owner`, and the `path` from `node` to the Scope root;
  `None` if the node does not exist or no Scope can be resolved.
- `has_capability(node_id, account_id, capability) -> bool` - whether the
  account may use `capability` at the node, with the same checks as the calls
  that require it: Scope ownership or Access entries, and for `CreateScope`
  also ownership of the immediately enclosing Scope at a nested Scope root;
  `false` if the node does not exist.

Clients call them through the `state_call` RPC at any block.

## Data privacy

All data stored by the pallet is public, including the tree structure, the
metadata and payload bytes, their sizes, and the times of updates. The pallet
does not encrypt anything. To keep metadata or payload confidential, encrypt
it before submitting it.

[libcps](https://github.com/airalab/robins/tree/master/crates/libcps)
implements such an encryption scheme: ECDH key agreement with SR25519 or
ED25519 keys, HKDF-SHA256 key derivation, and AEAD encryption with
XChaCha20-Poly1305, AES-256-GCM, or ChaCha20-Poly1305.

## Integration

1. Add the dependencies to the runtime (workspace dependencies in this
   repository):
   ```toml
   pallet-robonomics-cps = { workspace = true }
   pallet-robonomics-cps-runtime-api = { workspace = true }
   ```
   and enable their `std` features in the runtime's `std` feature.

2. Configure the pallet:
   ```rust
   impl pallet_robonomics_cps::Config for Runtime {
       type RuntimeEvent = RuntimeEvent;
       type WeightInfo = weights::pallet_robonomics_cps::WeightInfo<Runtime>;
   }
   ```

3. Add it to the runtime:
   ```rust
   #[runtime::pallet_index(..)]
   pub type CPS = pallet_robonomics_cps;
   ```

4. Implement the runtime API:
   ```rust
   impl pallet_robonomics_cps_runtime_api::CpsApi<Block, AccountId> for Runtime {
       fn resolve_scope(
           node: pallet_robonomics_cps::NodeId,
       ) -> Option<pallet_robonomics_cps::ResolvedScope<AccountId>> {
           CPS::resolve_scope(node).ok()
       }

       fn has_capability(
           node_id: pallet_robonomics_cps::NodeId,
           account_id: AccountId,
           capability: pallet_robonomics_cps::Capability,
       ) -> bool {
           CPS::has_capability(node_id, &account_id, capability)
       }
   }
   ```

5. On a chain with storage version 1 of the pallet, add
   `pallet_robonomics_cps::migration::MigrationToV2<Runtime>` to the runtime
   migrations.

## Client usage

With polkadot.js, for a runtime where the pallet is named `CPS`:

```javascript
// Read node data.
const info = await api.query.cps.nodes(nodeId);        // parent and Scope pointer
const meta = await api.query.cps.meta(nodeId);
const payload = await api.query.cps.payload(nodeId);
const children = await api.query.cps.children(nodeId);

// Resolve the Scope and check a capability through the runtime API.
const scope = await api.call.cpsApi.resolveScope(nodeId);
const canWrite = await api.call.cpsApi.hasCapability(nodeId, accountId, 'Write');

// Create a root node and a child.
await api.tx.cps.createNode(null, meta, payload).signAndSend(owner);
await api.tx.cps.createNode(parentId, meta, payload).signAndSend(owner);

// Make a node the root of a new Scope.
await api.tx.cps.createScope(nodeId).signAndSend(owner);

// Let another account write to a single node.
await api.tx.cps.grantAccess(nodeId, principal, 'Write', 'Node').signAndSend(owner);
```

## Testing

```bash
cargo test -p pallet-robonomics-cps
cargo test -p pallet-robonomics-cps --features runtime-benchmarks  # also runs the benchmark tests
```

## License

Apache License 2.0 - see [LICENSE](../../LICENSE).
