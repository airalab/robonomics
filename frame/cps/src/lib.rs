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
//! # CPS Pallet: On-chain Hierarchical Tree for Cyber-Physical Systems
//!
//! This pallet provides a decentralized registry for cyber-physical systems organized
//! as a hierarchical tree structure, with authority and resources delegated through
//! a **Scope / Access** architecture.
//!
//! ## Architecture
//!
//! ### Storage Layout
//!
//! Each node's attributes live in their own storage map, keyed by `NodeId`:
//!
//! 1. **`Parent`**: Mapping `NodeId` → `Option<NodeId>`
//!    - Presence of a key means the node exists; the value is `None` for a
//!      root node and `Some(parent_id)` otherwise
//!    - **Immutable** once a node is created - there is no way to change it
//!
//! 2. **`Meta`** / **`Payload`**: Mapping `NodeId` → `NodeData`
//!    - An entry is present only when the corresponding field has been set;
//!      absence means "unset", not "empty"
//!
//! 3. **`ActiveScope`**: Mapping `NodeId` → `ScopeId`
//!    - Present only on nodes that are the root of an active [`Scope`](self#scope)
//!    - A node without an entry resolves to the Scope of the nearest ancestor
//!      that has one (see [`Pallet::resolve_scope`])
//!
//! 4. **`ScopeRoot`** / **`ScopeOwner`** / **`ScopeResources`**: `ScopeId`-keyed
//!    mappings describing a Scope's root node, owner account and local
//!    resource limits, respectively. A `Scope` is an architectural concept
//!    composed from these independent mappings, not a stored struct.
//!
//! 5. **`Access`**: Mapping `(ScopeId, (NodeId, AccountId, Capability))` →
//!    `bool` ("inherited"). Delegates a `Capability` to an `AccountId` at a
//!    specific `NodeId` within one Scope.
//!
//! 6. **`NodesByParent`** / **`RootNodes`**: Index structures for O(1) child and
//!    root-node lookups
//!
//! ### Scope
//!
//! A `Scope` is the administrative and economic boundary of the CPS
//! hierarchy. `ScopeId` is a globally unique, never-reused numeric
//! identifier. Every active CPS node resolves to exactly one Scope by
//! walking `parent` links until the nearest ancestor with an `ActiveScope`
//! entry is found (see [`Pallet::resolve_scope`]):
//!
//! ```text
//! Global
//! Scope #1 / owner=A
//! |
//! `-- Japan
//!     Scope #7 / owner=B
//!     |
//!     `-- University
//! ```
//!
//! `University` resolves to `Scope #7` (owner `B`); `Japan`'s Scope has no
//! implicit administrative rights over `Global`'s other, independently
//! owned children, and vice versa. A nested Scope is always a hard boundary:
//! it stops inheritance of authority, `Access`, and resource limits, even
//! when parent and child Scope owners are the same account.
//!
//! ### Creating and Replacing a Scope
//!
//! Every CPS root is allocated a fresh Scope, owned by its creator, when the
//! root is created. [`Pallet::create_scope`] establishes a new Scope on any
//! node within the caller's Scope (owner authority), or replaces an existing
//! Scope on its own root node (owner authority, or a delegated
//! [`Capability::CreateScope`] grant). Replacement allocates a brand-new
//! `ScopeId` — the previous Scope's `Access` and resource entries become
//! immediately inactive without requiring any descendant rewrite.
//!
//! Changing control of a Scope means creating another Scope; a Scope's
//! `owner` is immutable once created. There is no transfer/accept state
//! machine.
//!
//! ### Deleting a Scope
//!
//! [`Pallet::delete_scope`] removes the administrative/economic boundary
//! from a CPS node without deleting the node or its descendants. Only the
//! Scope's owner may delete it (never through inherited `Access`). A CPS
//! root's Scope can never be deleted, since every node must resolve to
//! exactly one Scope. Nested Scopes below the deleted boundary are
//! unaffected: they keep resolving to their own `ScopeId` because
//! [`Pallet::resolve_scope`] always finds the *nearest* active Scope.
//!
//! ### Access
//!
//! [`Pallet::grant_access`] / [`Pallet::revoke_access`] let a Scope owner
//! delegate a [`Capability`] to another account at a specific `NodeId`,
//! either for that exact node (`inherited = false`) or for the node and all
//! its descendants within the same Scope (`inherited = true`). Access never
//! crosses a nested Scope boundary. The Scope owner always has implicit
//! authority and does not need explicit `Access` entries.
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
//! - **Scope resolution**: `resolve_scope` walks `parent` links one hop at a
//!   time until an `ActiveScope` entry is found → O(depth)
//! - **Depth validation**: counting `parent` hops up to the root →
//!   O(depth), bounded by `MAX_TREE_DEPTH`
//! - **Child lookup**: Direct index access via `NodesByParent` → O(1)
//!
//! ### Compact Encoding
//!
//! `NodeId` uses `#[codec(compact)]` attribute to enable SCALE compact encoding, which uses
//! variable-length encoding to reduce storage costs for small node IDs.
//!
//! ## Usage Examples
//!
//! ### Creating a Root Node
//!
//! ```ignore
//! use pallet_robonomics_cps::NodeData;
//! use frame_support::BoundedVec;
//!
//! // Plain metadata
//! let meta = Some(BoundedVec::try_from(b"sensor_config".to_vec()).unwrap());
//!
//! // Create root (parent = None) - the caller becomes the owner of a new
//! // Scope allocated for this root.
//! Cps::create_node(origin, None, meta, None)?;
//! ```
//!
//! ### Creating a Child Node
//!
//! ```ignore
//! use pallet_robonomics_cps::NodeId;
//!
//! // Data can be encrypted by client before submission
//! let encrypted_bytes = client_side_encrypt(sensitive_data);
//! let payload = Some(BoundedVec::try_from(encrypted_bytes).unwrap());
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
//! Cps::create_scope(origin, NodeId(5), Default::default())?;
//! ```
//!
//! ### Querying the Tree
//!
//! ```ignore
//! // Get a node's parent (existence check + parent link)
//! let parent = Parent::<T>::get(NodeId(0)).ok_or(Error::<T>::NodeNotFound)?;
//!
//! // Get all children
//! let children = NodesByParent::<T>::get(NodeId(0));
//!
//! // Get all root nodes
//! let roots = RootNodes::<T>::get();
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
//! 4. **Index Consistency**: `NodesByParent` and `RootNodes` stay synchronized
//!    with `Parent`.
//! 5. **Deletion Safety**: Cannot delete nodes with children.
//! 6. **Depth Limits**: Tree depth never exceeds `MAX_TREE_DEPTH`.
//! 7. **Stable Identity**: `NodeId` is never reused, and neither is `ScopeId`.
//! 8. **Immutable Scope Owner/Root**: A Scope's owner and root are fixed at
//!    creation; changing control means creating another Scope.
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

/// Callback trait invoked when a payload is set on a node.
///
/// This trait allows runtime-level hooks to be executed after a payload has been successfully
/// updated on a CPS node. It follows Substrate's standard pattern for runtime callbacks and
/// enables various use cases such as:
///
/// - **Indexing**: Track payload changes for off-chain indexing or querying
/// - **Notifications**: Trigger events or notifications to external systems
/// - **Analytics**: Collect metrics about payload updates
/// - **Automation**: Chain additional actions based on payload changes
/// - **Logging**: Maintain an audit trail of payload modifications
///
/// The callback is invoked AFTER the payload has been successfully written to storage,
/// ensuring that the operation has been completed before any side effects are triggered.
///
/// # Example Implementation
///
/// ```ignore
/// use pallet_robonomics_cps::{OnPayloadSet, NodeId, NodeData};
///
/// pub struct MyPayloadHandler;
///
/// impl OnPayloadSet<AccountId> for MyPayloadHandler {
///     fn on_payload_set(
///         node_id: NodeId,
///         meta: Option<NodeData>,
///         payload: Option<NodeData>
///     ) {
///         // Custom logic here - e.g., emit a custom event, update an index, etc.
///         log::info!("Payload set on node {:?}", node_id);
///     }
/// }
/// ```
///
/// # Multiple Handlers
///
/// Multiple handlers can be combined using tuples:
///
/// ```ignore
/// type OnPayloadSet = (HandlerA, HandlerB, HandlerC);
/// ```
pub trait OnPayloadSet<AccountId> {
    /// Called when a payload is set on a node.
    ///
    /// # Parameters
    ///
    /// - `node_id`: The ID of the node whose payload was updated
    /// - `meta`: The current metadata of the node (if any)
    /// - `payload`: The new payload that was set (if any, None means payload was cleared)
    fn on_payload_set(node_id: NodeId, meta: Option<NodeData>, payload: Option<NodeData>);
}

/// Default no-op implementation for `()` type.
///
/// This allows using `type OnPayloadSet = ()` in the runtime configuration
/// to disable the callback without requiring an explicit implementation.
impl<AccountId> OnPayloadSet<AccountId> for () {
    fn on_payload_set(_node_id: NodeId, _meta: Option<NodeData>, _payload: Option<NodeData>) {
        // No-op: do nothing
    }
}

/// Implementation for tuples to support multiple handlers.
///
/// This allows combining multiple callback handlers:
/// ```ignore
/// type OnPayloadSet = (HandlerA, HandlerB);
/// ```
macro_rules! impl_on_payload_set_for_tuples {
    ($($t:ident),+) => {
        impl<AccountId, $($t: OnPayloadSet<AccountId>),+> OnPayloadSet<AccountId> for ($($t,)+) {
            fn on_payload_set(node_id: NodeId, meta: Option<NodeData>, payload: Option<NodeData>) {
                $(
                    $t::on_payload_set(node_id, meta.clone(), payload.clone());
                )+
            }
        }
    };
}

impl_on_payload_set_for_tuples!(A);
impl_on_payload_set_for_tuples!(A, B);
impl_on_payload_set_for_tuples!(A, B, C);
impl_on_payload_set_for_tuples!(A, B, C, D);
impl_on_payload_set_for_tuples!(A, B, C, D, E);

/// Maximum data size for node metadata and payload.
///
/// Set to 2048 bytes to accommodate typical sensor readings, configuration data,
/// and encrypted payloads while preventing DoS attacks via large data submissions.
pub const MAX_DATA_SIZE: u32 = 2048;

/// Maximum tree depth (number of ancestors) a node may have.
///
/// Enforced at `create_node` time by walking the parent chain; bounds the
/// cost of `resolve_scope` and depth validation, both O(depth).
pub const MAX_TREE_DEPTH: u32 = 32;

/// Maximum number of direct children a single node may have.
///
/// Bounds the size of the `NodesByParent` index entry for any given node.
pub const MAX_CHILDREN_PER_NODE: u32 = 100;

/// Maximum number of root nodes (nodes with no parent) that may exist.
///
/// Bounds the size of the `RootNodes` index.
pub const MAX_ROOT_NODES: u32 = 100;

/// Maximum number of `(Resource, Limit)` entries that may be set on a Scope
/// in a single `create_scope` call.
pub const MAX_SCOPE_RESOURCES: u32 = 8;

/// [`ConstU32`] wrapper around [`MAX_DATA_SIZE`] for use as a `BoundedVec` bound.
pub type MaxDataSize = ConstU32<MAX_DATA_SIZE>;
/// [`ConstU32`] wrapper around [`MAX_TREE_DEPTH`] for use as a `BoundedVec` bound.
pub type MaxTreeDepth = ConstU32<MAX_TREE_DEPTH>;
/// [`ConstU32`] wrapper around [`MAX_CHILDREN_PER_NODE`] for use as a `BoundedVec` bound.
pub type MaxChildrenPerNode = ConstU32<MAX_CHILDREN_PER_NODE>;
/// [`ConstU32`] wrapper around [`MAX_ROOT_NODES`] for use as a `BoundedVec` bound.
pub type MaxRootNodes = ConstU32<MAX_ROOT_NODES>;
/// [`ConstU32`] wrapper around [`MAX_SCOPE_RESOURCES`] for use as a `BoundedVec` bound.
pub type MaxScopeResources = ConstU32<MAX_SCOPE_RESOURCES>;

/// Type alias for node data - bounded vector of bytes.
///
/// Stores data as plain bytes up to MAX_DATA_SIZE (2048 bytes).
/// For sensitive data, encryption should be handled at the client level
/// before storing in the pallet.
///
/// # Client-Side Encryption Recommendation
///
/// For encryption use cases, applications should:
/// 1. Encrypt sensitive data on the client side
/// 2. Store encrypted bytes in this BoundedVec
/// 3. Decrypt data after retrieving from chain
pub type NodeData = BoundedVec<u8, MaxDataSize>;

/// Node identifier newtype with compact encoding for efficient storage.
///
/// The `#[codec(compact)]` attribute enables SCALE compact encoding, which uses
/// variable-length encoding to reduce storage costs for small node IDs:
///
/// | Node ID Range | Standard | Compact | Savings |
/// |---------------|----------|---------|---------|
/// | 0-63          | 8 bytes  | 1 byte  | 87%     |
/// | 64-16,383     | 8 bytes  | 2 bytes | 75%     |
/// | 16,384+       | 8 bytes  | 3+ bytes| 62%+    |
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

/// Globally unique, never-reused identifier of a [`Scope`](self#scope).
///
/// A `Scope` is an architectural concept composed from independent
/// `ScopeId`-keyed mappings ([`ScopeRoot`], [`ScopeOwner`],
/// [`ScopeResources`]) rather than a single stored struct.
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
    /// Authority to mutate a node's `Meta` / `Payload`.
    Write,
    /// Authority to create a replacement Scope at the exact Scope root that
    /// grants it. The sole mechanism for handing over control of a Scope.
    CreateScope,
}

/// Key identifying a single [`Access`] entry: the node the grant applies to,
/// the account it is granted to, and the delegated [`Capability`].
pub type AccessKey<AccountId> = (NodeId, AccountId, Capability);

/// A Scope-local resource family that [`ScopeResources`] may bound.
///
/// Absence of a [`ScopeResources`] entry for a `(ScopeId, Resource)` pair
/// means no additional Scope-local limit applies for that resource; the
/// Scope owner's Subscription remains the global resource entitlement.
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
pub enum Resource {
    /// Transaction resource consumed on behalf of this Scope.
    Transaction,
    /// Storage resource consumed on behalf of this Scope.
    Storage,
    /// Compute resource consumed on behalf of this Scope.
    Compute,
}

/// Scope-local limit for a [`Resource`].
pub type Limit = u128;

/// Bounded list of `(Resource, Limit)` pairs accepted by [`Pallet::create_scope`].
pub type ScopeResourceList = BoundedVec<(Resource, Limit), MaxScopeResources>;

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

        /// Callback handler invoked when a payload is set on a node.
        type OnPayloadSet: OnPayloadSet<Self::AccountId>;

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

    /// Node existence and parent link.
    ///
    /// Presence of a key means the node exists. The value is `None` for a
    /// root node and `Some(parent_id)` for a child - **immutable** once a
    /// node is created, there is no way to change it.
    #[pallet::storage]
    #[pallet::getter(fn parent_of)]
    pub type Parent<T: Config> = StorageMap<_, Blake2_128Concat, NodeId, Option<NodeId>>;

    /// Node metadata. An entry is present only when metadata has been set.
    #[pallet::storage]
    #[pallet::getter(fn meta_of)]
    pub type Meta<T: Config> = StorageMap<_, Blake2_128Concat, NodeId, NodeData>;

    /// Node payload. An entry is present only when a payload has been set.
    #[pallet::storage]
    #[pallet::getter(fn payload_of)]
    pub type Payload<T: Config> = StorageMap<_, Blake2_128Concat, NodeId, NodeData>;

    /// Next Scope ID counter. `ScopeId` is globally unique and never reused.
    #[pallet::storage]
    #[pallet::getter(fn next_scope_id)]
    pub type NextScopeId<T> = StorageValue<_, ScopeId, ValueQuery>;

    /// The active Scope rooted at a given node, if any.
    ///
    /// An entry is present only on nodes that are the root of an active
    /// Scope. Nodes without an entry resolve to the Scope of the nearest
    /// ancestor that has one - see [`Pallet::resolve_scope`].
    #[pallet::storage]
    #[pallet::getter(fn active_scope)]
    pub type ActiveScope<T: Config> = StorageMap<_, Blake2_128Concat, NodeId, ScopeId>;

    /// The root `NodeId` of a Scope. Immutable once a Scope is created.
    #[pallet::storage]
    #[pallet::getter(fn scope_root)]
    pub type ScopeRoot<T: Config> = StorageMap<_, Blake2_128Concat, ScopeId, NodeId>;

    /// The owner `AccountId` of a Scope. Immutable once a Scope is created.
    #[pallet::storage]
    #[pallet::getter(fn scope_owner)]
    pub type ScopeOwner<T: Config> = StorageMap<_, Blake2_128Concat, ScopeId, T::AccountId>;

    /// Scope-local resource limits. Absence of an entry means no additional
    /// Scope-local limit for that resource.
    #[pallet::storage]
    #[pallet::getter(fn scope_resources)]
    pub type ScopeResources<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, ScopeId, Blake2_128Concat, Resource, Limit>;

    /// Access delegations, scoped to a `ScopeId`. The stored `bool` is
    /// `inherited`: whether the grant propagates to descendants of the
    /// granted `NodeId` (while they resolve to the same `ScopeId`).
    #[pallet::storage]
    #[pallet::getter(fn access)]
    pub type Access<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ScopeId,
        Blake2_128Concat,
        AccessKey<T::AccountId>,
        bool,
    >;

    /// Index of children by parent node
    #[pallet::storage]
    #[pallet::getter(fn nodes_by_parent)]
    pub type NodesByParent<T: Config> =
        StorageMap<_, Blake2_128Concat, NodeId, BoundedVec<NodeId, MaxChildrenPerNode>, ValueQuery>;

    /// Root nodes (nodes without parents)
    #[pallet::storage]
    #[pallet::getter(fn root_nodes)]
    pub type RootNodes<T: Config> = StorageValue<_, BoundedVec<NodeId, MaxRootNodes>, ValueQuery>;

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
        /// A new Scope was created (or replaced an existing one)
        /// [scope_id, root, owner]
        ScopeCreated(ScopeId, NodeId, T::AccountId),
        /// A Scope boundary was removed [scope_id, root]
        ScopeDeleted(ScopeId, NodeId),
        /// Access was granted [scope_id, node_id, principal, capability, inherited]
        AccessGranted(ScopeId, NodeId, T::AccountId, Capability, bool),
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
        /// Maximum tree depth exceeded
        MaxDepthExceeded,
        /// Too many children for node
        TooManyChildren,
        /// Too many root nodes
        TooManyRootNodes,
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
        /// A CPS root's Scope can never be deleted
        CannotDeleteRootScope,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new node.
        ///
        /// Creating a root node (`parent_id: None`) allocates a fresh Scope
        /// owned by the caller. Creating a child node requires the caller to
        /// hold `Write` authority (owner or matching `Access`) over
        /// `parent_id`'s resolved Scope; the child does not get its own
        /// Scope.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_node())]
        pub fn create_node(
            origin: OriginFor<T>,
            parent_id: Option<NodeId>,
            meta: Option<NodeData>,
            payload: Option<NodeData>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Reserve the terminal counter value rather than ever reusing an ID.
            let node_id = <NextNodeId<T>>::get();
            let next_id = node_id
                .0
                .checked_add(1)
                .ok_or(Error::<T>::NodeIdExhausted)?;
            <NextNodeId<T>>::put(NodeId(next_id));

            if let Some(pid) = parent_id {
                ensure!(<Parent<T>>::contains_key(pid), Error::<T>::ParentNotFound);
                Self::authorize(&sender, pid, Capability::Write)?;

                // Check tree depth by walking the parent chain up to the root.
                ensure!(
                    Self::depth_of(pid)? < MAX_TREE_DEPTH,
                    Error::<T>::MaxDepthExceeded
                );

                // Add to parent's children index
                <NodesByParent<T>>::try_mutate(pid, |children| {
                    children
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyChildren)
                })?;
            } else {
                // Root node always allocates a fresh, caller-owned Scope.
                <RootNodes<T>>::try_mutate(|roots| {
                    roots
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyRootNodes)
                })?;

                let scope_id =
                    Self::allocate_scope(node_id, sender.clone(), &ScopeResourceList::default())?;
                Self::deposit_event(Event::ScopeCreated(scope_id, node_id, sender.clone()));
            }

            // Store the node's attributes
            <Parent<T>>::insert(node_id, parent_id);
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
        #[pallet::weight(T::WeightInfo::set_meta())]
        pub fn set_meta(
            origin: OriginFor<T>,
            node_id: NodeId,
            meta: Option<NodeData>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Self::authorize(&sender, node_id, Capability::Write)?;

            match meta {
                Some(meta) => <Meta<T>>::insert(node_id, meta),
                None => <Meta<T>>::remove(node_id),
            }

            Self::deposit_event(Event::MetaSet(node_id, sender));
            Ok(())
        }

        /// Set node payload
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_payload())]
        pub fn set_payload(
            origin: OriginFor<T>,
            node_id: NodeId,
            payload: Option<NodeData>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Self::authorize(&sender, node_id, Capability::Write)?;

            match payload.clone() {
                Some(payload) => <Payload<T>>::insert(node_id, payload),
                None => <Payload<T>>::remove(node_id),
            }

            let meta = <Meta<T>>::get(node_id);

            Self::deposit_event(Event::PayloadSet(node_id, sender));

            // Invoke the callback after successful payload update
            T::OnPayloadSet::on_payload_set(node_id, meta, payload);

            Ok(())
        }

        /// Delete a node.
        ///
        /// Only leaf nodes (no children) can be deleted. If the node is the
        /// root of an active Scope, that Scope's `ActiveScope` entry is
        /// removed as well; the rest of the Scope's physical state
        /// (`ScopeRoot` / `ScopeOwner` / `ScopeResources` / `Access`) is left
        /// for background garbage collection.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::delete_node())]
        pub fn delete_node(origin: OriginFor<T>, node_id: NodeId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Check the node exists, and get its parent
            ensure!(<Parent<T>>::contains_key(node_id), Error::<T>::NodeNotFound);
            let parent = <Parent<T>>::get(node_id).flatten();

            // Verify Write authority
            Self::authorize(&sender, node_id, Capability::Write)?;

            // Check if node has children
            let children = <NodesByParent<T>>::get(node_id);
            ensure!(children.is_empty(), Error::<T>::NodeHasChildren);

            // Remove from parent's children index
            if let Some(parent_id) = parent {
                <NodesByParent<T>>::mutate(parent_id, |children| {
                    children.retain(|&id| id != node_id);
                });
            } else {
                // Remove from root nodes
                <RootNodes<T>>::mutate(|roots| {
                    roots.retain(|&id| id != node_id);
                });
            }

            // Remove the node's children index entry
            <NodesByParent<T>>::remove(node_id);

            // Remove the Scope boundary attached to this node, if any. The
            // rest of that Scope's physical state is left for GC.
            <ActiveScope<T>>::remove(node_id);

            // Remove the node's attributes
            <Meta<T>>::remove(node_id);
            <Payload<T>>::remove(node_id);
            <Parent<T>>::remove(node_id);

            Self::deposit_event(Event::NodeDeleted(node_id, sender));
            Ok(())
        }

        /// Create a new Scope rooted at `node_id`, or replace its existing
        /// Scope.
        ///
        /// The caller becomes the new Scope's owner and a fresh `ScopeId` is
        /// always allocated. Authorized either by owning the Scope currently
        /// governing `node_id` (establishing a brand-new nested boundary, or
        /// replacing the Scope if `node_id` is already an active Scope
        /// root), or - only when `node_id` is already an active Scope root -
        /// by holding a non-inherited `Capability::CreateScope` grant on
        /// that exact root.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::create_scope())]
        pub fn create_scope(
            origin: OriginFor<T>,
            node_id: NodeId,
            resources: ScopeResourceList,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            ensure!(<Parent<T>>::contains_key(node_id), Error::<T>::NodeNotFound);
            Self::authorize_create_scope(&sender, node_id)?;

            let scope_id = Self::allocate_scope(node_id, sender.clone(), &resources)?;

            Self::deposit_event(Event::ScopeCreated(scope_id, node_id, sender));
            Ok(())
        }

        /// Delete the Scope boundary rooted at `node_id`.
        ///
        /// Only the Scope's owner may delete it (never through inherited
        /// `Access`). This removes only the `ActiveScope` boundary, not the
        /// CPS node or its descendants; `node_id` and everything below it
        /// falls back to the nearest parent Scope. A CPS root's Scope can
        /// never be deleted. Nested Scopes below `node_id` are unaffected.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::delete_scope())]
        pub fn delete_scope(origin: OriginFor<T>, node_id: NodeId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let scope_id = <ActiveScope<T>>::get(node_id).ok_or(Error::<T>::ScopeNotFound)?;
            let owner = <ScopeOwner<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;
            ensure!(owner == sender, Error::<T>::NotScopeOwner);

            let parent = <Parent<T>>::get(node_id).ok_or(Error::<T>::NodeNotFound)?;
            ensure!(parent.is_some(), Error::<T>::CannotDeleteRootScope);

            <ActiveScope<T>>::remove(node_id);

            Self::deposit_event(Event::ScopeDeleted(scope_id, node_id));
            Ok(())
        }

        /// Grant `capability` to `principal` at `node_id`, within the Scope
        /// resolved for `node_id`. Only the Scope's owner may grant Access.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::grant_access())]
        pub fn grant_access(
            origin: OriginFor<T>,
            node_id: NodeId,
            principal: T::AccountId,
            capability: Capability,
            inherited: bool,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let scope_id = Self::resolve_scope(node_id)?;
            let owner = <ScopeOwner<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;
            ensure!(owner == sender, Error::<T>::NotScopeOwner);

            <Access<T>>::insert(
                scope_id,
                (node_id, principal.clone(), capability),
                inherited,
            );

            Self::deposit_event(Event::AccessGranted(
                scope_id, node_id, principal, capability, inherited,
            ));
            Ok(())
        }

        /// Revoke a previously granted `capability` from `principal` at
        /// `node_id`. Only the Scope's owner may revoke Access.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::revoke_access())]
        pub fn revoke_access(
            origin: OriginFor<T>,
            node_id: NodeId,
            principal: T::AccountId,
            capability: Capability,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let scope_id = Self::resolve_scope(node_id)?;
            let owner = <ScopeOwner<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;
            ensure!(owner == sender, Error::<T>::NotScopeOwner);

            <Access<T>>::remove(scope_id, (node_id, principal.clone(), capability));

            Self::deposit_event(Event::AccessRevoked(
                scope_id, node_id, principal, capability,
            ));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Resolve the [`ScopeId`] effective for `node_id`.
        ///
        /// If `node_id` itself has an `ActiveScope` entry, that Scope is
        /// returned directly. Otherwise, `parent` links are followed one hop
        /// at a time until an active Scope is found.
        ///
        /// This is the single canonical resolver: every authorization check
        /// in this pallet uses it, and it is also what the `CpsApi` runtime
        /// API exposes. Callers that also need the Scope's root `NodeId` or
        /// owner `AccountId` can look them up from the resulting `ScopeId`
        /// via the [`ScopeRoot`] / [`ScopeOwner`] storage maps.
        pub fn resolve_scope(node_id: NodeId) -> Result<ScopeId, Error<T>> {
            ensure!(<Parent<T>>::contains_key(node_id), Error::<T>::NodeNotFound);
            let parent = <Parent<T>>::get(node_id).flatten();
            Self::resolve_scope_from(node_id, parent)
        }

        /// Same as [`Self::resolve_scope`], but reuses an already-fetched
        /// `parent` link to avoid a redundant storage read for `node_id`
        /// itself.
        fn resolve_scope_from(
            node_id: NodeId,
            parent: Option<NodeId>,
        ) -> Result<ScopeId, Error<T>> {
            if let Some(scope_id) = <ActiveScope<T>>::get(node_id) {
                return Ok(scope_id);
            }

            let mut current = parent;
            while let Some(ancestor_id) = current {
                if let Some(scope_id) = <ActiveScope<T>>::get(ancestor_id) {
                    return Ok(scope_id);
                }
                ensure!(
                    <Parent<T>>::contains_key(ancestor_id),
                    Error::<T>::NodeNotFound
                );
                current = <Parent<T>>::get(ancestor_id).flatten();
            }

            Err(Error::<T>::ScopeNotFound)
        }

        /// Authorize `sender` to exercise `capability` at `node_id`.
        ///
        /// The Scope owner always has implicit authority. Otherwise, `Access`
        /// entries are checked by walking from `node_id` up to the resolved
        /// Scope's root (inclusive): at `node_id` itself, both
        /// `inherited = false` and `inherited = true` match; on strict
        /// ancestors, only `inherited = true` matches. The walk never
        /// crosses the Scope boundary.
        fn authorize(
            sender: &T::AccountId,
            node_id: NodeId,
            capability: Capability,
        ) -> Result<ScopeId, Error<T>> {
            let scope_id = Self::resolve_scope(node_id)?;
            let owner = <ScopeOwner<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;
            if owner == *sender {
                return Ok(scope_id);
            }
            let root = <ScopeRoot<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;

            let mut current = node_id;
            let mut at_target = true;
            for _ in 0..=MAX_TREE_DEPTH {
                if let Some(inherited) =
                    <Access<T>>::get(scope_id, (current, sender.clone(), capability))
                {
                    if at_target || inherited {
                        return Ok(scope_id);
                    }
                }
                if current == root {
                    break;
                }
                current = <Parent<T>>::get(current)
                    .flatten()
                    .ok_or(Error::<T>::ScopeNotFound)?;
                at_target = false;
            }

            Err(Error::<T>::AccessDenied)
        }

        /// Authorize `sender` to call [`Pallet::create_scope`] on `node_id`.
        ///
        /// The Scope owner may create/replace a Scope on any node in their
        /// Scope. A non-owner may only do so when `node_id` is already the
        /// exact root of the resolved Scope, and only via a non-inherited
        /// `Capability::CreateScope` grant on that exact node - `CreateScope`
        /// never propagates through descendants.
        fn authorize_create_scope(
            sender: &T::AccountId,
            node_id: NodeId,
        ) -> Result<ScopeId, Error<T>> {
            let scope_id = Self::resolve_scope(node_id)?;
            let owner = <ScopeOwner<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;
            if owner == *sender {
                return Ok(scope_id);
            }

            let root = <ScopeRoot<T>>::get(scope_id).ok_or(Error::<T>::ScopeNotFound)?;
            ensure!(node_id == root, Error::<T>::AccessDenied);
            let inherited =
                <Access<T>>::get(scope_id, (node_id, sender.clone(), Capability::CreateScope));
            ensure!(inherited == Some(false), Error::<T>::AccessDenied);

            Ok(scope_id)
        }

        /// Allocate a fresh `ScopeId` rooted at `root` and owned by `owner`,
        /// with the given `resources`, and activate it. Replaces any Scope
        /// previously active at `root`.
        fn allocate_scope(
            root: NodeId,
            owner: T::AccountId,
            resources: &ScopeResourceList,
        ) -> Result<ScopeId, Error<T>> {
            let scope_id = <NextScopeId<T>>::get();
            let next_id = scope_id
                .checked_add(1)
                .ok_or(Error::<T>::ScopeIdExhausted)?;
            <NextScopeId<T>>::put(next_id);

            <ScopeRoot<T>>::insert(scope_id, root);
            <ScopeOwner<T>>::insert(scope_id, owner);
            for (resource, limit) in resources.iter() {
                <ScopeResources<T>>::insert(scope_id, resource, limit);
            }
            <ActiveScope<T>>::insert(root, scope_id);

            Ok(scope_id)
        }

        /// Count the number of ancestors of `node_id` by walking `parent`
        /// links up to the root. A root node has depth `0`.
        fn depth_of(node_id: NodeId) -> Result<u32, Error<T>> {
            ensure!(<Parent<T>>::contains_key(node_id), Error::<T>::NodeNotFound);
            let mut current = <Parent<T>>::get(node_id).flatten();
            let mut depth = 0u32;

            while let Some(ancestor_id) = current {
                depth = depth.saturating_add(1);
                ensure!(
                    <Parent<T>>::contains_key(ancestor_id),
                    Error::<T>::NodeNotFound
                );
                current = <Parent<T>>::get(ancestor_id).flatten();
            }

            Ok(depth)
        }
    }
}
