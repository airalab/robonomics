///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
//! # On-chain Hierarchical Tree for Cyber-Physical Systems
//!
//! This pallet provides a decentralized registry for cyber-physical systems organized
//! as a hierarchical tree structure, with authority and resources delegated through
//! a **Scope / Access** architecture.
//!
//! ## Architecture
//!
//! ### Storage Layout
//!
//! 1. **`Nodes`**: Mapping `NodeId` → `NodeInfo { parent, scope }`
//!    - Presence of a key means the node exists
//!    - `parent` is `None` for a root node and `Some(parent_id)` otherwise -
//!      **immutable** once a node is created, there is no way to change it
//!    - `scope` is `Some(scope_id)` only on nodes that are the root of an
//!      active [`Scope`](self#scope); a node with `scope: None` resolves to
//!      the Scope of the nearest ancestor that has one (see
//!      [`Pallet::resolve_scope`])
//!
//! 2. **`Meta`**: Mapping `NodeId` → `NodeMeta` (≤1 KiB)
//!    / **`Payload`**: Mapping `NodeId` → `NodePayload` (≤8 KiB)
//!    - An entry is present only when the corresponding field has been set;
//!      absence means "unset", not "empty"
//!
//! 3. **`Scopes`**: Mapping `ScopeId` → `ScopeInfo<AccountId> { owner,
//!    access_count }`
//!    - The canonical location for a Scope's owner and its scope-local
//!      accounting (currently just `access_count`, the number of physical
//!      `Access` entries; see
//!      [`Bounded Access and synchronous Scope cleanup`](self#bounded-access-and-synchronous-scope-cleanup)
//!      below)
//!    - `NodeInfo.scope` stores only the current `ScopeId`; everything else
//!      about a Scope lives here
//!
//! 4. **`Access`**: Mapping `(ScopeId, (NodeId, AccountId))` → `AccessFlags`.
//!    A single compact entry per `(Scope, node, principal)` triple, bit-packing
//!    every delegated [`Capability`] and its [`GrantMode`] (see
//!    [`AccessFlags`]).
//!
//! 5. **`Children`**: Index structure for O(1) child lookups, kept separate
//!    from `NodeInfo` so child vectors never inflate authorization proofs.
//!
//! ### Scope
//!
//! A `Scope` is the administrative and economic boundary of the CPS
//! hierarchy. `ScopeId` is a globally unique, never-reused numeric
//! identifier. Every active CPS node resolves to exactly one Scope by
//! walking `parent` links until the nearest ancestor with a `NodeInfo.scope`
//! entry is found (see [`Pallet::resolve_scope`]):
//!
//! ```text
//! City
//! Scope #1 / owner=A
//! |
//! `-- Smart Building
//!     Scope #7 / owner=B
//!     |
//!     `-- Floor 3
//! ```
//!
//! `Floor 3` resolves to `Scope #7` (owner `B`); `Smart Building`'s Scope has
//! no implicit administrative rights over `City`'s other, independently
//! owned children, and vice versa. A nested Scope is always a hard boundary:
//! it stops inheritance of authority, `Access`, and resource limits, even
//! when parent and child Scope owners are the same account.
//!
//! ### Creating and Replacing a Scope
//!
//! Every CPS root is allocated a fresh Scope, owned by its creator, when the
//! root is created. [`Pallet::create_scope`] is the single operation for
//! establishing *and* replacing a Scope boundary:
//!
//! - if `node_id` is an ordinary node (`Nodes[node_id].scope == None`), it
//!   establishes a brand-new nested Scope there (owner authority reaching
//!   `node_id`, or a delegated [`Capability::CreateScope`] grant reaching
//!   it);
//! - if `node_id` is already an active Scope root
//!   (`Nodes[node_id].scope == Some(old_scope_id)`), it replaces that
//!   Scope's generation in place: a fresh `ScopeId` is allocated, the caller
//!   becomes the new owner, and the boundary stays on the same node. This is
//!   the sole ownership-transfer mechanism.
//!
//! Either way, a brand-new `ScopeId` is always allocated. On replacement,
//! the previous Scope's `Access` entries become immediately inactive
//! without requiring any descendant rewrite; the old Scope's remaining
//! physical state is synchronously cleared in the same call (see
//! [`Bounded Access and synchronous Scope cleanup`](self#bounded-access-and-synchronous-scope-cleanup)
//! below).
//!
//! Changing control of a Scope means replacing it with another Scope; a
//! Scope's `owner` is immutable once created. There is no transfer/accept
//! state machine.
//!
//! ### Deleting a Scope
//!
//! There is no standalone "delete a Scope, keep the node" operation - a
//! Scope boundary must never disappear while its node remains, since that
//! could merge two independently-bounded Scope segments into one segment
//! exceeding `MAX_SCOPE_DEPTH`. A Scope only ever disappears together with
//! its root node: [`Pallet::delete_node`] on a Scope-root *leaf* removes the
//! node and its Scope's remaining physical state (`Access`) in the same
//! call. A Scope root with children can never be deleted (see
//! [`Pallet::delete_node`]).
//!
//! ### Bounded Access and synchronous Scope cleanup
//!
//! `MAX_ACCESS_ENTRIES_PER_SCOPE` places a hard upper bound on the
//! number of physical `Access` entries (distinct `(NodeId, AccountId)`
//! pairs) any single Scope may hold at once, tracked in
//! [`ScopeInfo::access_count`]. [`Pallet::grant_access`] enforces the bound -
//! rejecting a brand-new entry past the limit with
//! [`Error::TooManyAccessEntries`] - while changing an existing entry's
//! `GrantMode` or adding another `Capability` bit never consumes an
//! additional slot. [`Pallet::revoke_access`] mirrors this: only fully
//! revoking an entry's last capability (removing it) frees a slot.
//!
//! Because `Scopes[scope_id].access_count <= MAX_ACCESS_ENTRIES_PER_SCOPE`
//! always holds, invalidating a Scope generation (via a
//! [`Pallet::create_scope`] replacement, or deleting a Scope-root leaf via
//! [`Pallet::delete_node`]) can synchronously delete every remaining
//! `Access` entry for it in one step, in the same extrinsic:
//!
//! ```text
//! invalidate ScopeId
//!     -> clear_prefix(Access(scope_id, *), MAX_ACCESS_ENTRIES_PER_SCOPE)
//!     -> remove Scopes[scope_id]
//!     -> done
//! ```
//!
//! There is no deferred cleanup queue, continuation cursor, or `on_idle`
//! background pass: the bound guarantees the single `clear_prefix` call
//! always finishes the prefix. A `clear_prefix` call that (contrary to the
//! bound) reports leftover work is treated as an invariant violation (see
//! `Pallet::clear_scope_access`), never as something to silently retry or
//! defer.
//!
//! `ScopeId`s are never reused, including after cleanup: a cleaned-up
//! `ScopeId` simply has no more physical state, but remains a valid
//! historical identifier that will never be handed out again by
//! [`Pallet::create_scope`]. A `ScopeId` is only ever cleared in the same
//! state transition that removes or replaces the `NodeInfo.scope` entry
//! pointing at it, so a cleared Scope can never become active again.
//!
//! ### Access
//!
//! [`Pallet::grant_access`] / [`Pallet::revoke_access`] let a Scope owner
//! delegate a [`Capability`] to another account at a specific `NodeId`,
//! either for that exact node ([`GrantMode::Node`]) or for the node and all
//! its descendants within the same Scope ([`GrantMode::Subtree`]). Access
//! never crosses a nested Scope boundary. The Scope owner always has
//! implicit authority and does not need explicit `Access` entries.
//!
//! ### Structural Immutability
//!
//! A node's parent is fixed at creation time and never changes. Relocating
//! an object is represented as creating a new node under the desired parent
//! and deleting the old one; the new node receives a fresh `NodeId`,
//! which is never reused.
//!
//! ### Performance Characteristics
//!
//! Core operation time complexity:
//! - **Scope resolution**: [`Pallet::resolve_scope`] (built on the internal
//!   `resolve_scope_path`) walks `parent` links one hop at a time until a
//!   `NodeInfo.scope` entry is found → O(scope-local depth), bounded by
//!   `MAX_SCOPE_DEPTH` - never by the (unbounded) global tree depth
//! - **Depth validation**: reuses the same resolved path's length, no
//!   separate traversal
//! - **Child lookup**: Direct index access via `Children` → O(1)
//!
//! ## Usage Examples
//!
//! ### Creating a Root Node
//!
//! ```ignore
//! use pallet_robonomics_cps::NodeMeta;
//! use frame_support::BoundedVec;
//!
//! // Plain metadata
//! let meta: Option<NodeMeta> = Some(BoundedVec::try_from(b"sensor_config".to_vec()).unwrap());
//!
//! // Create root (parent = None) - the caller becomes the owner of a new
//! // Scope allocated for this root.
//! Cps::create_node(origin, None, meta, None)?;
//! ```
//!
//! ### Creating a Child Node
//!
//! ```ignore
//! use pallet_robonomics_cps::{NodeId, NodePayload};
//!
//! // Data can be encrypted by client before submission
//! let encrypted_bytes = client_side_encrypt(sensitive_data);
//! let payload: Option<NodePayload> = Some(BoundedVec::try_from(encrypted_bytes).unwrap());
//!
//! // Create child under node 0. The caller must hold `Write` authority
//! // (Scope owner, or matching Access) over node 0's resolved Scope.
//! Cps::create_node(origin, Some(NodeId(0)), None, payload)?;
//! ```
//!
//! ### Establishing a Nested Scope
//!
//! ```ignore
//! // Node 5 currently inherits its Scope from an ancestor. Its owner
//! // establishes a new, independent Scope rooted at node 5.
//! Cps::create_scope(origin, NodeId(5))?;
//! ```
//!
//! ### Querying the Tree
//!
//! ```ignore
//! // Get a node's parent (existence check + parent link)
//! let parent = Nodes::<T>::get(NodeId(0)).ok_or(Error::<T>::NodeNotFound)?.parent;
//!
//! // Get all children
//! let children = Children::<T>::get(NodeId(0));
//!
//! // Resolve the effective Scope for a node
//! let scope = Cps::resolve_scope(NodeId(0))?;
//! ```
//!
//! ## Security Invariants
//!
//! The pallet maintains the following invariants:
//!
//! 1. **No Cycles**: The tree is acyclic and `parent` is immutable, so cycles
//!    cannot be created after node creation.
//! 2. **Scope Resolution**: Every active node resolves to exactly one Scope
//!    via [`Pallet::resolve_scope`] - the nearest active Scope ancestor wins.
//! 3. **Scope Boundaries**: Nested Scopes are a hard authority and resource
//!    boundary; ancestor Scope owners have no implicit administrative rights
//!    inside a nested Scope.
//! 4. **Index Consistency**: `Children` stays synchronized with `Nodes`'
//!    `parent` links.
//! 5. **Deletion Safety**: Cannot delete nodes with children.
//! 6. **Scope-Local Depth Limits**: The distance from any node to its
//!    nearest Scope root never exceeds `MAX_SCOPE_DEPTH`; the global tree
//!    depth is unbounded.
//! 7. **Stable Identity**: `NodeId` is never reused, and neither is `ScopeId`.
//! 8. **Immutable Scope Owner/Root**: A Scope's owner and root are fixed at
//!    creation; changing control means replacing it with another Scope.
//! 9. **Bounded Access / Synchronous Cleanup**: Every Scope holds at most
//!    `MAX_ACCESS_ENTRIES_PER_SCOPE` physical `Access` entries
//!    ([`ScopeInfo::access_count`] tracks the exact count); invalidating a
//!    Scope generation always synchronously removes its entire `Access`
//!    prefix and its `Scopes` entry in the same call - there is no deferred
//!    cleanup, and no stale Scope's physical state ever survives past the
//!    call that invalidated it.
//!
//! ## Testing
//!
//! Run the comprehensive test suite:
//!
//! ```bash
//! cargo test -p pallet-robonomics-cps
//! ```
//!
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migration;
pub mod weights;

#[cfg(test)]
mod tests;

pub use pallet::*;
pub use weights::WeightInfo;

use core::fmt::Debug;
use frame_support::{traits::ConstU32, BoundedVec};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Maximum size for node metadata.
///
/// Set to 1 KiB: metadata is meant to hold small structured information
/// (e.g. a sensor's configuration), not application payloads. Large objects
/// should remain external to CPS and be referenced by hash/CID/etc.
pub const MAX_META_SIZE: u32 = 1024;

/// Maximum size for node payload.
///
/// Set to 8 KiB to accommodate typical sensor readings, application data,
/// and encrypted payloads while preventing DoS attacks via large data
/// submissions.
pub const MAX_PAYLOAD_SIZE: u32 = 8192;

/// Maximum depth (number of ancestors up to the nearest Scope root) a node
/// may have *within a single Scope*.
///
/// Enforced at `create_node` time using the length of the resolved parent
/// path; bounds the cost of `resolve_scope` and authorization, both
/// O(scope-local depth). The global tree depth (across nested Scopes) is
/// not bounded by this constant.
pub const MAX_SCOPE_DEPTH: u32 = 32;

/// Maximum number of direct children a single node may have.
///
/// Bounds the size of the `Children` index entry for any given node.
pub const MAX_CHILDREN_PER_NODE: u32 = 100;

/// Hard upper bound on the number of physical `Access` entries (distinct
/// `(NodeId, AccountId)` pairs) that may exist under a single `ScopeId` at
/// once.
///
/// This bound is what makes synchronous Scope invalidation possible:
/// `Access::<T>::clear_prefix(scope_id, MAX_ACCESS_ENTRIES_PER_SCOPE, None)`
/// is guaranteed to remove every entry for `scope_id` in a single call, so
/// there is no need for a deferred/background cleanup queue. Unlike the
/// other bounds above, this one used to be runtime-configurable via
/// `Config::MaxAccessEntriesPerScope`; it is now a crate constant like every
/// other structural bound in this pallet - changing it requires a code
/// change (and a runtime upgrade, with a migration proving no existing
/// Scope already exceeds the new bound if lowered).
pub const MAX_ACCESS_ENTRIES_PER_SCOPE: u32 = 32;

/// [`ConstU32`] wrapper around [`MAX_META_SIZE`] for use as a `BoundedVec` bound.
pub type MaxMetaSize = ConstU32<MAX_META_SIZE>;
/// [`ConstU32`] wrapper around [`MAX_PAYLOAD_SIZE`] for use as a `BoundedVec` bound.
pub type MaxPayloadSize = ConstU32<MAX_PAYLOAD_SIZE>;
/// [`ConstU32`] wrapper around [`MAX_SCOPE_DEPTH`] for use as a `BoundedVec` bound.
pub type MaxScopeDepth = ConstU32<MAX_SCOPE_DEPTH>;
/// Maximum length of a [`ResolvedScopePath::path`]: the target node plus up
/// to `MAX_SCOPE_DEPTH` ancestors up to (and including) the Scope root.
pub const MAX_SCOPE_PATH_LEN: u32 = MAX_SCOPE_DEPTH + 1;
/// [`ConstU32`] wrapper around [`MAX_SCOPE_PATH_LEN`] for use as a `BoundedVec` bound.
pub type MaxScopePathLen = ConstU32<MAX_SCOPE_PATH_LEN>;
/// [`ConstU32`] wrapper around [`MAX_CHILDREN_PER_NODE`] for use as a `BoundedVec` bound.
pub type MaxChildrenPerNode = ConstU32<MAX_CHILDREN_PER_NODE>;

/// Type alias for node metadata - bounded vector of bytes.
///
/// Stores small structured information as plain bytes up to
/// `MAX_META_SIZE` (1 KiB). For sensitive data, encryption should be
/// handled at the client level before storing in the pallet.
///
/// # Client-Side Encryption Recommendation
///
/// For encryption use cases, applications should:
/// 1. Encrypt sensitive data on the client side
/// 2. Store encrypted bytes in this BoundedVec
/// 3. Decrypt data after retrieving from chain
pub type NodeMeta = BoundedVec<u8, MaxMetaSize>;

/// Type alias for node payload - bounded vector of bytes.
///
/// Stores application data as plain bytes up to `MAX_PAYLOAD_SIZE` (8 KiB).
/// For sensitive data, encryption should be handled at the client level
/// before storing in the pallet.
///
/// # Client-Side Encryption Recommendation
///
/// For encryption use cases, applications should:
/// 1. Encrypt sensitive data on the client side
/// 2. Store encrypted bytes in this BoundedVec
/// 3. Decrypt data after retrieving from chain
pub type NodePayload = BoundedVec<u8, MaxPayloadSize>;

/// Node identifier newtype with compact encoding for efficient storage.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Debug,
)]
pub struct NodeId(#[codec(compact)] pub u64);

impl From<u64> for NodeId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<NodeId> for u64 {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

impl NodeId {
    /// Saturating add for node ID increments.
    ///
    /// Returns `NodeId(u64::MAX)` if addition would overflow instead of wrapping.
    pub fn saturating_add(self, rhs: u64) -> Self {
        Self(self.0.saturating_add(rhs))
    }
}

/// Scope identifier newtype with compact encoding for efficient storage.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Debug,
)]
pub struct ScopeId(#[codec(compact)] pub u64);

impl From<u64> for ScopeId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<ScopeId> for u64 {
    fn from(id: ScopeId) -> Self {
        id.0
    }
}

impl ScopeId {
    /// Checked add for Scope ID increments. Returns `None` on overflow so
    /// callers can reject the operation rather than ever reusing an ID.
    pub fn checked_add(self, rhs: u64) -> Option<Self> {
        self.0.checked_add(rhs).map(Self)
    }

    /// Saturating add for Scope ID increments, used by the storage
    /// migration where rejecting the upgrade on overflow is not an option.
    ///
    /// Returns `ScopeId(u64::MAX)` if addition would overflow instead of
    /// wrapping.
    pub fn saturating_add(self, rhs: u64) -> Self {
        Self(self.0.saturating_add(rhs))
    }
}

/// A delegable capability that [`Access`] can grant within a [`Scope`](self#scope).
///
/// Capability indices used by [`AccessFlags`]'s bit layout are assigned
/// explicitly by [`Capability::index`] and are independent of this enum's
/// SCALE discriminant - reordering variants here never changes on-chain
/// encoding.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
)]
pub enum Capability {
    /// Authority to call [`Pallet::create_scope`] on a node: either
    /// establishing a brand-new nested Scope boundary on a descendant, or
    /// replacing the existing Scope at the exact root that holds the
    /// grant. This is the sole mechanism for handing over control of a
    /// Scope. A [`GrantMode::Node`] grant only authorizes the exact
    /// granted node; a [`GrantMode::Subtree`] grant also authorizes every
    /// descendant within the same Scope, not just the node it was made
    /// at.
    #[codec(index = 0)]
    CreateScope,
    /// Authority to mutate a node's `Meta` / `Payload`.
    #[codec(index = 1)]
    Write,
}

impl Capability {
    /// Stable, explicit bit-pair index used by [`AccessFlags`]. Must never
    /// be derived from SCALE discriminants (which can shift when variants
    /// are reordered) and, once assigned, must never change or be reused.
    pub fn index(self) -> u32 {
        match self {
            Capability::CreateScope => 0,
            Capability::Write => 1,
        }
    }
}

/// How a granted [`Capability`] propagates through the node hierarchy.
///
/// `GrantMode` describes propagation of a particular grant, not the CPS
/// [`Scope`](self#scope) architectural concept, hence the distinct name.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Debug,
)]
pub enum GrantMode {
    /// The grant applies only to the exact `NodeId` it was made at.
    #[codec(index = 0)]
    Node,
    /// The grant applies to the node and all its descendants, as long as
    /// they resolve to the same Scope (a `Subtree` grant never crosses a
    /// CPS Scope boundary).
    #[codec(index = 1)]
    Subtree,
}

/// Compact, internal bitset storage representation for every [`Capability`]
/// delegated to one `(ScopeId, NodeId, AccountId)` triple.
///
/// Each capability occupies two adjacent bits, indexed by
/// [`Capability::index`]:
///
/// ```text
/// bit 2*n       capability is granted
/// bit 2*n + 1   grant propagates to descendants (GrantMode::Subtree)
/// ```
///
/// Valid states per capability: `00` (absent), `01` (`GrantMode::Node`),
/// `11` (`GrantMode::Subtree`). `10` is invalid and is never produced. A
/// `u128` provides `128 / 2 = 64` capability slots, more than sufficient for
/// the expected CPS permission model.
///
/// `AccessFlags` is an internal storage representation only: public APIs
/// operate exclusively in terms of [`Capability`] / [`GrantMode`], and
/// permission enumeration uses `(Capability, GrantMode)` tuples rather than
/// exposing this type, its underlying `u128`, or raw bit positions.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Debug,
)]
pub struct AccessFlags(#[codec(compact)] u128);

impl AccessFlags {
    fn granted_bit(capability: Capability) -> u128 {
        1u128 << (capability.index() * 2)
    }

    fn subtree_bit(capability: Capability) -> u128 {
        1u128 << (capability.index() * 2 + 1)
    }

    /// Grant `capability` with the given `mode`, replacing only that
    /// capability's previous mode (`absent -> Node`, `absent -> Subtree`,
    /// `Node -> Subtree`, `Subtree -> Node`); other capabilities are
    /// unaffected.
    fn grant(&mut self, capability: Capability, mode: GrantMode) {
        let granted = Self::granted_bit(capability);
        let subtree = Self::subtree_bit(capability);

        self.0 |= granted;

        match mode {
            GrantMode::Node => self.0 &= !subtree,
            GrantMode::Subtree => self.0 |= subtree,
        }
    }

    /// Revoke `capability`, clearing both of its bits while leaving every
    /// other capability's state untouched.
    fn revoke(&mut self, capability: Capability) {
        self.0 &= !(Self::granted_bit(capability) | Self::subtree_bit(capability));
    }

    /// Whether `capability` is granted at all (`Node` or `Subtree`).
    fn contains(&self, capability: Capability) -> bool {
        self.0 & Self::granted_bit(capability) != 0
    }

    /// Whether `capability` is granted with [`GrantMode::Subtree`], i.e.
    /// whether it applies to descendants of the node it was granted at.
    fn applies_to_descendants(&self, capability: Capability) -> bool {
        self.contains(capability) && self.0 & Self::subtree_bit(capability) != 0
    }

    /// Whether no capability at all is currently granted. An empty
    /// `AccessFlags` entry is never stored - see [`Pallet::revoke_access`].
    fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

/// Hot-path topology data for a single CPS node.
///
/// Presence of a `Nodes` entry means the node exists. `parent` is
/// **immutable** once a node is created - there is no way to change it.
/// `scope` is `Some(scope_id)` only on nodes that are the root of an active
/// [`Scope`](self#scope); a node with `scope: None` resolves to the Scope of
/// the nearest ancestor that has one (see [`Pallet::resolve_scope`]).
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq, Debug,
)]
pub struct NodeInfo {
    /// `None` for a root node, `Some(parent_id)` otherwise.
    pub parent: Option<NodeId>,
    /// `Some(scope_id)` only if this node is the root of an active Scope.
    pub scope: Option<ScopeId>,
}

/// Canonical, first-class record for a [`Scope`](self#scope): its owner and
/// scope-local accounting.
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq, Debug,
)]
pub struct ScopeInfo<AccountId: MaxEncodedLen> {
    /// The Scope's owner account. Immutable once the Scope is created;
    /// changing control means replacing the Scope (see
    /// [`Pallet::create_scope`]).
    pub owner: AccountId,
    /// Number of physical `Access` entries (distinct `(NodeId, AccountId)`
    /// pairs) currently stored under this Scope. Bounded by
    /// `MAX_ACCESS_ENTRIES_PER_SCOPE`; see
    /// [`Bounded Access and synchronous Scope cleanup`](self#bounded-access-and-synchronous-scope-cleanup).
    #[codec(compact)]
    pub access_count: u32,
}

/// The Scope resolved for a CPS node: its `ScopeId`, root `NodeId`, and
/// owner `AccountId`, produced by [`Pallet::resolve_scope`] in a single walk
/// so callers never need a separate lookup for the root or owner.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug)]
pub struct ResolvedScope<AccountId> {
    /// The resolved Scope's identifier.
    pub id: ScopeId,
    /// The `NodeId` at which this Scope's `NodeInfo.scope` entry is stored.
    pub root: NodeId,
    /// The Scope's owner account.
    pub owner: AccountId,
}

/// The result of a single canonical topology walk from a target node up to
/// its resolved Scope: the [`ResolvedScope`] itself, together with the full
/// path walked to reach it.
///
/// `path` holds the target node first and the Scope root last (so
/// `path.len() == 1` when the target node is itself the Scope root). Depth
/// is deliberately *not* cached anywhere - it is always derived as
/// `path.len() - 1`, since caching it would go stale whenever
/// [`Pallet::create_scope`] moves a descendant's effective depth by
/// inserting a new Scope boundary above it.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug)]
pub struct ResolvedScopePath<AccountId> {
    /// The Scope resolved for the target node.
    pub scope: ResolvedScope<AccountId>,
    /// The path from the target node (first) to the Scope root (last),
    /// inclusive of both ends.
    pub path: BoundedVec<NodeId, MaxScopePathLen>,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    /// Storage version for migrations
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

    /// Next node ID counter
    #[pallet::storage]
    #[pallet::getter(fn next_node_id)]
    pub type NextNodeId<T> = StorageValue<_, NodeId, ValueQuery>;

    /// Node existence, parent link, and current Scope pointer.
    ///
    /// Presence of a key means the node exists. `parent` is `None` for a
    /// root node and `Some(parent_id)` for a child - **immutable** once a
    /// node is created, there is no way to change it. `scope` is
    /// `Some(scope_id)` only on nodes that are the root of an active Scope.
    #[pallet::storage]
    #[pallet::getter(fn node_info)]
    pub type Nodes<T: Config> = StorageMap<_, Twox64Concat, NodeId, NodeInfo, OptionQuery>;

    /// Node metadata. An entry is present only when metadata has been set.
    #[pallet::storage]
    #[pallet::getter(fn meta_of)]
    pub type Meta<T: Config> = StorageMap<_, Twox64Concat, NodeId, NodeMeta>;

    /// Node payload. An entry is present only when a payload has been set.
    #[pallet::storage]
    #[pallet::getter(fn payload_of)]
    pub type Payload<T: Config> = StorageMap<_, Twox64Concat, NodeId, NodePayload>;

    /// Next Scope ID counter. `ScopeId` is globally unique and never reused.
    #[pallet::storage]
    #[pallet::getter(fn next_scope_id)]
    pub type NextScopeId<T> = StorageValue<_, ScopeId, ValueQuery>;

    /// Canonical Scope record: owner and scope-local accounting
    /// (`access_count`). An entry is present only for an active Scope,
    /// keyed by the `ScopeId` referenced by the matching `Nodes[root].scope`.
    ///
    /// `ScopeId` is an internally generated, never-reused counter, so it
    /// uses the cheaper reversible `Twox64Concat` hasher.
    #[pallet::storage]
    #[pallet::getter(fn scope_info)]
    pub type Scopes<T: Config> =
        StorageMap<_, Twox64Concat, ScopeId, ScopeInfo<T::AccountId>, OptionQuery>;

    /// Access delegations, scoped to a `ScopeId`. One compact [`AccessFlags`]
    /// entry per `(ScopeId, NodeId, AccountId)` bit-packs every delegated
    /// [`Capability`] and its [`GrantMode`]; an entry is never stored once
    /// empty (see [`Pallet::revoke_access`]).
    ///
    /// The outer key (`ScopeId`) is an internally generated, never-reused
    /// counter, so it uses the cheaper reversible `Twox64Concat` hasher;
    /// this also allows single-call prefix removal by `ScopeId` when a
    /// Scope generation is invalidated (see [`Pallet::clear_scope_access`]).
    /// The inner key mixes in the attacker-influenced `AccountId`, so it
    /// keeps the cryptographic `Blake2_128Concat` hasher.
    #[pallet::storage]
    #[pallet::getter(fn access)]
    pub type Access<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        ScopeId,
        Blake2_128Concat,
        (NodeId, T::AccountId),
        AccessFlags,
        ValueQuery,
    >;

    /// Index of children by parent node.
    #[pallet::storage]
    #[pallet::getter(fn children_of)]
    pub type Children<T: Config> =
        StorageMap<_, Twox64Concat, NodeId, BoundedVec<NodeId, MaxChildrenPerNode>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Node created [node_id, parent_id, creator]
        NodeCreated(NodeId, Option<NodeId>, T::AccountId),
        /// Node metadata set [node_id, sender]
        MetaSet(NodeId, T::AccountId),
        /// Node payload set [node_id, sender]
        PayloadSet(NodeId, T::AccountId),
        /// Node deleted [node_id, sender]
        NodeDeleted(NodeId, T::AccountId),
        /// A new Scope was created (or replaced an existing generation)
        /// [scope_id, root, owner]
        ScopeCreated(ScopeId, NodeId, T::AccountId),
        /// A Scope boundary was removed, together with its (leaf) node
        /// [scope_id, root]
        ScopeDeleted(ScopeId, NodeId),
        /// Access was granted [scope_id, node_id, principal, capability, mode]
        AccessGranted(ScopeId, NodeId, T::AccountId, Capability, GrantMode),
        /// Access was revoked [scope_id, node_id, principal, capability]
        AccessRevoked(ScopeId, NodeId, T::AccountId, Capability),
    }

    #[pallet::error]
    #[derive(PartialEq)]
    pub enum Error<T> {
        /// Node not found
        NodeNotFound,
        /// Parent node not found
        ParentNotFound,
        /// Maximum scope-local depth exceeded
        MaxScopeDepthExceeded,
        /// Too many children for node
        TooManyChildren,
        /// Node has children and cannot be deleted
        NodeHasChildren,
        /// No fresh node ID can be allocated without overflowing the counter
        NodeIdExhausted,
        /// No Scope could be resolved for this node
        ScopeNotFound,
        /// No fresh Scope ID can be allocated without overflowing the counter
        ScopeIdExhausted,
        /// Caller is not the owner of the resolved Scope
        NotScopeOwner,
        /// Caller does not hold the required Access for this operation
        AccessDenied,
        /// The Scope already holds `MAX_ACCESS_ENTRIES_PER_SCOPE` physical
        /// `Access` entries; no further `(NodeId, AccountId)` pair can be
        /// granted access until an existing entry is fully revoked.
        TooManyAccessEntries,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new node.
        ///
        /// Creating a root node (`parent_id: None`) allocates a fresh Scope
        /// owned by the caller. Creating a child node is a structural
        /// change, not a data mutation, so it requires the caller to be the
        /// owner of `parent_id`'s resolved Scope - `Write` (owner or
        /// delegated `Access`) only authorizes `Meta`/`Payload` mutation,
        /// never hierarchy changes. The child does not get its own Scope -
        /// it inherits `parent_id`'s resolved Scope implicitly (`scope:
        /// None`).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_node(
            meta.as_ref().map_or(0, |v| v.len() as u32),
            payload.as_ref().map_or(0, |v| v.len() as u32),
        ))]
        pub fn create_node(
            origin: OriginFor<T>,
            parent_id: Option<NodeId>,
            meta: Option<NodeMeta>,
            payload: Option<NodePayload>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Peek the next node ID (and, for a root node, the next Scope
            // ID) without committing either counter until every fallible
            // check below succeeds. Storage writes are not automatically
            // rolled back when a dispatchable returns an error, so
            // committing early would let a failed call consume an ID or
            // leave a dangling index entry behind.
            let node_id = <NextNodeId<T>>::get();
            let next_node_id = node_id
                .0
                .checked_add(1)
                .ok_or(Error::<T>::NodeIdExhausted)?;

            let new_scope = if parent_id.is_none() {
                let scope_id = <NextScopeId<T>>::get();
                let next_scope_id = scope_id
                    .checked_add(1)
                    .ok_or(Error::<T>::ScopeIdExhausted)?;
                Some((scope_id, next_scope_id))
            } else {
                None
            };

            if let Some(pid) = parent_id {
                ensure!(<Nodes<T>>::contains_key(pid), Error::<T>::ParentNotFound);

                // Creating a child node is a structural change to the CPS
                // hierarchy, not a data mutation - `Write` only authorizes
                // `Meta`/`Payload` changes (see issue #656), so this always
                // requires the resolved Scope's owner authority.
                let resolved = Self::resolve_scope_path(pid)?;
                ensure!(resolved.scope.owner == sender, Error::<T>::NotScopeOwner);

                // Check the scope-local depth by reusing the parent's
                // already-resolved path: the new child's scope-local depth
                // equals the parent's path length (`path.len() ==
                // parent_depth + 1`), so admitting one more child is only
                // safe while that length is still within `MAX_SCOPE_DEPTH`.
                ensure!(
                    resolved.path.len() as u32 <= MAX_SCOPE_DEPTH,
                    Error::<T>::MaxScopeDepthExceeded
                );

                // Add to parent's children index
                <Children<T>>::try_mutate(pid, |children| {
                    children
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyChildren)
                })?;
            }
            // Root nodes (parent_id.is_none()) always allocate a fresh,
            // caller-owned Scope, handled below.

            // All fallible checks passed: commit the reserved node/Scope
            // IDs and the node's attributes.
            <NextNodeId<T>>::put(NodeId(next_node_id));

            let scope = if let Some((scope_id, next_scope_id)) = new_scope {
                <NextScopeId<T>>::put(next_scope_id);
                <Scopes<T>>::insert(
                    scope_id,
                    ScopeInfo {
                        owner: sender.clone(),
                        access_count: 0,
                    },
                );
                Self::deposit_event(Event::ScopeCreated(scope_id, node_id, sender.clone()));
                Some(scope_id)
            } else {
                None
            };

            // Store the node's attributes
            <Nodes<T>>::insert(
                node_id,
                NodeInfo {
                    parent: parent_id,
                    scope,
                },
            );
            if let Some(meta) = meta {
                <Meta<T>>::insert(node_id, meta);
            }
            if let Some(payload) = payload {
                <Payload<T>>::insert(node_id, payload);
            }

            Self::deposit_event(Event::NodeCreated(node_id, parent_id, sender));
            Ok(())
        }

        /// Set node metadata
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_meta(meta.as_ref().map_or(0, |v| v.len() as u32)))]
        pub fn set_meta(
            origin: OriginFor<T>,
            node_id: NodeId,
            meta: Option<NodeMeta>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Self::authorize(node_id, &sender, Capability::Write)?;

            match meta {
                Some(meta) => <Meta<T>>::insert(node_id, meta),
                None => <Meta<T>>::remove(node_id),
            }

            Self::deposit_event(Event::MetaSet(node_id, sender));
            Ok(())
        }

        /// Set node payload
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_payload(payload.as_ref().map_or(0, |v| v.len() as u32)))]
        pub fn set_payload(
            origin: OriginFor<T>,
            node_id: NodeId,
            payload: Option<NodePayload>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Self::authorize(node_id, &sender, Capability::Write)?;

            match payload {
                Some(payload) => <Payload<T>>::insert(node_id, payload),
                None => <Payload<T>>::remove(node_id),
            }

            Self::deposit_event(Event::PayloadSet(node_id, sender));

            Ok(())
        }

        /// Delete a node.
        ///
        /// Only leaf nodes (no children) can be deleted. If the node is the
        /// root of an active Scope, that Scope's `NodeInfo.scope` boundary
        /// disappears together with the node - this is the *only* way a
        /// Scope boundary is ever removed (there is no standalone
        /// `delete_scope`); the rest of the Scope's physical state
        /// (`Access`, `Scopes`) is synchronously cleared in this same call
        /// (see `Pallet::clear_scope_access`).
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::delete_node())]
        pub fn delete_node(origin: OriginFor<T>, node_id: NodeId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Check the node exists, and get its topology info.
            let info = <Nodes<T>>::get(node_id).ok_or(Error::<T>::NodeNotFound)?;

            // Only owner can remove node
            let resolved = Self::resolve_scope(node_id)?;
            ensure!(resolved.owner == sender, Error::<T>::NotScopeOwner);

            // Check if node has children
            let children = <Children<T>>::get(node_id);
            ensure!(children.is_empty(), Error::<T>::NodeHasChildren);

            // Remove from parent's children index
            if let Some(parent_id) = info.parent {
                <Children<T>>::mutate(parent_id, |children| {
                    children.retain(|&id| id != node_id);
                });
            }

            // Remove the node's own children index entry
            <Children<T>>::remove(node_id);

            // Remove the Scope boundary attached to this node, if any. The
            // rest of that Scope's physical state is synchronously cleared
            // now, in the same call.
            if let Some(stale_scope) = info.scope {
                Self::clear_scope_access(stale_scope);
                <Scopes<T>>::remove(stale_scope);
                Self::deposit_event(Event::ScopeDeleted(stale_scope, node_id));
            }

            // Remove the node's attributes
            <Meta<T>>::remove(node_id);
            <Payload<T>>::remove(node_id);
            <Nodes<T>>::remove(node_id);

            Self::deposit_event(Event::NodeDeleted(node_id, sender));
            Ok(())
        }

        /// Create a new Scope rooted at `node_id`, or replace its existing
        /// generation.
        ///
        /// The caller becomes the new Scope's owner and a fresh `ScopeId` is
        /// always allocated. This is a single operation with two cases,
        /// distinguished by whether `node_id` already roots an active
        /// Scope:
        ///
        /// - **Ordinary node** (`Nodes[node_id].scope == None`): establishes
        ///   a brand-new nested Scope boundary there. Authorized either by
        ///   owning the Scope currently governing `node_id`, or by holding a
        ///   delegated `Capability::CreateScope` grant reaching `node_id` -
        ///   a `GrantMode::Node` grant at the exact `node_id`, or a
        ///   `GrantMode::Subtree` grant at `node_id` or a strict ancestor
        ///   within the same Scope.
        /// - **Existing Scope root** (`Nodes[node_id].scope ==
        ///   Some(old_scope_id)`): replaces that Scope's generation in
        ///   place - the sole ownership-transfer mechanism. Authorized by
        ///   the old Scope's owner, or by a `CreateScope` grant reaching
        ///   `node_id` *within that old Scope*. The boundary stays on the
        ///   same node; only the Scope generation changes. The previous
        ///   generation's `Access` entries are synchronously invalidated
        ///   (see `Pallet::clear_scope_access`).
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::create_scope())]
        pub fn create_scope(origin: OriginFor<T>, node_id: NodeId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let info = <Nodes<T>>::get(node_id).ok_or(Error::<T>::NodeNotFound)?;

            Self::authorize(node_id, &sender, Capability::CreateScope)?;

            let scope_id = Self::allocate_scope(node_id, info.scope, sender.clone())?;

            Self::deposit_event(Event::ScopeCreated(scope_id, node_id, sender));
            Ok(())
        }

        /// Grant `capability` to `principal` at `node_id`, within the Scope
        /// resolved for `node_id`. Only the Scope's owner may grant Access.
        ///
        /// A brand-new `(node_id, principal)` `Access` entry consumes one of
        /// the Scope's bounded `MAX_ACCESS_ENTRIES_PER_SCOPE` slots (rejected
        /// with [`Error::TooManyAccessEntries`] once the bound is reached);
        /// changing the `GrantMode` or adding another `Capability` bit to an
        /// already-existing entry never does.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::grant_access())]
        pub fn grant_access(
            origin: OriginFor<T>,
            node_id: NodeId,
            principal: T::AccountId,
            capability: Capability,
            mode: GrantMode,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let resolved = Self::resolve_scope(node_id)?;
            ensure!(resolved.owner == sender, Error::<T>::NotScopeOwner);

            let key = (node_id, principal.clone());
            let is_new_entry = !<Access<T>>::contains_key(resolved.id, &key);
            if is_new_entry {
                let scope_info = <Scopes<T>>::get(resolved.id).ok_or(Error::<T>::ScopeNotFound)?;
                ensure!(
                    scope_info.access_count < MAX_ACCESS_ENTRIES_PER_SCOPE,
                    Error::<T>::TooManyAccessEntries
                );
            }

            <Access<T>>::mutate(resolved.id, &key, |flags| {
                flags.grant(capability, mode);
            });

            if is_new_entry {
                <Scopes<T>>::mutate(resolved.id, |maybe_info| {
                    if let Some(scope_info) = maybe_info {
                        scope_info.access_count = scope_info.access_count.saturating_add(1);
                    }
                });
            }

            Self::deposit_event(Event::AccessGranted(
                resolved.id,
                node_id,
                principal,
                capability,
                mode,
            ));
            Ok(())
        }

        /// Revoke a previously granted `capability` from `principal` at
        /// `node_id`. Only the Scope's owner may revoke Access.
        ///
        /// Only fully revoking the last remaining `Capability` on a
        /// `(node_id, principal)` entry (removing it entirely) frees up
        /// that Scope's `MAX_ACCESS_ENTRIES_PER_SCOPE` slot; revoking one
        /// capability while others remain leaves the entry - and the count
        /// - untouched.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::revoke_access())]
        pub fn revoke_access(
            origin: OriginFor<T>,
            node_id: NodeId,
            principal: T::AccountId,
            capability: Capability,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let resolved = Self::resolve_scope(node_id)?;
            ensure!(resolved.owner == sender, Error::<T>::NotScopeOwner);

            let key = (node_id, principal.clone());
            let existed = <Access<T>>::contains_key(resolved.id, &key);
            let mut flags = <Access<T>>::get(resolved.id, &key);
            flags.revoke(capability);
            if flags.is_empty() {
                if existed {
                    <Access<T>>::remove(resolved.id, &key);
                    <Scopes<T>>::mutate(resolved.id, |maybe_info| {
                        if let Some(scope_info) = maybe_info {
                            if let Some(next_count) = scope_info.access_count.checked_sub(1) {
                                scope_info.access_count = next_count;
                            } else {
                                frame_support::defensive!(
                                    "CPS: access_count underflow while revoking Access entry"
                                );
                                scope_info.access_count = 0;
                            }
                        }
                    });
                }
            } else {
                <Access<T>>::insert(resolved.id, &key, flags);
            }

            Self::deposit_event(Event::AccessRevoked(
                resolved.id,
                node_id,
                principal,
                capability,
            ));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Resolve the [`ResolvedScope`] effective for `node_id`.
        ///
        /// Thin wrapper over [`Self::resolve_scope_path`] returning just the
        /// Scope, for callers that don't need the full walked path (e.g. the
        /// `CpsApi` runtime API).
        pub fn resolve_scope(node_id: NodeId) -> Result<ResolvedScope<T::AccountId>, Error<T>> {
            Self::resolve_scope_path(node_id).map(|resolved| resolved.scope)
        }

        /// Check whether `account_id` currently holds `capability` at
        /// `node_id`.
        ///
        /// This is a read-only query reusing the exact same [`Self::authorize`]
        /// logic enforced by the dispatchables, so it never drifts from the
        /// on-chain behavior. Returns `false` rather than an error if
        /// `node_id` does not exist or no Scope can be resolved for it.
        ///
        /// The `CpsApi` runtime API (`pallet-robonomics-cps-runtime-api`)
        /// passes `Capability` directly across the API boundary. Its SCALE
        /// encoding is pinned by explicit `#[codec(index = ..)]` attributes
        /// on each variant (see [`Capability`]), so new capabilities may be
        /// added anywhere without disturbing the encoding of existing
        /// callers - only never reusing or reassigning an already-shipped
        /// index matters.
        pub fn has_capability(
            node_id: NodeId,
            account_id: &T::AccountId,
            capability: Capability,
        ) -> bool {
            Self::authorize(node_id, account_id, capability).unwrap_or(false)
        }

        /// The single canonical topology resolver: walk from `node_id`
        /// towards the root, one hop at a time, until a `NodeInfo.scope`
        /// entry is found, collecting every node visited along the way.
        ///
        /// Returns the resolved [`ResolvedScope`] together with the walked
        /// `path` (target node first, Scope root last) as a
        /// [`ResolvedScopePath`]. Every authorization check and depth
        /// admission in this pallet is built on top of this single walk -
        /// there is no separate traversal for depth vs. Access lookups.
        ///
        /// The walk never visits more than `MAX_SCOPE_PATH_LEN` nodes: this
        /// bounds resolution cost to the *scope-local* depth, never the
        /// (unbounded) global tree depth. Exceeding the bound without
        /// finding a Scope indicates a broken invariant (every node must
        /// resolve to a Scope within `MAX_SCOPE_DEPTH`) and is reported as
        /// [`Error::ScopeNotFound`].
        pub fn resolve_scope_path(
            node_id: NodeId,
        ) -> Result<ResolvedScopePath<T::AccountId>, Error<T>> {
            let mut path: BoundedVec<NodeId, MaxScopePathLen> = BoundedVec::default();
            let mut current = node_id;

            loop {
                path.try_push(current)
                    .map_err(|_| Error::<T>::ScopeNotFound)?;

                let info = <Nodes<T>>::get(current).ok_or(Error::<T>::NodeNotFound)?;

                if let Some(scope_id) = info.scope {
                    let scope_info = <Scopes<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;
                    return Ok(ResolvedScopePath {
                        scope: ResolvedScope {
                            id: scope_id,
                            root: current,
                            owner: scope_info.owner,
                        },
                        path,
                    });
                }

                current = info.parent.ok_or(Error::<T>::ScopeNotFound)?;
            }
        }

        /// Authorize `sender` to exercise `capability` at `node_id`.
        ///
        /// The Scope owner always has implicit authority. Otherwise, this
        /// resolves `node_id`'s Scope path exactly once (via
        /// [`Self::resolve_scope_path`]) and reuses it to check `Access`
        /// entries without a second topology traversal: at `node_id` itself
        /// (`path[0]`), both `GrantMode::Node` and `GrantMode::Subtree`
        /// grants authorize; on every strict ancestor in the path, only
        /// `GrantMode::Subtree` grants do. The walk never crosses the Scope
        /// boundary, since `path` never extends past the resolved Scope's
        /// root.
        pub fn authorize(
            node_id: NodeId,
            sender: &T::AccountId,
            capability: Capability,
        ) -> Result<bool, Error<T>> {
            let resolved = Self::resolve_scope_path(node_id)?;
            if resolved.scope.owner == *sender {
                return Ok(true);
            }

            for (index, current) in resolved.path.iter().enumerate() {
                let flags = <Access<T>>::get(resolved.scope.id, (*current, sender.clone()));
                if index == 0 {
                    if flags.contains(capability) {
                        return Ok(true);
                    }
                } else if flags.applies_to_descendants(capability) {
                    return Ok(true);
                }
            }

            Err(Error::<T>::AccessDenied)
        }

        /// Allocate a fresh `ScopeId` rooted at `root` and owned by `owner`,
        /// and activate it. `old_scope` is `root`'s current
        /// `NodeInfo.scope` value: if `Some`, that generation is replaced -
        /// its remaining physical state (`Access`, `ScopeInfo`) is
        /// synchronously cleared (see [`Self::clear_scope_access`]) as part
        /// of the same call that installs the new one.
        fn allocate_scope(
            root: NodeId,
            old_scope: Option<ScopeId>,
            owner: T::AccountId,
        ) -> Result<ScopeId, Error<T>> {
            let scope_id = <NextScopeId<T>>::get();
            let next_id = scope_id
                .checked_add(1)
                .ok_or(Error::<T>::ScopeIdExhausted)?;
            <NextScopeId<T>>::put(next_id);

            if let Some(stale_scope) = old_scope {
                Self::clear_scope_access(stale_scope);
                <Scopes<T>>::remove(stale_scope);
            }

            <Scopes<T>>::insert(
                scope_id,
                ScopeInfo {
                    owner,
                    access_count: 0,
                },
            );
            <Nodes<T>>::mutate(root, |maybe_info| {
                if let Some(info) = maybe_info {
                    info.scope = Some(scope_id);
                }
            });

            Ok(scope_id)
        }

        /// Synchronously delete every `Access` entry belonging to an
        /// invalidated `scope_id`.
        ///
        /// Must be called in the same state transition that removes or
        /// replaces the corresponding `NodeInfo.scope` entry (`ScopeId`s are
        /// never reused, so an invalidated Scope can never become active
        /// again). Callers are responsible for removing the `Scopes[scope_id]`
        /// entry itself once this returns.
        ///
        /// Because [`MAX_ACCESS_ENTRIES_PER_SCOPE`] bounds the number of
        /// physical `Access` entries any Scope can ever hold, a single
        /// `clear_prefix` call with that same bound as its removal limit is
        /// always sufficient to remove the entire prefix in one step - there is
        /// no deferred/background continuation. `clear_prefix` reporting a
        /// non-empty continuation cursor despite the enforced bound would mean
        /// the `access_count <= MAX_ACCESS_ENTRIES_PER_SCOPE` invariant was
        /// violated elsewhere; this is treated as a bug via `defensive!` rather
        /// than silently leaving stale entries behind or enqueuing further
        /// work.
        pub(crate) fn clear_scope_access(scope_id: ScopeId) {
            let result = <Access<T>>::clear_prefix(scope_id, MAX_ACCESS_ENTRIES_PER_SCOPE, None);
            if result.maybe_cursor.is_some() {
                frame_support::defensive!(
                    "CPS: clear_prefix left Access entries behind despite \
                     MAX_ACCESS_ENTRIES_PER_SCOPE bound",
                    scope_id
                );
            }
        }
    }
}
