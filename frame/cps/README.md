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

### Node identity

A node may have an **identity**: an account bound to it 1:1, so clients can
address the tree by account while the tree, Scope resolution, storage keys,
and references stay in `NodeId` space. Identity is optional; a node without
one works the same.

- An account is the identity of at most one node. `NodeIdentity` and
  `IdentityNode` always agree.
- Only the owner of the node's Scope can set (`set_identity`) or clear
  (`clear_identity`) it. `Write` access is not enough. An identity can be
  replaced without changing the `NodeId`.
- Deleting a node removes its identity.
- If the identity is the caller, no further proof is needed. Otherwise the
  identity account must sign a **permit**.
- `create_node` can bind an identity in the same call. A new root node accepts
  only the caller as identity, because the Scope a permit has to name does not
  exist before the root is created; a delegated identity can be set later with
  `set_identity`.

**Permit.** The identity account signs the SCALE encoding of the tuple

```
(b"robonomics:cps:node-identity:v1", genesis_hash, scope_id, valid_until)
```

(the domain as a `Vec<u8>`, i.e. with a compact length prefix), over the raw
bytes, without a `<Bytes>` wrapper. `Pallet::identity_permit_payload` builds
it. The permit must not be expired, and `valid_until` may be at most
`MaxIdentityPermitTtl` blocks ahead (1 day, 14 400 blocks, in the Robonomics
runtime). Call arguments carry `NodeIdentityPermit { valid_until, signature }`.

The payload names no node, delegate, or nonce. A `ScopeId` is never reused and
its owner never changes, and only the Scope owner can use a permit, so the
`ScopeId` already fixes who may consume it:

- the permit is not consumed; the owner may clear an identity and reuse the
  same permit for another node of the Scope until it expires;
- an account is still the identity of one node at a time
  (`IdentityAlreadyInUse`);
- replacing the Scope gives its nodes a new `ScopeId`, which invalidates the
  old permits.

> **Warning.** This relies on identity administration being limited to the
> Scope owner. If it is ever delegated (for example through a `ManageIdentity`
> capability), the permit payload must also name the delegate; do not add such
> a capability without changing the permit format.

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
| `create_node(parent_id: None, meta, payload, identity)` | any signed account; the caller becomes owner of the new root's Scope         |
| `create_node(parent_id: Some(parent), meta, payload, identity)` | owner of the parent's Scope                                                  |
| `set_meta(node_id, meta)`                              | holder of `Write` at the node                                                |
| `set_payload(node_id, payload)`                        | holder of `Write` at the node                                                |
| `delete_node(node_id)`                                 | owner of the node's Scope                                                    |
| `create_scope(node_id)`                                | holder of `CreateScope` at the node; for a nested Scope root, also the owner of the enclosing Scope |
| `grant_access(node_id, principal, capability, mode)`   | owner of the node's Scope                                                    |
| `revoke_access(node_id, principal, capability)`        | owner of the node's Scope                                                    |
| `set_identity(node_id, identity, permit)`              | owner of the node's Scope; `permit` is needed unless `identity` is the caller |
| `clear_identity(node_id)`                              | owner of the node's Scope                                                    |

A "holder of a capability at the node" is the Scope owner, an account granted
the capability at the node, or an account granted it with `GrantMode::Subtree`
at an ancestor in the same Scope. Notes:

- `set_meta` / `set_payload` with `None` remove the field.
- `create_node` fails with `MaxScopeDepthExceeded` if the new node would
  exceed `MAX_SCOPE_DEPTH`, and with `TooManyChildren` if the parent already
  has `MAX_CHILDREN_PER_NODE` children.
- `delete_node` also removes the node's metadata and payload, and, if the
  node roots a Scope, the Scope and its Access entries.
- `identity` of `create_node` is optional. A root node accepts only the caller
  as identity (`RootIdentityMustBeCaller` otherwise); for a child node, a
  different account needs a permit for the parent's Scope.
- `set_identity` fails with `IdentityPermitRequired`, `IdentityPermitExpired`,
  `IdentityPermitTooLong`, or `InvalidIdentityPermit` if the permit is
  missing or not acceptable, and with `IdentityAlreadyInUse` if the account
  is the identity of another node. `clear_identity` fails with
  `IdentityNotSet` for a node without identity.
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
| `IdentitySet { node, identity }`                             | `set_identity`, `create_node` with an identity                |
| `IdentityCleared { node, identity }`                          | `clear_identity`, `delete_node` of a node with an identity    |

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
| `NodeIdentity` | `NodeId`                          | identity account of the node                                 |
| `IdentityNode` | `AccountId`                       | node the account is the identity of                          |

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

## View functions

The pallet exposes read-only queries of derived CPS semantics as
[pallet view functions](https://docs.rs/frame-support/latest/frame_support/pallet_macros/attr.view_functions.html),
implemented by the same logic as the calls:

- `resolve_scope(node_id) -> Result<ResolvedScope<AccountId>, Error>` - the
  Scope's `id`, `root` node, `owner`, and the `path` from the node to the
  Scope root; `NodeNotFound` or `ScopeNotFound` if no Scope can be resolved.
- `has_capability(node_id, account, capability) -> Result<bool, Error>` - whether the
  account may use `capability` at the node, with the same checks as the calls
  that require it: Scope ownership or Access entries, and for `CreateScope`
  also ownership of the immediately enclosing Scope at a nested Scope root;
  `false` if access is denied, `NodeNotFound` if the node does not exist and
  `ScopeNotFound` if no Scope can be resolved for it.
- `identity_of(node_id) -> Option<AccountId>` - the identity of the node, if
  it has one.
- `node_by_identity(account) -> Option<NodeId>` - the node the account is the
  identity of, if any.

They are described in runtime metadata V16 and executed through the generic
`RuntimeViewFunction` runtime API, so no CPS-specific runtime API is needed.
Simple stored-state reads (node info, metadata, payload, children) stay plain
storage queries.

### Address-oriented views

These views let a client work with the tree as if nodes were addressed by
`AccountId`. They take an identity account, resolve it through `IdentityNode`,
run on the canonical `NodeId` tree and return only accounts, `ScopeId`,
`Capability` and stored data; a `NodeId` is never returned. All return
`Result<_, AddressViewError>`:

- `parent(identity) -> Option<AccountId>` - `None` for a root.
- `children(identity) -> Vec<AccountId>` - the direct children in creation
  order, at most `MAX_CHILDREN_PER_NODE`.
- `meta(identity) -> Option<NodeMeta>` and `payload(identity) ->
  Option<NodePayload>`.
- `scope_of(identity) -> ScopeId` - the id of the Scope of the node.
- `address_capability(caller, target, capability) -> bool` - the same
  check as `has_capability`, at the node `target` is the identity of.

The projection is strict. `NodeIdentity` is optional, so a result that has to
contain a node without identity fails as a whole with
`MissingProjectedIdentity`; nodes are never silently omitted. For example, if
Alice has the children Bob, Carol and a node without identity,
`children(Alice)` fails rather than return `[Bob, Carol]`, which would show a
different topology. An unlinked input account fails with `IdentityNotFound`.
Only nodes that appear in the result need an identity. `scope_of` and
`address_capability` show no node, so only the input account has to be
linked.

There is no filtered view ("children with identity only", "nearest identified
parent") and no recursive subtree view. A client walks the hierarchy one
bounded call at a time. The views read storage only and add none.

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
   ```
   and enable its `std` feature in the runtime's `std` feature.

2. Configure the pallet:
   ```rust
   impl pallet_robonomics_cps::Config for Runtime {
       type RuntimeEvent = RuntimeEvent;
       type WeightInfo = weights::pallet_robonomics_cps::WeightInfo<Runtime>;
       // Signature scheme of identity permits; its signer must identify `AccountId`.
       type Signature = sp_runtime::MultiSignature;
       // Longest validity of an identity permit, in blocks (1 day).
       type MaxIdentityPermitTtl = ConstU32<{ DAYS }>;
       #[cfg(feature = "runtime-benchmarks")]
       type BenchmarkHelper = CpsBenchmarkHelper;
   }
   ```

3. Add it to the runtime:
   ```rust
   #[runtime::pallet_index(..)]
   pub type CPS = pallet_robonomics_cps;
   ```

4. Derive `RuntimeViewFunction` in the `#[frame_support::runtime]` and expose
   the generic view-function runtime API:
   ```rust
   impl frame_support::view_functions::runtime_api::RuntimeViewFunction<Block> for Runtime {
       fn execute_view_function(
           id: frame_support::view_functions::ViewFunctionId,
           input: Vec<u8>,
       ) -> Result<Vec<u8>, frame_support::view_functions::ViewFunctionDispatchError> {
           Runtime::execute_view_function(id, input)
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

// Resolve the Scope and check a capability through the pallet view functions.
const scope = await api.view.cps.resolveScope(nodeId);
const canWrite = await api.view.cps.hasCapability(nodeId, accountId, 'Write');

// Create a root node and a child.
await api.tx.cps.createNode(null, meta, payload, null).signAndSend(owner);
await api.tx.cps.createNode(parentId, meta, payload, null).signAndSend(owner);

// Make a node the root of a new Scope.
await api.tx.cps.createScope(nodeId).signAndSend(owner);

// Let another account write to a single node.
await api.tx.cps.grantAccess(nodeId, principal, 'Write', 'Node').signAndSend(owner);

// Use the owner's own account as the identity of a node.
await api.tx.cps.setIdentity(nodeId, owner.address, null).signAndSend(owner);

// Use another account as the identity: it signs a permit for the node's Scope
// first. `scopeId` comes from `api.view.cps.resolveScope(nodeId)`.
const validUntil = (await api.rpc.chain.getHeader()).number.toNumber() + 1000;
const genesis = api.genesisHash;
const payload = api.createType('(Vec<u8>, H256, Compact<u64>, u32)',
  [stringToU8a('robonomics:cps:node-identity:v1'), genesis, scopeId, validUntil]);
const signature = robot.sign(payload.toU8a());   // raw bytes, no <Bytes> wrapper
await api.tx.cps
  .setIdentity(nodeId, robot.address, { validUntil, signature })
  .signAndSend(owner);

// Address-based lookups.
const identity = await api.view.cps.identityOf(nodeId);
const node = await api.view.cps.nodeByIdentity(robot.address);
```

## Testing

```bash
cargo test -p pallet-robonomics-cps
cargo test -p pallet-robonomics-cps --features runtime-benchmarks  # also runs the benchmark tests
```

## License

Apache License 2.0 - see [LICENSE](../../LICENSE).
