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
//! as a hierarchical tree structure with ownership-based access control.
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
//! 3. **`Ownership`**: Mapping `NodeId` → `AccountId`
//!    - An entry is only present on nodes that start a new administrative and
//!      resource-accounting scope (an "Ownership boundary")
//!    - A node without an entry inherits ownership from the nearest ancestor
//!      that has one (see [`Pallet::resolve_ownership`])
//!
//! 4. **`PendingOwnershipTransfer`**: Mapping `NodeId` → `PendingTransfer`
//!    - Stores the proposed owner together with the authorizing boundary and
//!      `OwnershipGeneration`, preventing stale acceptance
//!
//! 5. **`NodesByParent`** / **`RootNodes`**: Index structures for O(1) child and
//!    root-node lookups
//!
//! ### Ownership Model
//!
//! An explicit `Ownership` association is attached only to nodes that start a
//! new administrative and resource-accounting scope; all descendants inherit
//! it until another `Ownership` association is encountered:
//!
//! ```text
//! Building-A
//! Ownership(PropertyManager)
//! |
//! `-- Floor-3
//!     Ownership(TenantCorp)
//!     |
//!     `-- HVAC-Unit-07
//!         |
//!         `-- Thermostat-142
//! ```
//!
//! `HVAC-Unit-07` and `Thermostat-142` inherit ownership from `Floor-3`'s
//! `Ownership(TenantCorp)` boundary; `TenantCorp` has no implicit
//! administrative rights over `Building-A`'s other independently owned
//! floors, and `PropertyManager` has no implicit administrative rights
//! inside `TenantCorp`'s boundary.
//!
//! [`Pallet::resolve_ownership`] is the single canonical resolver: it returns
//! the boundary root `NodeId` and effective owner `AccountId` for any node,
//! and is used by every authorization check in this pallet (and is intended to
//! be used by future resource-accounting pallets such as Subscription,
//! Storage, and Compute).
//!
//! ### Ownership Transfer
//!
//! Ownership of a boundary is established through a two-step propose/accept
//! flow:
//!
//! 1. The current effective owner calls [`Pallet::transfer_ownership`] with the
//!    target node and the proposed new owner.
//! 2. The proposed new owner calls [`Pallet::accept_ownership`] to finalize the
//!    transfer, which writes (or overwrites) the explicit `Ownership` entry for
//!    that node.
//!
//! A node can be turned into a new, independent boundary without changing the
//! effective owner account by "self-transferring" it (propose and accept with
//! the same account).
//!
//! Accepting a transfer increments the boundary's generation. Proposals authorized
//! by an older generation or a different boundary must be proposed again by the
//! current owner; transferring ownership back does not revive old proposals.
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
//! - **Ownership resolution**: `resolve_ownership` walks `parent` links one
//!   hop at a time until an explicit `Ownership` entry is found → O(depth)
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
//! // Create root (parent = None) - the caller becomes the explicit owner of this
//! // new boundary.
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
//! // Create child under node 0. The child inherits ownership from node 0's
//! // resolved boundary; the caller must be that resolved owner.
//! Cps::create_node(origin, Some(NodeId(0)), None, payload)?;
//! ```
//!
//! ### Establishing a Nested Ownership Boundary
//!
//! ```ignore
//! // Node 5 currently inherits ownership from an ancestor. Its resolved owner
//! // proposes a new boundary owner (can be a different account, or the same
//! // account to simply carve out a new administrative/accounting scope).
//! Cps::transfer_ownership(origin, NodeId(5), new_owner.clone())?;
//!
//! // The proposed owner accepts, which writes the explicit Ownership entry.
//! Cps::accept_ownership(RuntimeOrigin::signed(new_owner), NodeId(5))?;
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
//! // Resolve the effective ownership boundary and owner for a node
//! let (root, owner) = Cps::resolve_ownership(NodeId(0))?;
//! ```
//!
//! ## Security Invariants
//!
//! The pallet maintains the following invariants:
//!
//! 1. **No Cycles**: The tree is acyclic and `parent` is immutable, so cycles
//!    cannot be created after node creation.
//! 2. **Ownership Resolution**: Every active node resolves to exactly one
//!    effective owner via [`Pallet::resolve_ownership`].
//! 3. **Ownership Boundaries**: An explicit `Ownership` entry stops
//!    inheritance from ancestors; ancestor owners have no implicit
//!    administrative rights inside a nested boundary.
//! 4. **Index Consistency**: `NodesByParent` and `RootNodes` stay synchronized
//!    with `Parent`.
//! 5. **Deletion Safety**: Cannot delete nodes with children.
//! 6. **Depth Limits**: Tree depth never exceeds `MAX_TREE_DEPTH`.
//! 7. **Stable Identity**: `NodeId` is never reused.
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
/// cost of `resolve_ownership` and depth validation, both O(depth).
pub const MAX_TREE_DEPTH: u32 = 32;

/// Maximum number of direct children a single node may have.
///
/// Bounds the size of the `NodesByParent` index entry for any given node.
pub const MAX_CHILDREN_PER_NODE: u32 = 100;

/// Maximum number of root nodes (nodes with no parent) that may exist.
///
/// Bounds the size of the `RootNodes` index.
pub const MAX_ROOT_NODES: u32 = 100;

/// [`ConstU32`] wrapper around [`MAX_DATA_SIZE`] for use as a `BoundedVec` bound.
pub type MaxDataSize = ConstU32<MAX_DATA_SIZE>;
/// [`ConstU32`] wrapper around [`MAX_TREE_DEPTH`] for use as a `BoundedVec` bound.
pub type MaxTreeDepth = ConstU32<MAX_TREE_DEPTH>;
/// [`ConstU32`] wrapper around [`MAX_CHILDREN_PER_NODE`] for use as a `BoundedVec` bound.
pub type MaxChildrenPerNode = ConstU32<MAX_CHILDREN_PER_NODE>;
/// [`ConstU32`] wrapper around [`MAX_ROOT_NODES`] for use as a `BoundedVec` bound.
pub type MaxRootNodes = ConstU32<MAX_ROOT_NODES>;

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

/// Proposed owner and the ownership scope that authorized the transfer.
///
/// Acceptance requires the same boundary and generation, so changes in inherited
/// ownership invalidate the proposal without traversing the boundary's subtree.
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq, Debug,
)]
pub struct PendingTransfer<AccountId> {
    /// Account permitted to accept this proposal.
    pub proposed_owner: AccountId,
    /// Effective ownership boundary at proposal time.
    pub boundary: NodeId,
    /// Boundary generation at proposal time.
    pub generation: u64,
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

        /// Callback handler invoked when a payload is set on a node.
        ///
        /// Use `()` for no callback, or implement the `OnPayloadSet` trait
        /// for custom runtime-level hooks. Multiple handlers can be combined
        /// using tuples: `(HandlerA, HandlerB)`.
        type OnPayloadSet: OnPayloadSet<Self::AccountId>;

        /// Weight information for extrinsics
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

    /// Explicit ownership boundaries.
    ///
    /// An entry is present only on nodes that start a new administrative and
    /// resource-accounting scope. Nodes without an entry inherit ownership
    /// from the nearest ancestor that has one - see
    /// [`Pallet::resolve_ownership`].
    #[pallet::storage]
    #[pallet::getter(fn ownership_of)]
    pub type Ownership<T: Config> = StorageMap<_, Blake2_128Concat, NodeId, T::AccountId>;

    /// Proposed, not-yet-accepted ownership transfers.
    #[pallet::storage]
    #[pallet::getter(fn pending_ownership_transfer)]
    pub type PendingOwnershipTransfer<T: Config> =
        StorageMap<_, Blake2_128Concat, NodeId, PendingTransfer<T::AccountId>>;

    /// Generation of an explicit boundary, incremented on every accepted transfer.
    ///
    /// Newly created and migrated boundaries start at zero. Generations never
    /// wrap, so transferring back to a former owner cannot revive old proposals.
    #[pallet::storage]
    #[pallet::getter(fn ownership_generation)]
    pub type OwnershipGeneration<T: Config> =
        StorageMap<_, Blake2_128Concat, NodeId, u64, ValueQuery>;

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
        /// Node metadata set [node_id, owner]
        MetaSet(NodeId, T::AccountId),
        /// Node payload set [node_id, owner]
        PayloadSet(NodeId, T::AccountId),
        /// Node deleted [node_id, owner]
        NodeDeleted(NodeId, T::AccountId),
        /// Ownership transfer proposed [node_id, current_owner, proposed_owner]
        OwnershipTransferProposed(NodeId, T::AccountId, T::AccountId),
        /// Ownership transfer accepted, establishing an explicit boundary
        /// [node_id, new_owner]
        OwnershipTransferred(NodeId, T::AccountId),
    }

    #[pallet::error]
    #[derive(PartialEq)]
    pub enum Error<T> {
        /// Node not found
        NodeNotFound,
        /// Parent node not found
        ParentNotFound,
        /// Caller is not the resolved owner of the node
        NotNodeOwner,
        /// Maximum tree depth exceeded
        MaxDepthExceeded,
        /// Too many children for node
        TooManyChildren,
        /// Too many root nodes
        TooManyRootNodes,
        /// Node has children and cannot be deleted
        NodeHasChildren,
        /// No explicit or inherited Ownership could be resolved for this node
        OwnershipNotFound,
        /// There is no pending ownership transfer for this node
        NoPendingOwnershipTransfer,
        /// Caller is not the account proposed in the pending ownership transfer
        NotProposedOwner,
        /// No fresh node ID can be allocated without overflowing the counter
        NodeIdExhausted,
        /// The ownership boundary authorizing this proposal has changed
        StaleOwnershipTransfer,
        /// The ownership generation cannot be incremented without overflow
        OwnershipGenerationExhausted,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new node.
        ///
        /// Creating a root node (`parent_id: None`) makes the caller the
        /// explicit owner of a new Ownership boundary. Creating a child node
        /// requires the caller to be the resolved owner of `parent_id`; the
        /// child inherits ownership and does not get its own explicit entry.
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
                let parent_parent = <Parent<T>>::get(pid).flatten();
                let (_, owner) = Self::resolve_ownership_from(pid, parent_parent)?;
                ensure!(owner == sender, Error::<T>::NotNodeOwner);

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
                // Root node is always an explicit Ownership boundary
                <RootNodes<T>>::try_mutate(|roots| {
                    roots
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyRootNodes)
                })?;

                <Ownership<T>>::insert(node_id, sender.clone());
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

            let (_, owner) = Self::resolve_ownership(node_id)?;
            ensure!(owner == sender, Error::<T>::NotNodeOwner);

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

            let (_, owner) = Self::resolve_ownership(node_id)?;
            ensure!(owner == sender, Error::<T>::NotNodeOwner);

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
        /// Only leaf nodes (no children) can be deleted. Any explicit
        /// Ownership entry / pending transfer attached to the node is removed
        /// as well.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::delete_node())]
        pub fn delete_node(origin: OriginFor<T>, node_id: NodeId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Check the node exists, and get its parent
            ensure!(<Parent<T>>::contains_key(node_id), Error::<T>::NodeNotFound);
            let parent = <Parent<T>>::get(node_id).flatten();

            // Verify ownership
            let (_, owner) = Self::resolve_ownership_from(node_id, parent)?;
            ensure!(owner == sender, Error::<T>::NotNodeOwner);

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

            // Clean up any ownership state attached to this node
            <Ownership<T>>::remove(node_id);
            <PendingOwnershipTransfer<T>>::remove(node_id);
            <OwnershipGeneration<T>>::remove(node_id);

            // Remove the node's attributes
            <Meta<T>>::remove(node_id);
            <Payload<T>>::remove(node_id);
            <Parent<T>>::remove(node_id);

            Self::deposit_event(Event::NodeDeleted(node_id, sender));
            Ok(())
        }

        /// Propose transferring the Ownership boundary of `node_id` to
        /// `new_owner`.
        ///
        /// The caller must be the resolved owner of `node_id`. This can be
        /// used both to transfer an existing boundary to another account and
        /// to establish a brand-new boundary on a node that currently
        /// inherits ownership (including "self-transfers", where
        /// `new_owner == caller`, to carve out a boundary without changing
        /// the effective owner).
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::transfer_ownership())]
        pub fn transfer_ownership(
            origin: OriginFor<T>,
            node_id: NodeId,
            new_owner: T::AccountId,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let (boundary, owner) = Self::resolve_ownership(node_id)?;
            ensure!(owner == sender, Error::<T>::NotNodeOwner);

            <PendingOwnershipTransfer<T>>::insert(
                node_id,
                PendingTransfer {
                    proposed_owner: new_owner.clone(),
                    boundary,
                    generation: <OwnershipGeneration<T>>::get(boundary),
                },
            );

            Self::deposit_event(Event::OwnershipTransferProposed(node_id, owner, new_owner));
            Ok(())
        }

        /// Accept a pending ownership transfer for `node_id`, establishing (or
        /// overwriting) its explicit Ownership entry.
        ///
        /// The authorizing boundary and generation must still match the proposal.
        /// Stale proposals can only be replaced by the current effective owner.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::accept_ownership())]
        pub fn accept_ownership(origin: OriginFor<T>, node_id: NodeId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let proposal = <PendingOwnershipTransfer<T>>::take(node_id)
                .ok_or(Error::<T>::NoPendingOwnershipTransfer)?;
            ensure!(
                proposal.proposed_owner == sender,
                Error::<T>::NotProposedOwner
            );

            let (boundary, _) = Self::resolve_ownership(node_id)?;
            ensure!(
                proposal.boundary == boundary
                    && proposal.generation == <OwnershipGeneration<T>>::get(boundary),
                Error::<T>::StaleOwnershipTransfer
            );
            let generation = <OwnershipGeneration<T>>::get(node_id)
                .checked_add(1)
                .ok_or(Error::<T>::OwnershipGenerationExhausted)?;
            <OwnershipGeneration<T>>::insert(node_id, generation);
            <Ownership<T>>::insert(node_id, sender.clone());

            Self::deposit_event(Event::OwnershipTransferred(node_id, sender));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Resolve the effective Ownership boundary and owner for `node_id`.
        ///
        /// If `node_id` itself has an explicit `Ownership` entry, it is
        /// returned directly. Otherwise, `parent` links are followed one hop
        /// at a time until an explicit entry is found.
        ///
        /// This is the single canonical resolver: every authorization check
        /// in this pallet uses it, and future resource-accounting pallets
        /// (Subscription, Storage, Compute) are expected to use it as well.
        pub fn resolve_ownership(node_id: NodeId) -> Result<(NodeId, T::AccountId), Error<T>> {
            ensure!(<Parent<T>>::contains_key(node_id), Error::<T>::NodeNotFound);
            let parent = <Parent<T>>::get(node_id).flatten();
            Self::resolve_ownership_from(node_id, parent)
        }

        /// Same as [`Self::resolve_ownership`], but reuses an already-fetched
        /// `parent` link to avoid a redundant storage read for `node_id`
        /// itself.
        fn resolve_ownership_from(
            node_id: NodeId,
            parent: Option<NodeId>,
        ) -> Result<(NodeId, T::AccountId), Error<T>> {
            if let Some(owner) = <Ownership<T>>::get(node_id) {
                return Ok((node_id, owner));
            }

            let mut current = parent;
            while let Some(ancestor_id) = current {
                if let Some(owner) = <Ownership<T>>::get(ancestor_id) {
                    return Ok((ancestor_id, owner));
                }
                ensure!(
                    <Parent<T>>::contains_key(ancestor_id),
                    Error::<T>::NodeNotFound
                );
                current = <Parent<T>>::get(ancestor_id).flatten();
            }

            Err(Error::<T>::OwnershipNotFound)
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
