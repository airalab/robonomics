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

use frame_support::{traits::Get, BoundedVec};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Node identifier type
pub type NodeId = u64;

/// Crypto profile identifier type
pub type CryptoProfileId = u64;

/// Algorithm identifier type
pub type AlgorithmId = u64;

/// Node data that can be either plain or encrypted
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxSize))]
#[codec(mel_bound())]
pub enum NodeData<MaxSize: Get<u32>> {
    /// Plain unencrypted data
    Plain(BoundedVec<u8, MaxSize>),
    /// Encrypted data with crypto profile reference
    Encrypted {
        /// Crypto profile ID used for encryption
        crypto_profile: CryptoProfileId,
        /// Encrypted ciphertext
        ciphertext: BoundedVec<u8, MaxSize>,
    },
}

impl<MaxSize: Get<u32>> sp_std::fmt::Debug for NodeData<MaxSize> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        match self {
            Self::Plain(vec) => f.debug_tuple("Plain").field(&vec.len()).finish(),
            Self::Encrypted {
                crypto_profile,
                ciphertext,
            } => f
                .debug_struct("Encrypted")
                .field("crypto_profile", crypto_profile)
                .field("ciphertext_len", &ciphertext.len())
                .finish(),
        }
    }
}

impl<MaxSize: Get<u32>> PartialEq for NodeData<MaxSize> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Plain(a), Self::Plain(b)) => a == b,
            (
                Self::Encrypted {
                    crypto_profile: cp1,
                    ciphertext: ct1,
                },
                Self::Encrypted {
                    crypto_profile: cp2,
                    ciphertext: ct2,
                },
            ) => cp1 == cp2 && ct1 == ct2,
            _ => false,
        }
    }
}

impl<MaxSize: Get<u32>> Eq for NodeData<MaxSize> {}

impl<MaxSize: Get<u32>> Clone for NodeData<MaxSize> {
    fn clone(&self) -> Self {
        match self {
            Self::Plain(vec) => Self::Plain(vec.clone()),
            Self::Encrypted {
                crypto_profile,
                ciphertext,
            } => Self::Encrypted {
                crypto_profile: *crypto_profile,
                ciphertext: ciphertext.clone(),
            },
        }
    }
}

/// Crypto profile for encryption
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxSize))]
#[codec(mel_bound())]
pub struct CryptoProfile<MaxSize: Get<u32>> {
    /// Profile ID
    pub id: CryptoProfileId,
    /// Algorithm identifier
    pub algorithm: AlgorithmId,
    /// Public parameters for the crypto algorithm
    pub public_params: BoundedVec<u8, MaxSize>,
}

impl<MaxSize: Get<u32>> sp_std::fmt::Debug for CryptoProfile<MaxSize> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        f.debug_struct("CryptoProfile")
            .field("id", &self.id)
            .field("algorithm", &self.algorithm)
            .field("public_params_len", &self.public_params.len())
            .finish()
    }
}

impl<MaxSize: Get<u32>> PartialEq for CryptoProfile<MaxSize> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.algorithm == other.algorithm
            && self.public_params == other.public_params
    }
}

impl<MaxSize: Get<u32>> Eq for CryptoProfile<MaxSize> {}

impl<MaxSize: Get<u32>> Clone for CryptoProfile<MaxSize> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            algorithm: self.algorithm,
            public_params: self.public_params.clone(),
        }
    }
}

/// Node structure representing a CPS node
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxSize))]
#[codec(mel_bound())]
pub struct Node<AccountId: MaxEncodedLen, MaxSize: Get<u32>> {
    /// Node ID
    pub id: NodeId,
    /// Parent node ID (None for root nodes)
    pub parent: Option<NodeId>,
    /// Node owner
    pub owner: AccountId,
    /// Metadata
    pub meta: Option<NodeData<MaxSize>>,
    /// Payload data
    pub payload: Option<NodeData<MaxSize>>,
}

impl<AccountId: MaxEncodedLen + sp_std::fmt::Debug, MaxSize: Get<u32>> sp_std::fmt::Debug
    for Node<AccountId, MaxSize>
{
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("parent", &self.parent)
            .field("owner", &self.owner)
            .field("meta", &self.meta)
            .field("payload", &self.payload)
            .finish()
    }
}

impl<AccountId: MaxEncodedLen, MaxSize: Get<u32>> PartialEq for Node<AccountId, MaxSize>
where
    AccountId: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.parent == other.parent
            && self.owner == other.owner
            && self.meta == other.meta
            && self.payload == other.payload
    }
}

impl<AccountId: MaxEncodedLen, MaxSize: Get<u32>> Eq for Node<AccountId, MaxSize> where AccountId: Eq
{}

impl<AccountId: MaxEncodedLen, MaxSize: Get<u32>> Clone for Node<AccountId, MaxSize>
where
    AccountId: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            parent: self.parent,
            owner: self.owner.clone(),
            meta: self.meta.clone(),
            payload: self.payload.clone(),
        }
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

        /// Maximum data size for node metadata and payload
        #[pallet::constant]
        type MaxDataSize: Get<u32>;

        /// Maximum tree depth
        #[pallet::constant]
        type MaxTreeDepth: Get<u32>;

        /// Maximum children per node
        #[pallet::constant]
        type MaxChildrenPerNode: Get<u32>;

        /// Maximum nodes per owner
        #[pallet::constant]
        type MaxNodesPerOwner: Get<u32>;

        /// Maximum root nodes
        #[pallet::constant]
        type MaxRootNodes: Get<u32>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Next node ID counter
    #[pallet::storage]
    #[pallet::getter(fn next_node_id)]
    pub type NextNodeId<T> = StorageValue<_, NodeId, ValueQuery>;

    /// Nodes storage
    #[pallet::storage]
    #[pallet::getter(fn nodes)]
    pub type Nodes<T: Config> =
        StorageMap<_, Twox64Concat, NodeId, Node<T::AccountId, T::MaxDataSize>>;

    /// Index of nodes by owner
    #[pallet::storage]
    #[pallet::getter(fn nodes_by_owner)]
    pub type NodesByOwner<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        BoundedVec<NodeId, T::MaxNodesPerOwner>,
        ValueQuery,
    >;

    /// Index of children by parent node
    #[pallet::storage]
    #[pallet::getter(fn nodes_by_parent)]
    pub type NodesByParent<T: Config> =
        StorageMap<_, Twox64Concat, NodeId, BoundedVec<NodeId, T::MaxChildrenPerNode>, ValueQuery>;

    /// Root nodes (nodes without parents)
    #[pallet::storage]
    #[pallet::getter(fn root_nodes)]
    pub type RootNodes<T: Config> =
        StorageValue<_, BoundedVec<NodeId, T::MaxRootNodes>, ValueQuery>;

    /// Crypto profiles storage
    #[pallet::storage]
    #[pallet::getter(fn crypto_profiles)]
    pub type CryptoProfiles<T: Config> =
        StorageMap<_, Twox64Concat, CryptoProfileId, CryptoProfile<T::MaxDataSize>>;

    /// Next crypto profile ID counter
    #[pallet::storage]
    #[pallet::getter(fn next_profile_id)]
    pub type NextProfileId<T> = StorageValue<_, CryptoProfileId, ValueQuery>;

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
        /// Crypto profile created [profile_id, algorithm]
        CryptoProfileCreated(CryptoProfileId, AlgorithmId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Node not found
        NodeNotFound,
        /// Parent node not found
        ParentNotFound,
        /// Crypto profile not found
        CryptoProfileNotFound,
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
        /// Too many nodes per owner
        TooManyNodesPerOwner,
        /// Too many root nodes
        TooManyRootNodes,
        /// Data size exceeds maximum
        DataTooLarge,
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
            meta: Option<NodeData<T::MaxDataSize>>,
            payload: Option<NodeData<T::MaxDataSize>>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Validate node data
            if let Some(ref m) = meta {
                Self::validate_node_data(m)?;
            }
            if let Some(ref p) = payload {
                Self::validate_node_data(p)?;
            }

            // Get new node ID
            let node_id = <NextNodeId<T>>::get();
            <NextNodeId<T>>::put(node_id.saturating_add(1));

            // Validate parent and ownership
            if let Some(pid) = parent_id {
                let parent = <Nodes<T>>::get(pid).ok_or(Error::<T>::ParentNotFound)?;
                ensure!(parent.owner == sender, Error::<T>::OwnerMismatch);

                // Check tree depth
                Self::check_tree_depth(pid)?;

                // Add to parent's children index
                <NodesByParent<T>>::try_mutate(pid, |children| {
                    children
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyChildren)
                })?;
            } else {
                // Add to root nodes
                <RootNodes<T>>::try_mutate(|roots| {
                    roots
                        .try_push(node_id)
                        .map_err(|_| Error::<T>::TooManyRootNodes)
                })?;
            }

            // Create node
            let node = Node {
                id: node_id,
                parent: parent_id,
                owner: sender.clone(),
                meta,
                payload,
            };

            // Store node
            <Nodes<T>>::insert(node_id, node);

            // Add to owner index
            <NodesByOwner<T>>::try_mutate(&sender, |nodes| {
                nodes
                    .try_push(node_id)
                    .map_err(|_| Error::<T>::TooManyNodesPerOwner)
            })?;

            Self::deposit_event(Event::NodeCreated(node_id, parent_id, sender));
            Ok(())
        }

        /// Set node metadata
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_meta())]
        pub fn set_meta(
            origin: OriginFor<T>,
            node_id: NodeId,
            meta: Option<NodeData<T::MaxDataSize>>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Validate node data
            if let Some(ref m) = meta {
                Self::validate_node_data(m)?;
            }

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
            payload: Option<NodeData<T::MaxDataSize>>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Validate node data
            if let Some(ref p) = payload {
                Self::validate_node_data(p)?;
            }

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

            // Check for cycles
            ensure!(
                !Self::is_ancestor(new_parent_id, node_id)?,
                Error::<T>::CycleDetected
            );

            // Check tree depth after move
            Self::check_tree_depth(new_parent_id)?;

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

            // Update node's parent
            <Nodes<T>>::mutate(node_id, |node_opt| {
                if let Some(node) = node_opt {
                    node.parent = Some(new_parent_id);
                }
            });

            Self::deposit_event(Event::NodeMoved(node_id, old_parent, new_parent_id, sender));
            Ok(())
        }

        /// Create a crypto profile
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::create_crypto_profile())]
        pub fn create_crypto_profile(
            origin: OriginFor<T>,
            algorithm: AlgorithmId,
            public_params: BoundedVec<u8, T::MaxDataSize>,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let profile_id = <NextProfileId<T>>::get();
            <NextProfileId<T>>::put(profile_id.saturating_add(1));

            let profile = CryptoProfile {
                id: profile_id,
                algorithm,
                public_params,
            };

            <CryptoProfiles<T>>::insert(profile_id, profile);

            Self::deposit_event(Event::CryptoProfileCreated(profile_id, algorithm));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Check if ancestor_id is an ancestor of node_id
        pub fn is_ancestor(node_id: NodeId, ancestor_id: NodeId) -> Result<bool, DispatchError> {
            let mut current_id = node_id;
            let max_depth = T::MaxTreeDepth::get();
            let mut depth = 0u32;

            loop {
                if current_id == ancestor_id {
                    return Ok(true);
                }

                if depth >= max_depth {
                    return Err(Error::<T>::MaxDepthExceeded.into());
                }

                match <Nodes<T>>::get(current_id) {
                    Some(node) => match node.parent {
                        Some(parent_id) => {
                            current_id = parent_id;
                            depth += 1;
                        }
                        None => return Ok(false),
                    },
                    None => return Ok(false),
                }
            }
        }

        /// Check tree depth from node to root
        fn check_tree_depth(node_id: NodeId) -> DispatchResult {
            let mut current_id = node_id;
            let max_depth = T::MaxTreeDepth::get();
            let mut depth = 0u32;

            loop {
                if depth >= max_depth {
                    return Err(Error::<T>::MaxDepthExceeded.into());
                }

                match <Nodes<T>>::get(current_id) {
                    Some(node) => match node.parent {
                        Some(parent_id) => {
                            current_id = parent_id;
                            depth += 1;
                        }
                        None => return Ok(()),
                    },
                    None => return Ok(()),
                }
            }
        }

        /// Validate node data
        fn validate_node_data(data: &NodeData<T::MaxDataSize>) -> DispatchResult {
            match data {
                NodeData::Plain(vec) => {
                    ensure!(
                        vec.len() as u32 <= T::MaxDataSize::get(),
                        Error::<T>::DataTooLarge
                    );
                }
                NodeData::Encrypted {
                    crypto_profile,
                    ciphertext,
                } => {
                    ensure!(
                        ciphertext.len() as u32 <= T::MaxDataSize::get(),
                        Error::<T>::DataTooLarge
                    );
                    ensure!(
                        <CryptoProfiles<T>>::contains_key(crypto_profile),
                        Error::<T>::CryptoProfileNotFound
                    );
                }
            }
            Ok(())
        }
    }
}
