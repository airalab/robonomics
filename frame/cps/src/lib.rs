///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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
//! as a hierarchical tree structure with ownership-based access control and support
//! for both plain and encrypted data storage.
//!
//! ## Architecture
//!
//! ### Storage Layout
//!
//! The pallet uses three storage items:
//!
//! 1. **`Nodes`**: Primary storage mapping `NodeId` → `Node<AccountId, Config>`
//!    - Uses `Blake2_128Concat` hasher for cryptographic security
//!    - Each node stores its complete ancestor path for O(1) operations
//!
//! 2. **`NodesByParent`**: Index mapping `NodeId` → `BoundedVec<NodeId>`
//!    - Enables O(1) lookup of all children for a given parent
//!    - Uses `Blake2_128Concat` hasher
//!
//! 3. **`RootNodes`**: Global list of `BoundedVec<NodeId>`
//!    - Tracks all nodes without parents
//!    - Limited by `MaxRootNodes` configuration
//!
//! ### Performance Characteristics
//!
//! All core operations are O(1) time complexity:
//! - **Cycle detection**: `new_parent.path.contains(&node_id)` → O(1)
//! - **Depth validation**: `parent.path.len() < MaxTreeDepth` → O(1)
//! - **Child lookup**: Direct index access via `NodesByParent` → O(1)
//!
//! Trade-off: Requires O(depth) storage per node for path tracking, but eliminates
//! expensive recursive tree traversals during validation.
//!
//! ### Compact Encoding
//!
//! `NodeId` uses `#[codec(compact)]` attribute to enable SCALE compact encoding:
//! - Node IDs 0-63: 1 byte (87% savings vs 8 bytes)
//! - Node IDs 64-16,383: 2 bytes (75% savings)
//! - Node IDs 16,384+: 3+ bytes (62%+ savings)
//!
//! ## Usage Examples
//!
//! ### Creating a Root Node
//!
//! ```ignore
//! use pallet_robonomics_cps::{NodeData, NodeId};
//! use frame_support::BoundedVec;
//!
//! // Plain metadata
//! let meta = Some(NodeData::Plain(
//!     BoundedVec::try_from(b"sensor_config".to_vec()).unwrap()
//! ));
//!
//! // Create root (parent = None)
//! Cps::create_node(origin, None, meta, None)?;
//! ```
//!
//! ### Creating a Child Node with Encrypted Data
//!
//! ```ignore
//! use pallet_robonomics_cps::{NodeData, NodeId, CryptoAlgorithm};
//!
//! // Encrypted payload
//! let payload = Some(NodeData::Encrypted {
//!     algorithm: CryptoAlgorithm::XChaCha20Poly1305,
//!     ciphertext: BoundedVec::try_from(encrypted_bytes).unwrap(),
//! });
//!
//! // Create child under node 0
//! Cps::create_node(origin, Some(NodeId(0)), None, payload)?;
//! ```
//!
//! ### Moving Nodes with Cycle Detection
//!
//! ```ignore
//! // This will FAIL if node_id is an ancestor of new_parent_id
//! Cps::move_node(origin, NodeId(5), NodeId(10))?;
//! // Error: CycleDetected if NodeId(10) descends from NodeId(5)
//! ```
//!
//! ### Querying the Tree
//!
//! ```ignore
//! // Get a node
//! let node = Nodes::<T>::get(NodeId(0)).ok_or(Error::<T>::NodeNotFound)?;
//!
//! // Get all children
//! let children = NodesByParent::<T>::get(NodeId(0));
//!
//! // Get all root nodes
//! let roots = RootNodes::<T>::get();
//!
//! // Check if node is ancestor (O(1))
//! let is_ancestor = node.path.contains(&NodeId(ancestor_id));
//! ```
//!
//! ## Security Invariants
//!
//! The pallet maintains the following invariants:
//!
//! 1. **No Cycles**: The tree is acyclic (enforced by path checking)
//! 2. **Ownership Consistency**: Children always have parent's owner
//! 3. **Index Consistency**: `NodesByParent` and `RootNodes` stay synchronized
//! 4. **Deletion Safety**: Cannot delete nodes with children
//! 5. **Depth Limits**: Tree depth never exceeds `MaxTreeDepth`
//!
//! ## Testing
//!
//! Run the comprehensive test suite:
//!
//! ```bash
//! cargo test -p pallet-robonomics-cps
//! ```
//!
//! Tests cover:
//! - Node creation (root and children)
//! - Data updates (metadata and payload)
//! - Node movement with cycle detection
//! - Node deletion with safety checks
//! - Ownership validation
//! - Index consistency
//! - Encrypted data handling
//! - Path tracking and updates
//!
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

#[cfg(test)]
mod tests;

pub use pallet::*;
pub use weights::WeightInfo;

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
/// ensuring that the operation has completed before any side effects are triggered.
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

/// Maximum data size for node metadata, payload, and crypto profile parameters.
///
/// Set to 2048 bytes to accommodate typical sensor readings, configuration data,
/// and encrypted payloads while preventing DoS attacks via large data submissions.
pub type MaxDataSize = ConstU32<2048>;

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
///
/// # Example
///
/// ```ignore
/// let node_id = NodeId(42);  // Uses 1 byte in compact encoding
/// let next_id = node_id.saturating_add(1);  // NodeId(43)
/// ```
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
    Default,
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
    ///
    /// # Example
    ///
    /// ```ignore
    /// let id = NodeId(u64::MAX);
    /// let next = id.saturating_add(1);  // Still NodeId(u64::MAX)
    /// ```
    pub fn saturating_add(self, rhs: u64) -> Self {
        Self(self.0.saturating_add(rhs))
    }
}

/// Cryptographic algorithm identifier for encrypted data.
///
/// Currently supports XChaCha20-Poly1305, an AEAD (Authenticated Encryption with
/// Associated Data) cipher providing:
/// - 256-bit key security
/// - 192-bit nonce (no reuse concerns)
/// - Authentication tag for integrity
///
/// Additional algorithms can be added as enum variants while maintaining backward
/// compatibility with existing encrypted data.
///
/// # Example
///
/// ```ignore
/// let algo = CryptoAlgorithm::XChaCha20Poly1305;
/// ```
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
pub enum CryptoAlgorithm {
    /// XChaCha20-Poly1305 AEAD encryption.
    ///
    /// Recommended for most use cases due to:
    /// - High performance (software-optimized)
    /// - Large nonce space (192 bits prevents reuse concerns)
    /// - Strong security guarantees (256-bit keys, authenticated encryption)
    ///
    /// Resources:
    /// - RFC 8439: ChaCha20-Poly1305
    /// - XChaCha20 extends nonce to 192 bits
    XChaCha20Poly1305,
}

/// Node data container supporting both plain and encrypted storage.
///
/// This enum allows mixed privacy models within the same tree:
/// - Public metadata with encrypted payload
/// - Encrypted metadata with public payload
/// - Both encrypted or both plain
///
/// # Storage Considerations
///
/// Both variants use `BoundedVec<u8, MaxDataSize>` which:
/// - Enforces 2048-byte limit at construction time
/// - Prevents DoS attacks via oversized data
/// - Implements `MaxEncodedLen` for predictable storage costs
///
/// # Examples
///
/// ## Plain Data
///
/// ```ignore
/// let meta = NodeData::Plain(
///     BoundedVec::try_from(b"temperature: 22.5C".to_vec()).unwrap()
/// );
/// ```
///
/// ## Encrypted Data
///
/// ```ignore
/// // Assume `encrypt()` returns ciphertext bytes
/// let payload = NodeData::Encrypted {
///     algorithm: CryptoAlgorithm::XChaCha20Poly1305,
///     ciphertext: BoundedVec::try_from(encrypted_bytes).unwrap(),
/// };
/// ```
///
/// ## Mixed Privacy
///
/// ```ignore
/// // Public configuration, private operational data
/// Cps::create_node(
///     origin,
///     Some(parent_id),
///     Some(NodeData::Plain(config_bytes)),           // Public
///     Some(NodeData::Encrypted { ... })              // Private
/// )?;
/// ```
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq)]
pub enum NodeData {
    /// Plain unencrypted data visible to all.
    ///
    /// Use for:
    /// - Public system specifications
    /// - Non-sensitive configuration
    /// - Transparently verifiable data
    Plain(BoundedVec<u8, MaxDataSize>),
    
    /// Encrypted data with algorithm specification.
    ///
    /// Use for:
    /// - Sensitive operational data
    /// - Personal information (GDPR/HIPAA)
    /// - Trade secrets or proprietary information
    ///
    /// Note: Encryption/decryption happens off-chain. The pallet only stores
    /// ciphertext and algorithm identifier.
    Encrypted {
        /// Crypto algorithm used for encryption
        algorithm: CryptoAlgorithm,
        /// Encrypted ciphertext (includes nonce/tag if AEAD)
        ciphertext: BoundedVec<u8, MaxDataSize>,
    },
}

impl sp_std::fmt::Debug for NodeData {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        match self {
            Self::Plain(vec) => f.debug_tuple("Plain").field(&vec.len()).finish(),
            Self::Encrypted {
                algorithm,
                ciphertext,
            } => f
                .debug_struct("Encrypted")
                .field("algorithm", algorithm)
                .field("ciphertext_len", &ciphertext.len())
                .finish(),
        }
    }
}

/// Node structure representing a cyber-physical system in the tree.
///
/// Each node maintains:
/// 1. **Parent link** (`Option<NodeId>`): None for root nodes
/// 2. **Owner** (`AccountId`): Who controls this node and its subtree
/// 3. **Path** (`BoundedVec<NodeId>`): Complete ancestor chain for O(1) operations
/// 4. **Metadata** (`Option<NodeData>`): Configuration, specifications, capabilities
/// 5. **Payload** (`Option<NodeData>`): Operational data, sensor readings, telemetry
///
/// # Path-Based Performance
///
/// The `path` field stores all ancestor node IDs from root to parent:
///
/// ```text
/// Tree:        Node A (root)
///                 |
///              Node B         <- path: [A]
///                 |
///              Node C         <- path: [A, B]
/// ```
///
/// This enables:
/// - **O(1) Cycle Detection**: `path.contains(&node_id)` checks if moving would create a cycle
/// - **O(1) Depth Check**: `path.len()` returns current depth without traversal
/// - **O(1) Ancestor Test**: Direct lookup in path vector
///
/// Trade-off: Uses O(depth) storage per node, but eliminates expensive recursive operations.
///
/// # Examples
///
/// ## Creating a Root Node
///
/// ```ignore
/// let root = Node {
///     parent: None,
///     owner: account_id,
///     path: BoundedVec::default(),  // Empty for root
///     meta: Some(NodeData::Plain(...)),
///     payload: None,
/// };
/// ```
///
/// ## Creating a Child Node
///
/// ```ignore
/// let parent_node = Nodes::<T>::get(parent_id).unwrap();
///
/// // Build path: parent's path + parent's ID
/// let mut child_path = parent_node.path.clone();
/// child_path.try_push(parent_id).map_err(|_| Error::<T>::MaxDepthExceeded)?;
///
/// let child = Node {
///     parent: Some(parent_id),
///     owner: parent_node.owner.clone(),  // Inherit owner
///     path: child_path,
///     meta: Some(NodeData::Plain(...)),
///     payload: Some(NodeData::Encrypted { ... }),
/// };
/// ```
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct Node<AccountId: MaxEncodedLen, T: Config> {
    /// Parent node ID (None for root nodes)
    pub parent: Option<NodeId>,
    /// Node owner
    pub owner: AccountId,
    /// Complete path from root to this node (includes all ancestor IDs in order)
    /// NodeId uses compact encoding for efficient storage
    pub path: BoundedVec<NodeId, T::MaxTreeDepth>,
    /// Metadata
    pub meta: Option<NodeData>,
    /// Payload data
    pub payload: Option<NodeData>,
}

impl<AccountId: MaxEncodedLen + sp_std::fmt::Debug, T: Config> sp_std::fmt::Debug
    for Node<AccountId, T>
{
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        f.debug_struct("Node")
            .field("parent", &self.parent)
            .field("owner", &self.owner)
            .field("path", &self.path)
            .field("meta", &self.meta)
            .field("payload", &self.payload)
            .finish()
    }
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

        /// Maximum tree depth
        #[pallet::constant]
        type MaxTreeDepth: Get<u32>;

        /// Maximum children per node
        #[pallet::constant]
        type MaxChildrenPerNode: Get<u32>;

        /// Maximum root nodes
        #[pallet::constant]
        type MaxRootNodes: Get<u32>;

        /// Callback handler invoked when a payload is set on a node.
        ///
        /// Use `()` for no callback, or implement the `OnPayloadSet` trait
        /// for custom runtime-level hooks. Multiple handlers can be combined
        /// using tuples: `(HandlerA, HandlerB)`.
        ///
        /// The callback receives:
        /// - The node ID that was updated
        /// - The current metadata of the node
        /// - The new payload that was set
        ///
        /// # Example
        ///
        /// ```ignore
        /// type OnPayloadSet = (); // No callback
        /// type OnPayloadSet = MyCustomHandler; // Single handler
        /// type OnPayloadSet = (HandlerA, HandlerB); // Multiple handlers
        /// ```
        type OnPayloadSet: OnPayloadSet<Self::AccountId>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    /// Storage version for migrations
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    /// Next node ID counter
    #[pallet::storage]
    #[pallet::getter(fn next_node_id)]
    pub type NextNodeId<T> = StorageValue<_, NodeId, ValueQuery>;

    /// Nodes storage
    #[pallet::storage]
    #[pallet::getter(fn nodes)]
    pub type Nodes<T: Config> = StorageMap<_, Blake2_128Concat, NodeId, Node<T::AccountId, T>>;

    /// Index of children by parent node
    #[pallet::storage]
    #[pallet::getter(fn nodes_by_parent)]
    pub type NodesByParent<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        NodeId,
        BoundedVec<NodeId, T::MaxChildrenPerNode>,
        ValueQuery,
    >;

    /// Root nodes (nodes without parents)
    #[pallet::storage]
    #[pallet::getter(fn root_nodes)]
    pub type RootNodes<T: Config> =
        StorageValue<_, BoundedVec<NodeId, T::MaxRootNodes>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Node created [node_id, parent_id, owner]
        NodeCreated(NodeId, Option<NodeId>, T::AccountId),
        /// Node metadata set [node_id, owner]
        MetaSet(NodeId, T::AccountId),
        /// Node payload set [node_id, owner]
        PayloadSet(NodeId, T::AccountId),
        /// Node moved [node_id, old_parent, new_parent, owner]
        NodeMoved(NodeId, Option<NodeId>, NodeId, T::AccountId),
        /// Node deleted [node_id, owner]
        NodeDeleted(NodeId, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Node not found
        NodeNotFound,
        /// Parent node not found
        ParentNotFound,
        /// Caller is not the node owner
        NotNodeOwner,
        /// Owner mismatch between parent and child
        OwnerMismatch,
        /// Cycle detected in tree structure
        CycleDetected,
        /// Maximum tree depth exceeded
        MaxDepthExceeded,
        /// Too many children for node
        TooManyChildren,
        /// Too many root nodes
        TooManyRootNodes,
        /// Node has children and cannot be deleted
        NodeHasChildren,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new node
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_node())]
        pub fn create_node(
            origin: OriginFor<T>,
            parent_id: Option<NodeId>,
            meta: Option<NodeData>,
            payload: Option<NodeData>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Get new node ID
            let node_id = <NextNodeId<T>>::get();
            <NextNodeId<T>>::put(node_id.saturating_add(1));

            // Build path based on parent
            let path = if let Some(pid) = parent_id {
                let parent = <Nodes<T>>::get(pid).ok_or(Error::<T>::ParentNotFound)?;
                ensure!(parent.owner == sender, Error::<T>::OwnerMismatch);

                // Check tree depth - path already includes all ancestors
                ensure!(
                    parent.path.len() < T::MaxTreeDepth::get() as usize,
                    Error::<T>::MaxDepthExceeded
                );

                // Build new path by extending parent's path
                let mut new_path = parent.path.clone();
                new_path
                    .try_push(pid)
                    .map_err(|_| Error::<T>::MaxDepthExceeded)?;

                // Add to parent's children index
                <NodesByParent<T>>::try_mutate(pid, |children| {
                    children
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyChildren)
                })?;

                new_path
            } else {
                // Root node has empty path
                <RootNodes<T>>::try_mutate(|roots| {
                    roots
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyRootNodes)
                })?;

                BoundedVec::default()
            };

            // Create node
            let node = Node {
                parent: parent_id,
                owner: sender.clone(),
                path,
                meta,
                payload,
            };

            // Store node
            <Nodes<T>>::insert(node_id, node);

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

            // Update node
            <Nodes<T>>::try_mutate(node_id, |node_opt| {
                let node = node_opt.as_mut().ok_or(Error::<T>::NodeNotFound)?;
                ensure!(node.owner == sender, Error::<T>::NotNodeOwner);
                node.meta = meta;
                Ok::<(), DispatchError>(())
            })?;

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

            // Capture metadata and new payload for callback
            let (meta, new_payload) = <Nodes<T>>::try_mutate(node_id, |node_opt| {
                let node = node_opt.as_mut().ok_or(Error::<T>::NodeNotFound)?;
                ensure!(node.owner == sender, Error::<T>::NotNodeOwner);
                let meta = node.meta.clone();
                node.payload = payload.clone();
                Ok::<(Option<NodeData>, Option<NodeData>), DispatchError>((meta, payload))
            })?;

            Self::deposit_event(Event::PayloadSet(node_id, sender));

            // Invoke the callback after successful payload update
            T::OnPayloadSet::on_payload_set(node_id, meta, new_payload);

            Ok(())
        }

        /// Move node to a new parent
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::move_node())]
        pub fn move_node(
            origin: OriginFor<T>,
            node_id: NodeId,
            new_parent_id: NodeId,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Get node and new parent
            let node = <Nodes<T>>::get(node_id).ok_or(Error::<T>::NodeNotFound)?;
            let new_parent = <Nodes<T>>::get(new_parent_id).ok_or(Error::<T>::ParentNotFound)?;

            // Verify ownership
            ensure!(node.owner == sender, Error::<T>::NotNodeOwner);
            ensure!(new_parent.owner == sender, Error::<T>::OwnerMismatch);

            // Check for cycles - node_id cannot be an ancestor of new_parent
            // If node_id is in new_parent's path, moving node under new_parent would create a cycle
            ensure!(
                !new_parent.path.contains(&node_id),
                Error::<T>::CycleDetected
            );

            // Check tree depth after move
            ensure!(
                new_parent.path.len() < T::MaxTreeDepth::get() as usize,
                Error::<T>::MaxDepthExceeded
            );

            let old_parent = node.parent;

            // Update parent-child indexes
            if let Some(old_pid) = old_parent {
                // Remove from old parent's children
                <NodesByParent<T>>::mutate(old_pid, |children| {
                    children.retain(|&id| id != node_id);
                });
            } else {
                // Remove from root nodes
                <RootNodes<T>>::mutate(|roots| {
                    roots.retain(|&id| id != node_id);
                });
            }

            // Add to new parent's children
            <NodesByParent<T>>::try_mutate(new_parent_id, |children| {
                children
                    .try_push(node_id)
                    .map_err(|_| Error::<T>::TooManyChildren)
            })?;

            // Build new path
            let mut new_path = new_parent.path.clone();
            new_path
                .try_push(new_parent_id)
                .map_err(|_| Error::<T>::MaxDepthExceeded)?;

            // Update node's parent and path
            <Nodes<T>>::mutate(node_id, |node_opt| {
                if let Some(node) = node_opt {
                    node.parent = Some(new_parent_id);
                    node.path = new_path.clone();
                }
            });

            // Recursively update all descendant paths
            Self::update_descendant_paths(node_id, &new_path)?;

            Self::deposit_event(Event::NodeMoved(node_id, old_parent, new_parent_id, sender));
            Ok(())
        }

        /// Delete a node
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::delete_node())]
        pub fn delete_node(origin: OriginFor<T>, node_id: NodeId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Get the node
            let node = <Nodes<T>>::get(node_id).ok_or(Error::<T>::NodeNotFound)?;

            // Verify ownership
            ensure!(node.owner == sender, Error::<T>::NotNodeOwner);

            // Check if node has children
            let children = <NodesByParent<T>>::get(node_id);
            ensure!(children.is_empty(), Error::<T>::NodeHasChildren);

            // Remove from parent's children index
            if let Some(parent_id) = node.parent {
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

            // Remove the node itself
            <Nodes<T>>::remove(node_id);

            Self::deposit_event(Event::NodeDeleted(node_id, sender));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Recursively update paths of all descendant nodes
        fn update_descendant_paths(
            parent_id: NodeId,
            parent_path: &BoundedVec<NodeId, T::MaxTreeDepth>,
        ) -> DispatchResult {
            let children = <NodesByParent<T>>::get(parent_id);

            for child_id in children.iter() {
                // Build new path for child
                let mut new_path = parent_path.clone();
                new_path
                    .try_push(parent_id)
                    .map_err(|_| Error::<T>::MaxDepthExceeded)?;

                // Update child's path
                <Nodes<T>>::mutate(child_id, |node_opt| {
                    if let Some(node) = node_opt {
                        node.path = new_path.clone();
                    }
                });

                // Recursively update descendants
                Self::update_descendant_paths(*child_id, &new_path)?;
            }

            Ok(())
        }
    }
}
