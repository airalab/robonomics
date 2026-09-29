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
//! # CPS Pallet
//!
//! On-chain registry of cyber-physical systems, stored as a tree of nodes.
//!
//! Each node has a fixed parent link and may carry opaque `Meta` and
//! `Payload` bytes. Authority over nodes is organized in *Scopes*: a Scope
//! is rooted at a node, has one owner account, and covers its root and all
//! descendants down to the next nested Scope root. A Scope owner may
//! delegate a [`Capability`] on a node or subtree to other accounts through
//! *Access* entries.
//!
//! ## Nodes
//!
//! - [`NodeId`]s are allocated sequentially from [`NextNodeId`] and never
//!   reused.
//! - A node's parent is set by [`Pallet::create_node`] and never changes;
//!   there is no call to move a node. To relocate an object, create a new
//!   node under the new parent and delete the old one.
//! - A node has at most [`MAX_CHILDREN_PER_NODE`] direct children, indexed
//!   in [`Children`].
//! - Only a node without children can be deleted.
//! - [`Meta`] (up to [`MAX_META_SIZE`] bytes) and [`Payload`] (up to
//!   [`MAX_PAYLOAD_SIZE`] bytes) are stored as given; the pallet does not
//!   interpret or encrypt them. A missing entry means the field is unset.
//!
//! ## Scopes
//!
//! Every node resolves to exactly one Scope: the Scope rooted at the node
//! itself, otherwise the Scope rooted at its nearest ancestor that roots
//! one. [`Pallet::resolve_scope`] performs this walk; every authorization
//! check starts with it.
//!
//! ```text
//! City                 root of Scope #1, owner A
//! `-- Smart Building   root of Scope #7, owner B
//!     `-- Floor 3      resolves to Scope #7
//! ```
//!
//! - A root node (created with `parent_id: None`) always roots a new Scope
//!   owned by its creator.
//! - [`Pallet::create_scope`] on any other node makes it the root of a new
//!   nested Scope owned by the caller.
//! - A nested Scope is a boundary: the owner and the Access entries of an
//!   enclosing Scope have no effect on nodes inside it, even when both
//!   Scopes have the same owner. The one exception is that the owner of the
//!   immediately enclosing Scope may replace the nested Scope (see
//!   [Replacing a Scope](crate#replacing-a-scope)).
//! - [`ScopeId`]s are allocated sequentially from [`NextScopeId`] and never
//!   reused. A Scope's owner is stored in [`Scopes`] and never changes.
//!
//! ### Scope-local depth
//!
//! The path from a node up to its Scope root, both ends included, holds at
//! most [`MAX_SCOPE_DEPTH`] nodes; a Scope root has depth 1.
//! [`Pallet::create_node`] rejects a child that would exceed this bound with
//! [`Error::MaxScopeDepthExceeded`]. A nested Scope root starts a new count,
//! so the depth of the whole tree is not bounded, while
//! [`Pallet::resolve_scope`] never visits more than [`MAX_SCOPE_DEPTH`]
//! nodes.
//!
//! ### Replacing a Scope
//!
//! [`Pallet::create_scope`] on a node that already roots a Scope allocates
//! a new `ScopeId` for the same root node, owned by the caller. Nodes that
//! resolved to the old Scope now resolve to the new one. The old Scope's
//! [`Scopes`] record and all of its [`Access`] entries are deleted in the
//! same call. This is the only way to change the owner of a Scope.
//!
//! The replacement may be made by:
//!
//! - the owner of the old Scope;
//! - an account holding [`Capability::CreateScope`] at the root node in the
//!   old Scope;
//! - the owner of the Scope that the root's parent resolves to (the
//!   immediately enclosing Scope). Owners of Scopes further up the tree and
//!   Access holders of the enclosing Scope are not authorized.
//!
//! ### Removing a Scope
//!
//! A Scope is removed only together with its root node:
//! [`Pallet::delete_node`] on a Scope root without children deletes the
//! node, its Scope's [`Scopes`] record, and all of the Scope's [`Access`]
//! entries. A node with children cannot be deleted, so a Scope root is
//! never removed while nodes below it exist.
//!
//! ## Access
//!
//! The Scope owner is authorized for every capability on every node of the
//! Scope without any Access entry. Other accounts need an [`Access`] entry,
//! which only the Scope owner can create ([`Pallet::grant_access`]) or
//! change ([`Pallet::revoke_access`]). An entry is keyed by
//! `(ScopeId, (NodeId, AccountId))` and holds a [`GrantMode`] for each
//! granted [`Capability`]:
//!
//! - [`GrantMode::Node`] applies to the granted node only;
//! - [`GrantMode::Subtree`] applies to the granted node and to its
//!   descendants that resolve to the same Scope.
//!
//! [`Pallet::authorize`] allows `account` to use `capability` at `node` if:
//!
//! 1. `account` owns the Scope that `node` resolves to; or
//! 2. `account` holds `capability` at `node` in that Scope, in either mode;
//!    or
//! 3. `account` holds `capability` with [`GrantMode::Subtree`] at an
//!    ancestor of `node` on the path to the Scope root.
//!
//! [`Capability::Write`] allows [`Pallet::set_meta`] and
//! [`Pallet::set_payload`]; [`Capability::CreateScope`] allows
//! [`Pallet::create_scope`]. Adding a child node, deleting a node, and
//! granting or revoking Access require the owner of the resolved Scope and
//! cannot be delegated.
//!
//! ### Access entry limit
//!
//! A Scope holds at most [`MAX_ACCESS_ENTRIES_PER_SCOPE`] Access entries
//! (distinct `(NodeId, AccountId)` pairs), counted in
//! [`ScopeInfo::access_count`]. A grant to a new pair takes a slot and fails
//! with [`Error::TooManyAccessEntries`] when none is left; adding a
//! capability to an existing entry or changing its mode does not. Revoking
//! the last capability of an entry deletes the entry and frees its slot.
//!
//! Since the count is bounded, replacing or removing a Scope deletes all of
//! its entries with a single `clear_prefix` call limited to `access_count`.
//! [`Pallet::create_scope`] and [`Pallet::delete_node`] are charged for
//! [`MAX_ACCESS_ENTRIES_PER_SCOPE`] entries and refund the difference to
//! the number actually deleted.
//!
//! An entry is stored under the `ScopeId` its node resolved to when it was
//! granted, while [`Pallet::revoke_access`] looks in the Scope the node
//! resolves to at call time. An entry at a node that was later deleted, or
//! that has since become part of a nested Scope, therefore no longer
//! authorizes anything and can no longer be revoked; it keeps counting
//! toward `access_count` until its Scope is replaced or removed. The Scope
//! owner can free these slots by replacing the Scope with
//! [`Pallet::create_scope`], which deletes all of the Scope's entries.
//!
//! ## Storage
//!
//! | Item            | Key                                | Value                                              |
//! |-----------------|------------------------------------|----------------------------------------------------|
//! | [`NextNodeId`]  | -                                  | next [`NodeId`] to allocate                        |
//! | [`Nodes`]       | [`NodeId`]                         | [`NodeInfo`]: parent, and Scope if the node roots one |
//! | [`Meta`]        | [`NodeId`]                         | [`NodeMeta`]                                       |
//! | [`Payload`]     | [`NodeId`]                         | [`NodePayload`]                                    |
//! | [`Children`]    | [`NodeId`]                         | direct children, in creation order                 |
//! | [`NextScopeId`] | -                                  | next [`ScopeId`] to allocate                       |
//! | [`Scopes`]      | [`ScopeId`]                        | [`ScopeInfo`]: owner and Access entry count        |
//! | [`Access`]      | [`ScopeId`], `(NodeId, AccountId)` | [`AccessFlags`]                                    |
//!
//! ## Dispatchable functions
//!
//! | Call                                                                  | Allowed caller                                                           |
//! |-----------------------------------------------------------------------|--------------------------------------------------------------------------|
//! | [`create_node`](Pallet::create_node) with `parent_id: None`           | any signed account                                                       |
//! | [`create_node`](Pallet::create_node) with `parent_id: Some(_)`        | owner of the parent's Scope                                              |
//! | [`set_meta`](Pallet::set_meta), [`set_payload`](Pallet::set_payload)  | authorized for [`Capability::Write`]                                     |
//! | [`delete_node`](Pallet::delete_node)                                  | owner of the node's Scope                                                |
//! | [`create_scope`](Pallet::create_scope)                                | authorized for [`Capability::CreateScope`]; for a nested Scope root, also the owner of the enclosing Scope |
//! | [`grant_access`](Pallet::grant_access), [`revoke_access`](Pallet::revoke_access) | owner of the node's Scope                                     |
//!
//! ## Public functions
//!
//! - [`Pallet::resolve_scope`]: Scope id, root, owner, and path of a node.
//! - [`Pallet::authorize`]: the capability check of [`Pallet::set_meta`],
//!   [`Pallet::set_payload`], and [`Pallet::create_scope`].
//! - [`Pallet::has_capability`]: whether an account may use a capability at
//!   a node, with the same checks as the calls that require it, including
//!   the enclosing Scope owner's right to use [`Pallet::create_scope`].
//!
//! `resolve_scope` and `has_capability` are exposed to off-chain clients by
//! the `CpsApi` runtime API (`pallet-robonomics-cps-runtime-api`).
//!
//! ## Invariants
//!
//! 1. A node's parent never changes and was created before it, so the tree
//!    has no cycles.
//! 2. `Children[p]` contains `c` exactly when `Nodes[c].parent == Some(p)`.
//! 3. Every node resolves to exactly one Scope within [`MAX_SCOPE_DEPTH`]
//!    nodes.
//! 4. `Scopes[s]` exists exactly when one node has `Nodes[_].scope ==
//!    Some(s)`.
//! 5. `Scopes[s].access_count` equals the number of `Access` entries under
//!    `s` and never exceeds [`MAX_ACCESS_ENTRIES_PER_SCOPE`].
//! 6. [`NodeId`]s and [`ScopeId`]s are never reused.
//!
//! ## Example
//!
//! ```ignore
//! use pallet_robonomics_cps::{Capability, Children, GrantMode, NodeId};
//!
//! // Account A creates a root node and becomes the owner of Scope #0.
//! Cps::create_node(RuntimeOrigin::signed(a), None, Some(meta), None)?;
//! let building = NodeId(0);
//!
//! // Only the Scope owner can add children.
//! Cps::create_node(RuntimeOrigin::signed(a), Some(building), None, None)?;
//! let floor = NodeId(1);
//!
//! // A makes `floor` the root of nested Scope #1 and hands it over to B:
//! // B, holding `CreateScope` at the Scope root, replaces Scope #1 with
//! // Scope #2 owned by B. Scope #1 and its Access entries are deleted.
//! Cps::create_scope(RuntimeOrigin::signed(a), floor)?;
//! Cps::grant_access(RuntimeOrigin::signed(a), floor, b, Capability::CreateScope, GrantMode::Node)?;
//! Cps::create_scope(RuntimeOrigin::signed(b), floor)?;
//!
//! // B adds a sensor and lets account C update its payload.
//! Cps::create_node(RuntimeOrigin::signed(b), Some(floor), None, None)?;
//! let sensor = NodeId(2);
//! Cps::grant_access(RuntimeOrigin::signed(b), sensor, c, Capability::Write, GrantMode::Node)?;
//! Cps::set_payload(RuntimeOrigin::signed(c), sensor, Some(payload))?;
//!
//! // Queries.
//! let scope = Cps::resolve_scope(sensor)?; // Scope #2, root `floor`, owner B
//! let children = Children::<Runtime>::get(building); // [floor]
//! ```
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

/// Maximum size of [`NodeMeta`] in bytes (1 KiB).
///
/// Metadata is intended for small descriptive data such as a device
/// configuration; larger objects can be stored off-chain and referenced by
/// hash or CID.
pub const MAX_META_SIZE: u32 = 1024;

/// Maximum size of [`NodePayload`] in bytes (8 KiB).
pub const MAX_PAYLOAD_SIZE: u32 = 8192;

/// Maximum scope-local depth: the number of nodes on the path from a node up
/// to its Scope root, both ends included.
///
/// A Scope root has depth 1, so one Scope holds chains of at most
/// `MAX_SCOPE_DEPTH` nodes. [`Pallet::create_node`] enforces it using the
/// length of the parent's resolved path, and [`Pallet::resolve_scope`] never
/// visits more nodes than this. The depth of the whole tree, across nested
/// Scopes, is not limited.
pub const MAX_SCOPE_DEPTH: u32 = 32;

/// Maximum number of direct children of a node, i.e. the bound of a
/// [`Children`] entry.
pub const MAX_CHILDREN_PER_NODE: u32 = 100;

/// Maximum number of [`Access`] entries (distinct `(NodeId, AccountId)`
/// pairs) stored under one `ScopeId`.
///
/// [`ScopeInfo::access_count`] never exceeds this value, which bounds the
/// cost of deleting all Access entries of a Scope when it is replaced or
/// removed.
pub const MAX_ACCESS_ENTRIES_PER_SCOPE: u32 = 32;

/// [`ConstU32`] wrapper around [`MAX_META_SIZE`] for use as a `BoundedVec` bound.
pub type MaxMetaSize = ConstU32<MAX_META_SIZE>;
/// [`ConstU32`] wrapper around [`MAX_PAYLOAD_SIZE`] for use as a `BoundedVec` bound.
pub type MaxPayloadSize = ConstU32<MAX_PAYLOAD_SIZE>;
/// [`ConstU32`] wrapper around [`MAX_SCOPE_DEPTH`] for use as a `BoundedVec`
/// bound (e.g. [`ResolvedScope::path`]).
pub type MaxScopeDepth = ConstU32<MAX_SCOPE_DEPTH>;
/// [`ConstU32`] wrapper around [`MAX_CHILDREN_PER_NODE`] for use as a `BoundedVec` bound.
pub type MaxChildrenPerNode = ConstU32<MAX_CHILDREN_PER_NODE>;

/// Node metadata: up to [`MAX_META_SIZE`] opaque bytes.
///
/// Stored as given and publicly readable; data that must stay confidential
/// has to be encrypted by the client before submission.
pub type NodeMeta = BoundedVec<u8, MaxMetaSize>;

/// Node payload: up to [`MAX_PAYLOAD_SIZE`] opaque bytes.
///
/// Stored as given and publicly readable; data that must stay confidential
/// has to be encrypted by the client before submission.
pub type NodePayload = BoundedVec<u8, MaxPayloadSize>;

/// Node identifier. SCALE-encoded in compact form.
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

/// Scope identifier. SCALE-encoded in compact form.
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
    /// `self + rhs`, or `None` on overflow. Used to allocate Scope IDs, so
    /// that an exhausted counter fails the call instead of reusing an ID.
    pub fn checked_add(self, rhs: u64) -> Option<Self> {
        self.0.checked_add(rhs).map(Self)
    }

    /// `self + rhs`, saturating at `ScopeId(u64::MAX)`. Used by the storage
    /// migration, which cannot fail.
    pub fn saturating_add(self, rhs: u64) -> Self {
        Self(self.0.saturating_add(rhs))
    }
}

/// A permission that a Scope owner can delegate with [`Pallet::grant_access`].
///
/// Each variant has a fixed SCALE index (`#[codec(index = ..)]`) and a fixed
/// [`AccessFlags`] bit index ([`Capability::index`]), so reordering or
/// adding variants does not change the encoding used in calls, events,
/// storage, and the runtime API.
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
    /// Allows [`Pallet::create_scope`] on a node.
    ///
    /// On an ordinary node the call makes it the root of a new nested
    /// Scope; on a Scope root it replaces the Scope with a new one owned by
    /// the caller. A [`GrantMode::Node`] grant covers only the granted node;
    /// a [`GrantMode::Subtree`] grant also covers its descendants in the
    /// same Scope.
    #[codec(index = 0)]
    CreateScope,
    /// Allows [`Pallet::set_meta`] and [`Pallet::set_payload`] on a node.
    #[codec(index = 1)]
    Write,
}

impl Capability {
    /// Bit-pair index of this capability in [`AccessFlags`].
    ///
    /// Assigned explicitly, independent of the SCALE variant index. An
    /// assigned index must never change or be reused, since it is part of
    /// the stored [`Access`] values.
    pub fn index(self) -> u32 {
        match self {
            Capability::CreateScope => 0,
            Capability::Write => 1,
        }
    }
}

/// Reach of a granted [`Capability`] from the node it was granted at.
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
    /// Applies to the granted node only.
    #[codec(index = 0)]
    Node,
    /// Applies to the granted node and its descendants that resolve to the
    /// same Scope; it does not reach into nested Scopes.
    #[codec(index = 1)]
    Subtree,
}

/// Capabilities granted to one account at one node within one Scope: the
/// value of an [`Access`] entry.
///
/// Each capability occupies two adjacent bits, starting at
/// `2 * Capability::index()`:
///
/// ```text
/// bit 2*n       capability is granted
/// bit 2*n + 1   grant applies to descendants (GrantMode::Subtree)
/// ```
///
/// Per capability, `00` means not granted, `01` [`GrantMode::Node`], and `11`
/// [`GrantMode::Subtree`]; `10` is never written. The `u128` holds up to 64
/// capabilities. The inner value and bit operations are private to the
/// pallet; calls, events, and the runtime API use [`Capability`] and
/// [`GrantMode`].
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

    /// Set `capability` with `mode`, replacing its previous mode if it was
    /// already granted. Other capabilities are unchanged.
    fn grant(&mut self, capability: Capability, mode: GrantMode) {
        let granted = Self::granted_bit(capability);
        let subtree = Self::subtree_bit(capability);

        self.0 |= granted;

        match mode {
            GrantMode::Node => self.0 &= !subtree,
            GrantMode::Subtree => self.0 |= subtree,
        }
    }

    /// Clear both bits of `capability`. Other capabilities are unchanged.
    fn revoke(&mut self, capability: Capability) {
        self.0 &= !(Self::granted_bit(capability) | Self::subtree_bit(capability));
    }

    /// Whether `capability` is granted in either mode.
    fn contains(&self, capability: Capability) -> bool {
        self.0 & Self::granted_bit(capability) != 0
    }

    /// Whether `capability` is granted with [`GrantMode::Subtree`].
    fn applies_to_descendants(&self, capability: Capability) -> bool {
        self.contains(capability) && self.0 & Self::subtree_bit(capability) != 0
    }

    /// Whether no capability is granted. [`Pallet::revoke_access`] deletes
    /// an entry instead of storing an empty value.
    fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

/// Topology record of a node, stored in [`Nodes`].
///
/// A node exists exactly when it has a [`Nodes`] entry. `parent` never
/// changes after creation. `scope` is set only on Scope roots; any other
/// node resolves to the Scope of its nearest ancestor that has one (see
/// [`Pallet::resolve_scope`]).
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq, Debug,
)]
pub struct NodeInfo {
    /// `None` for a root node, `Some(parent_id)` otherwise.
    pub parent: Option<NodeId>,
    /// `Some(scope_id)` if this node is the root of Scope `scope_id`.
    pub scope: Option<ScopeId>,
}

/// Scope record, stored in [`Scopes`].
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq, Debug,
)]
pub struct ScopeInfo<AccountId: MaxEncodedLen> {
    /// Owner of the Scope. Never changes; [`Pallet::create_scope`] replaces
    /// the Scope with a new one to change the owner.
    pub owner: AccountId,
    /// Number of [`Access`] entries (distinct `(NodeId, AccountId)` pairs)
    /// stored under this Scope, at most [`MAX_ACCESS_ENTRIES_PER_SCOPE`]. See
    /// [Access entry limit](crate#access-entry-limit).
    #[codec(compact)]
    pub access_count: u32,
}

/// Result of [`Pallet::resolve_scope`] for a node.
///
/// `path` starts with the queried node and ends with the Scope root, so
/// `path.len()` is the node's scope-local depth (1 for a Scope root).
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug)]
pub struct ResolvedScope<AccountId> {
    /// Identifier of the Scope.
    pub id: ScopeId,
    /// Root node of the Scope, the node whose `NodeInfo.scope` is `id`.
    pub root: NodeId,
    /// Owner of the Scope.
    pub owner: AccountId,
    /// Nodes from the queried node (first) to the Scope root (last),
    /// both included.
    pub path: BoundedVec<NodeId, MaxScopeDepth>,
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

        /// Weights of this pallet's calls.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    /// Current storage version. See [`crate::migration`] for the upgrade
    /// from version 1.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

    /// [`NodeId`] of the next created node.
    #[pallet::storage]
    #[pallet::getter(fn next_node_id)]
    pub type NextNodeId<T> = StorageValue<_, NodeId, ValueQuery>;

    /// Parent link and Scope pointer of every node (see [`NodeInfo`]).
    ///
    /// A node exists exactly when it has an entry here.
    #[pallet::storage]
    #[pallet::getter(fn node_info)]
    pub type Nodes<T: Config> = StorageMap<_, Twox64Concat, NodeId, NodeInfo, OptionQuery>;

    /// Node metadata. No entry means the metadata is unset.
    #[pallet::storage]
    #[pallet::getter(fn meta_of)]
    pub type Meta<T: Config> = StorageMap<_, Twox64Concat, NodeId, NodeMeta>;

    /// Node payload. No entry means the payload is unset.
    #[pallet::storage]
    #[pallet::getter(fn payload_of)]
    pub type Payload<T: Config> = StorageMap<_, Twox64Concat, NodeId, NodePayload>;

    /// [`ScopeId`] of the next created Scope.
    #[pallet::storage]
    #[pallet::getter(fn next_scope_id)]
    pub type NextScopeId<T> = StorageValue<_, ScopeId, ValueQuery>;

    /// Owner and Access entry count of every Scope (see [`ScopeInfo`]).
    ///
    /// Holds an entry exactly for the `ScopeId`s referenced by a
    /// `Nodes[root].scope`. The entry is removed when the Scope is replaced
    /// or its root node deleted.
    ///
    /// `ScopeId` is allocated by the pallet, so the non-cryptographic
    /// `Twox64Concat` hasher is used.
    #[pallet::storage]
    #[pallet::getter(fn scope_info)]
    pub type Scopes<T: Config> =
        StorageMap<_, Twox64Concat, ScopeId, ScopeInfo<T::AccountId>, OptionQuery>;

    /// Capabilities delegated to an account at a node, keyed by
    /// `(ScopeId, (NodeId, AccountId))`. See [`AccessFlags`].
    ///
    /// An entry with no capability left is deleted rather than stored. All
    /// entries of a Scope are deleted when the Scope is replaced or its root
    /// node deleted.
    ///
    /// The first key (`ScopeId`) is allocated by the pallet and uses
    /// `Twox64Concat`, which also groups a Scope's entries under one prefix
    /// for removal. The second key contains a caller-chosen `AccountId` and
    /// uses `Blake2_128Concat`.
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

    /// Direct children of each node, in creation order, up to
    /// [`MAX_CHILDREN_PER_NODE`].
    #[pallet::storage]
    #[pallet::getter(fn children_of)]
    pub type Children<T: Config> =
        StorageMap<_, Twox64Concat, NodeId, BoundedVec<NodeId, MaxChildrenPerNode>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A node was created. [node_id, parent_id, creator]
        NodeCreated(NodeId, Option<NodeId>, T::AccountId),
        /// Node metadata was set or removed. [node_id, sender]
        MetaSet(NodeId, T::AccountId),
        /// Node payload was set or removed. [node_id, sender]
        PayloadSet(NodeId, T::AccountId),
        /// A node was deleted. [node_id, sender]
        NodeDeleted(NodeId, T::AccountId),
        /// A Scope was created, either for a new root node by `create_node`
        /// or by `create_scope`. When `create_scope` replaces a Scope, no
        /// separate event is emitted for the old one. [scope_id, root, owner]
        ScopeCreated(ScopeId, NodeId, T::AccountId),
        /// A Scope was deleted together with its root node by `delete_node`.
        /// [scope_id, root]
        ScopeDeleted(ScopeId, NodeId),
        /// A capability was granted.
        /// [scope_id, node_id, principal, capability, mode]
        AccessGranted(ScopeId, NodeId, T::AccountId, Capability, GrantMode),
        /// A capability was revoked. Also emitted when the capability was
        /// not granted. [scope_id, node_id, principal, capability]
        AccessRevoked(ScopeId, NodeId, T::AccountId, Capability),
    }

    #[pallet::error]
    #[derive(PartialEq)]
    pub enum Error<T> {
        /// The node does not exist.
        NodeNotFound,
        /// The parent node does not exist.
        ParentNotFound,
        /// The new node would have more than [`MAX_SCOPE_DEPTH`] nodes on
        /// its path to the Scope root.
        MaxScopeDepthExceeded,
        /// The parent already has [`MAX_CHILDREN_PER_NODE`] children.
        TooManyChildren,
        /// The node has children and cannot be deleted.
        NodeHasChildren,
        /// [`NextNodeId`] cannot be incremented without overflow.
        NodeIdExhausted,
        /// No Scope could be resolved for the node. Not expected while the
        /// storage invariants hold.
        ScopeNotFound,
        /// [`NextScopeId`] cannot be incremented without overflow.
        ScopeIdExhausted,
        /// The caller does not own the Scope the node resolves to.
        NotScopeOwner,
        /// The caller neither owns the Scope nor holds the required
        /// capability.
        AccessDenied,
        /// The Scope already holds [`MAX_ACCESS_ENTRIES_PER_SCOPE`] Access
        /// entries, so no entry for a new `(NodeId, AccountId)` pair can be
        /// added.
        TooManyAccessEntries,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a node under `parent_id`, or a root node if `parent_id` is
        /// `None`, with optional `meta` and `payload`.
        ///
        /// A root node may be created by any signed account and becomes the
        /// root of a new Scope owned by the caller. A child node requires
        /// the caller to own the Scope that `parent_id` resolves to
        /// ([`Capability::Write`] is not enough); it does not root a Scope
        /// and resolves to its parent's Scope.
        ///
        /// Emits [`Event::ScopeCreated`] for a root node, then
        /// [`Event::NodeCreated`].
        ///
        /// # Errors
        ///
        /// - [`Error::ParentNotFound`]: `parent_id` does not exist.
        /// - [`Error::NotScopeOwner`]: the caller does not own the parent's
        ///   Scope.
        /// - [`Error::MaxScopeDepthExceeded`]: the parent's path to its Scope
        ///   root already holds [`MAX_SCOPE_DEPTH`] nodes.
        /// - [`Error::TooManyChildren`]: the parent already has
        ///   [`MAX_CHILDREN_PER_NODE`] children.
        /// - [`Error::NodeIdExhausted`], [`Error::ScopeIdExhausted`]: an ID
        ///   counter cannot be incremented.
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

            // Read the next node ID (and, for a root node, the next Scope
            // ID) and check that the counters can be incremented. The
            // counters are written only after every check below has passed.
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

                // Adding a child requires the Scope owner; `Write` only
                // covers `Meta`/`Payload`.
                let resolved = Self::resolve_scope(pid)?;
                ensure!(resolved.owner == sender, Error::<T>::NotScopeOwner);

                // The child's path is the parent's path plus one node, so
                // the parent's path must be shorter than `MAX_SCOPE_DEPTH`.
                ensure!(
                    (resolved.path.len() as u32) < MAX_SCOPE_DEPTH,
                    Error::<T>::MaxScopeDepthExceeded
                );

                // Add to parent's children index
                <Children<T>>::try_mutate(pid, |children| {
                    children
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyChildren)
                })?;
            }

            // All checks passed: write the counters, the root node's Scope,
            // and the node itself.
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

        /// Set the metadata of `node_id` to `meta`, or remove it if `meta` is
        /// `None`.
        ///
        /// The caller must be authorized for [`Capability::Write`] at
        /// `node_id` (see [`Pallet::authorize`]). Emits [`Event::MetaSet`].
        ///
        /// # Errors
        ///
        /// - [`Error::NodeNotFound`]: `node_id` does not exist.
        /// - [`Error::AccessDenied`]: the caller is not authorized.
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

        /// Set the payload of `node_id` to `payload`, or remove it if
        /// `payload` is `None`.
        ///
        /// The caller must be authorized for [`Capability::Write`] at
        /// `node_id` (see [`Pallet::authorize`]). Emits
        /// [`Event::PayloadSet`].
        ///
        /// # Errors
        ///
        /// - [`Error::NodeNotFound`]: `node_id` does not exist.
        /// - [`Error::AccessDenied`]: the caller is not authorized.
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

        /// Delete `node_id` together with its `Meta` and `Payload`.
        ///
        /// The caller must own the Scope that `node_id` resolves to; for a
        /// Scope root that is the node's own Scope. Only a node without
        /// children can be deleted.
        ///
        /// If `node_id` roots a Scope, the Scope's [`Scopes`] record and all
        /// of its [`Access`] entries are deleted as well and
        /// [`Event::ScopeDeleted`] is emitted; this is the only way a Scope
        /// is removed. Access entries granted at `node_id` in the Scope it
        /// resolves to are not deleted (see
        /// [Access entry limit](crate#access-entry-limit)). Emits
        /// [`Event::NodeDeleted`].
        ///
        /// Weight is charged for [`MAX_ACCESS_ENTRIES_PER_SCOPE`] deleted
        /// Access entries and refunded to the number actually deleted.
        ///
        /// # Errors
        ///
        /// - [`Error::NodeNotFound`]: `node_id` does not exist.
        /// - [`Error::NotScopeOwner`]: the caller does not own the Scope.
        /// - [`Error::NodeHasChildren`]: `node_id` has children.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::delete_node(MAX_ACCESS_ENTRIES_PER_SCOPE))]
        pub fn delete_node(origin: OriginFor<T>, node_id: NodeId) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            // Check the node exists, and get its topology info.
            let info = <Nodes<T>>::get(node_id).ok_or(Error::<T>::NodeNotFound)?;

            // Only the Scope owner can delete a node.
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

            // If the node roots a Scope, delete the Scope's Access entries
            // and record.
            let mut cleared_access_items = 0;
            if let Some(stale_scope) = info.scope {
                cleared_access_items = <Scopes<T>>::get(stale_scope)
                    .map(|scope_info| scope_info.access_count)
                    .unwrap_or(0);
                Self::clear_scope_access(stale_scope, cleared_access_items);
                <Scopes<T>>::remove(stale_scope);
                Self::deposit_event(Event::ScopeDeleted(stale_scope, node_id));
            }

            // Remove the node's attributes
            <Meta<T>>::remove(node_id);
            <Payload<T>>::remove(node_id);
            <Nodes<T>>::remove(node_id);

            Self::deposit_event(Event::NodeDeleted(node_id, sender));
            Ok(Some(T::WeightInfo::delete_node(cleared_access_items)).into())
        }

        /// Make `node_id` the root of a new Scope owned by the caller.
        ///
        /// A new `ScopeId` is always allocated. The effect depends on
        /// whether `node_id` already roots a Scope:
        ///
        /// - **Ordinary node**: `node_id` becomes the root of a new nested
        ///   Scope, and `node_id` and its descendants in the current Scope
        ///   now resolve to it. Allowed for the owner of the Scope `node_id`
        ///   resolves to, or for an account holding
        ///   [`Capability::CreateScope`] at `node_id` (either mode) or with
        ///   [`GrantMode::Subtree`] at an ancestor within that Scope.
        /// - **Scope root**: the Scope is replaced. Nodes that resolved to
        ///   the old Scope resolve to the new one, and the old Scope's
        ///   [`Scopes`] record and all of its [`Access`] entries are
        ///   deleted. Allowed for the owner of the old Scope, for an account
        ///   holding [`Capability::CreateScope`] at `node_id` in the old
        ///   Scope, and for the owner of the Scope that `node_id`'s parent
        ///   resolves to (the immediately enclosing Scope). Owners of Scopes
        ///   further up the tree and Access holders of the enclosing Scope
        ///   are not allowed.
        ///
        /// Emits [`Event::ScopeCreated`]. Weight is charged for
        /// [`MAX_ACCESS_ENTRIES_PER_SCOPE`] deleted Access entries and
        /// refunded to the number actually deleted.
        ///
        /// # Errors
        ///
        /// - [`Error::NodeNotFound`]: `node_id` does not exist.
        /// - [`Error::AccessDenied`]: the caller is not allowed.
        /// - [`Error::ScopeIdExhausted`]: [`NextScopeId`] cannot be
        ///   incremented.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::create_scope(MAX_ACCESS_ENTRIES_PER_SCOPE))]
        pub fn create_scope(origin: OriginFor<T>, node_id: NodeId) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            let info = <Nodes<T>>::get(node_id).ok_or(Error::<T>::NodeNotFound)?;
            let old_scope_access_items = info
                .scope
                .and_then(|scope_id| {
                    <Scopes<T>>::get(scope_id).map(|scope_info| scope_info.access_count)
                })
                .unwrap_or(0);

            Self::authorize_create_scope(node_id, &info, &sender)?;

            let scope_id =
                Self::allocate_scope(node_id, info.scope, sender.clone(), old_scope_access_items)?;

            Self::deposit_event(Event::ScopeCreated(scope_id, node_id, sender));
            Ok(Some(T::WeightInfo::create_scope(old_scope_access_items)).into())
        }

        /// Grant `capability` with `mode` to `principal` at `node_id`, in the
        /// Scope that `node_id` resolves to.
        ///
        /// Only the owner of that Scope may call this. If `principal`
        /// already holds `capability` at `node_id`, its mode is replaced by
        /// `mode`. The first grant for a `(node_id, principal)` pair creates
        /// an Access entry and increments [`ScopeInfo::access_count`]; further
        /// grants for the same pair update the entry and do not. Emits
        /// [`Event::AccessGranted`].
        ///
        /// # Errors
        ///
        /// - [`Error::NodeNotFound`]: `node_id` does not exist.
        /// - [`Error::NotScopeOwner`]: the caller does not own the Scope.
        /// - [`Error::TooManyAccessEntries`]: the pair has no entry yet and
        ///   the Scope already holds [`MAX_ACCESS_ENTRIES_PER_SCOPE`]
        ///   entries.
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

        /// Revoke `capability` from `principal` at `node_id`, in the Scope
        /// that `node_id` resolves to.
        ///
        /// Only the owner of that Scope may call this. When the revoked
        /// capability was the last one in the `(node_id, principal)` entry,
        /// the entry is deleted and [`ScopeInfo::access_count`] decremented;
        /// otherwise the entry keeps its other capabilities and the count is
        /// unchanged. Revoking a capability that is not granted succeeds
        /// without changing storage. Emits [`Event::AccessRevoked`] in every
        /// successful case.
        ///
        /// # Errors
        ///
        /// - [`Error::NodeNotFound`]: `node_id` does not exist.
        /// - [`Error::NotScopeOwner`]: the caller does not own the Scope.
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
            let mut flags = <Access<T>>::get(resolved.id, &key);
            flags.revoke(capability);
            if flags.is_empty() {
                if <Access<T>>::contains_key(resolved.id, &key) {
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
        /// Resolve the Scope that `node_id` belongs to.
        ///
        /// Starting at `node_id`, walks towards the tree root until it
        /// reaches a node whose `NodeInfo.scope` is set, and returns that
        /// Scope's id, root, and owner together with the visited path (see
        /// [`ResolvedScope`]).
        /// The walk visits at most [`MAX_SCOPE_DEPTH`] nodes. Authorization
        /// checks and the depth check of [`Pallet::create_node`] use the
        /// returned path.
        ///
        /// # Errors
        ///
        /// - [`Error::NodeNotFound`]: `node_id` does not exist.
        /// - [`Error::ScopeNotFound`]: no Scope root is found within
        ///   [`MAX_SCOPE_DEPTH`] nodes, or the Scope has no [`Scopes`]
        ///   record. Not expected while the storage invariants hold.
        pub fn resolve_scope(node_id: NodeId) -> Result<ResolvedScope<T::AccountId>, Error<T>> {
            let mut path: BoundedVec<NodeId, MaxScopeDepth> = BoundedVec::default();
            let mut current = node_id;

            for _ in 0..MAX_SCOPE_DEPTH {
                path.try_push(current)
                    .map_err(|_| Error::<T>::ScopeNotFound)?;

                let info = <Nodes<T>>::get(current).ok_or(Error::<T>::NodeNotFound)?;

                if let Some(scope_id) = info.scope {
                    let scope_info = <Scopes<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;
                    return Ok(ResolvedScope {
                        id: scope_id,
                        root: current,
                        owner: scope_info.owner,
                        path,
                    });
                }

                current = info.parent.ok_or(Error::<T>::ScopeNotFound)?;
            }

            Err(Error::<T>::ScopeNotFound)
        }

        /// Whether `account_id` may use `capability` at `node_id`, with the
        /// same check as the calls that require it:
        ///
        /// - [`Capability::Write`]: [`Self::authorize`], as in
        ///   [`Pallet::set_meta`] and [`Pallet::set_payload`];
        /// - [`Capability::CreateScope`]: the check of
        ///   [`Pallet::create_scope`], i.e. [`Self::authorize`] or, at a
        ///   nested Scope root, ownership of the immediately enclosing Scope.
        ///
        /// Returns `false` if `node_id` does not exist or no Scope can be
        /// resolved for it.
        ///
        /// Exposed to off-chain clients by the `CpsApi` runtime API.
        pub fn has_capability(
            node_id: NodeId,
            account_id: &T::AccountId,
            capability: Capability,
        ) -> bool {
            match capability {
                Capability::CreateScope => <Nodes<T>>::get(node_id).is_some_and(|info| {
                    Self::authorize_create_scope(node_id, &info, account_id).is_ok()
                }),
                Capability::Write => Self::authorize(node_id, account_id, capability).is_ok(),
            }
        }

        /// Check that `sender` may use `capability` at `node_id`.
        ///
        /// Resolves the Scope of `node_id` once and returns `Ok(true)` if
        /// `sender`:
        ///
        /// - owns that Scope; or
        /// - holds `capability` at `node_id` in that Scope, in either
        ///   [`GrantMode`]; or
        /// - holds `capability` with [`GrantMode::Subtree`] at an ancestor of
        ///   `node_id` on the resolved path.
        ///
        /// The path ends at the Scope root, so Access entries of enclosing
        /// Scopes are never consulted. Never returns `Ok(false)`.
        /// [`Pallet::create_scope`] additionally allows the owner of the
        /// immediately enclosing Scope at a nested Scope root.
        ///
        /// # Errors
        ///
        /// - [`Error::AccessDenied`]: none of the conditions hold.
        /// - Errors of [`Self::resolve_scope`].
        pub fn authorize(
            node_id: NodeId,
            sender: &T::AccountId,
            capability: Capability,
        ) -> Result<bool, Error<T>> {
            let resolved = Self::resolve_scope(node_id)?;
            if resolved.owner == *sender {
                return Ok(true);
            }

            for (index, current) in resolved.path.iter().enumerate() {
                let flags = <Access<T>>::get(resolved.id, (*current, sender.clone()));
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

        /// Check that `sender` may call [`Pallet::create_scope`] on `node_id`,
        /// whose [`NodeInfo`] is `info`.
        ///
        /// Allowed if [`Self::authorize`] allows [`Capability::CreateScope`]
        /// at `node_id`, or if `node_id` roots a Scope and `sender` owns the
        /// Scope that its parent resolves to (the immediately enclosing
        /// Scope). Shared by [`Pallet::create_scope`] and
        /// [`Self::has_capability`].
        ///
        /// # Errors
        ///
        /// The error of [`Self::authorize`] if neither condition holds.
        fn authorize_create_scope(
            node_id: NodeId,
            info: &NodeInfo,
            sender: &T::AccountId,
        ) -> Result<(), Error<T>> {
            Self::authorize(node_id, sender, Capability::CreateScope)
                .map(|_| ())
                .or_else(|err| {
                    if Self::is_enclosing_scope_owner(info, sender) {
                        Ok(())
                    } else {
                        Err(err)
                    }
                })
        }

        /// Whether `sender` owns the Scope immediately enclosing the Scope
        /// rooted at the node described by `info`, i.e. the Scope that the
        /// node's parent resolves to.
        ///
        /// Returns `false` if the node is not a Scope root or has no parent.
        fn is_enclosing_scope_owner(info: &NodeInfo, sender: &T::AccountId) -> bool {
            match (info.scope, info.parent) {
                (Some(_), Some(parent)) => {
                    Self::resolve_scope(parent).is_ok_and(|resolved| resolved.owner == *sender)
                }
                _ => false,
            }
        }

        /// Allocate a new `ScopeId` owned by `owner` and make `root` its root.
        ///
        /// `old_scope` is the current `NodeInfo.scope` of `root`. If it is
        /// `Some`, that Scope's Access entries (`old_scope_access_items` of
        /// them, its `access_count`) and its [`Scopes`] record are deleted.
        ///
        /// # Errors
        ///
        /// - [`Error::ScopeIdExhausted`]: [`NextScopeId`] cannot be
        ///   incremented.
        fn allocate_scope(
            root: NodeId,
            old_scope: Option<ScopeId>,
            owner: T::AccountId,
            old_scope_access_items: u32,
        ) -> Result<ScopeId, Error<T>> {
            let scope_id = <NextScopeId<T>>::get();
            let next_id = scope_id
                .checked_add(1)
                .ok_or(Error::<T>::ScopeIdExhausted)?;
            <NextScopeId<T>>::put(next_id);

            if let Some(stale_scope) = old_scope {
                Self::clear_scope_access(stale_scope, old_scope_access_items);
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

        /// Delete all [`Access`] entries stored under `scope_id`.
        ///
        /// `access_items` must be the Scope's [`ScopeInfo::access_count`]; it
        /// is used as the `clear_prefix` limit, so the cost of the call is
        /// bounded by it. The caller also removes the [`Scopes`] record, and
        /// removes or replaces the root's `NodeInfo.scope`, in the same call.
        ///
        /// Since `access_count` equals the number of stored entries, a single
        /// `clear_prefix` call removes all of them. If entries remain, the
        /// invariant is broken: this is reported with
        /// [`frame_support::defensive!`] and the remaining entries are left
        /// in storage.
        pub(crate) fn clear_scope_access(scope_id: ScopeId, access_items: u32) {
            let result = <Access<T>>::clear_prefix(scope_id, access_items, None);
            if result.maybe_cursor.is_some() {
                frame_support::defensive!(
                    "CPS: clear_prefix left Access entries behind despite \
                     ScopeInfo.access_count bound",
                    scope_id
                );
            }
        }
    }
}
