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
//! Cyber-Physical System pallet. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

#[cfg(test)]
mod tests;

pub use pallet::*;
pub use weights::WeightInfo;

use frame_support::traits::{Get, StorageVersion};
use frame_support::{traits::ConstU32, BoundedVec};
use parity_scale_codec::{Compact, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Maximum data size for node metadata, payload, and crypto profile parameters
pub type MaxDataSize = ConstU32<2048>;

/// Node identifier type
pub type NodeId = u64;

/// Compact-encoded node path for efficient storage
/// Wraps BoundedVec<NodeId> with compact encoding for each element
#[derive(PartialEq, Eq, TypeInfo, MaxEncodedLen, DecodeWithMemTracking)]
#[scale_info(skip_type_params(S))]
pub struct CompactNodePath<S: Get<u32>> {
    inner: BoundedVec<NodeId, S>,
}

impl<S: Get<u32>> CompactNodePath<S> {
    /// Create a new empty path
    pub fn new() -> Self {
        Self {
            inner: BoundedVec::default(),
        }
    }

    /// Create from a BoundedVec
    pub fn from_bounded_vec(vec: BoundedVec<NodeId, S>) -> Self {
        Self { inner: vec }
    }

    /// Get the inner BoundedVec
    pub fn as_bounded_vec(&self) -> &BoundedVec<NodeId, S> {
        &self.inner
    }

    /// Check if path contains a node ID
    pub fn contains(&self, id: &NodeId) -> bool {
        self.inner.contains(id)
    }

    /// Get the length of the path
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if path is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Try to push a node ID to the path
    pub fn try_push(&mut self, id: NodeId) -> Result<(), ()> {
        self.inner.try_push(id).map_err(|_| ())
    }

    /// Iterate over the path
    pub fn iter(&self) -> sp_std::slice::Iter<NodeId> {
        self.inner.iter()
    }

    /// Get a slice of the path
    pub fn as_slice(&self) -> &[NodeId] {
        self.inner.as_slice()
    }
}

impl<S: Get<u32>> sp_std::ops::Index<usize> for CompactNodePath<S> {
    type Output = NodeId;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

impl<S: Get<u32>> Default for CompactNodePath<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Get<u32>> Clone for CompactNodePath<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<S: Get<u32>> sp_std::fmt::Debug for CompactNodePath<S> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        self.inner.fmt(f)
    }
}

// Custom SCALE encoding using Compact for each NodeId
impl<S: Get<u32>> Encode for CompactNodePath<S> {
    fn size_hint(&self) -> usize {
        // Compact length + compact elements
        Compact(self.inner.len() as u32).size_hint()
            + self
                .inner
                .iter()
                .map(|id| Compact(*id).size_hint())
                .sum::<usize>()
    }

    fn encode_to<T: parity_scale_codec::Output + ?Sized>(&self, dest: &mut T) {
        // Encode length as compact
        Compact(self.inner.len() as u32).encode_to(dest);
        // Encode each element as compact
        for id in self.inner.iter() {
            Compact(*id).encode_to(dest);
        }
    }
}

// Custom SCALE decoding using Compact for each NodeId
impl<S: Get<u32>> Decode for CompactNodePath<S> {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let len = <Compact<u32>>::decode(input)?.0 as usize;
        let mut inner = BoundedVec::default();
        for _ in 0..len {
            let id = <Compact<u64>>::decode(input)?.0;
            inner
                .try_push(id)
                .map_err(|_| parity_scale_codec::Error::from("Path length exceeded"))?;
        }
        Ok(Self { inner })
    }
}

/// Crypto algorithm enum for encryption
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
    /// XChaCha20-Poly1305 AEAD encryption
    XChaCha20Poly1305,
}

/// Node data that can be either plain or encrypted
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq)]
pub enum NodeData {
    /// Plain unencrypted data
    Plain(BoundedVec<u8, MaxDataSize>),
    /// Encrypted data with crypto algorithm
    Encrypted {
        /// Crypto algorithm used for encryption
        algorithm: CryptoAlgorithm,
        /// Encrypted ciphertext
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

/// Node structure representing a CPS node
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Clone, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct Node<AccountId: MaxEncodedLen, T: Config> {
    /// Parent node ID (None for root nodes)
    pub parent: Option<NodeId>,
    /// Node owner
    pub owner: AccountId,
    /// Complete path from root to this node (includes all ancestor IDs in order)
    /// Uses compact encoding for efficient storage
    pub path: CompactNodePath<T::MaxTreeDepth>,
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

                CompactNodePath::default()
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

            // Update node
            <Nodes<T>>::try_mutate(node_id, |node_opt| {
                let node = node_opt.as_mut().ok_or(Error::<T>::NodeNotFound)?;
                ensure!(node.owner == sender, Error::<T>::NotNodeOwner);
                node.payload = payload;
                Ok::<(), DispatchError>(())
            })?;

            Self::deposit_event(Event::PayloadSet(node_id, sender));
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
            parent_path: &CompactNodePath<T::MaxTreeDepth>,
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
