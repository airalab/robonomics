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
//! Storage migrations for `pallet-robonomics-cps`.
//!
//! ## v1 -> v2: single `Node` struct -> per-field storage maps + Scope/Access
//!
//! Version 1 stored every node as a single `Node { parent, owner, path, meta,
//! payload }` struct in one `Nodes` map. Version 2 splits a node's attributes
//! into separate maps (`Parent`, `Meta`, `Payload`) and removes the per-node
//! `owner`/`path` fields entirely, replacing them with the Scope/Access
//! architecture: a node that starts a new administrative/economic boundary
//! gets a freshly allocated `ScopeId` (`ActiveScope`, `ScopeRoot`,
//! `ScopeOwner`); every other node resolves its Scope from the nearest such
//! ancestor.
//!
//! The migration preserves the *effective* owner of every node:
//!
//! - a root node always gets a freshly allocated Scope (its old owner
//!   becomes the new Scope's owner);
//! - a non-root node gets a freshly allocated Scope only if its old owner
//!   differs from its parent's old owner; otherwise it inherits the
//!   parent's (already migrated) Scope.
//!
//! `ScopeId`s are allocated in the same order nodes are iterated, starting
//! from `NextScopeId`. Version 1 had no Access grants to migrate.

use crate::{
    ActiveScope, Config, MaxTreeDepth, Meta, NextScopeId, NodeData, NodeId, Pallet, Parent,
    Payload, ScopeId, ScopeOwner, ScopeRoot,
};
use core::fmt::Debug;
use frame_support::{
    migrations::VersionedMigration,
    pallet_prelude::PhantomData,
    storage_alias,
    traits::{Get, UncheckedOnRuntimeUpgrade},
    weights::Weight,
    Blake2_128Concat, BoundedVec,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sp_std::collections::btree_map::BTreeMap;

/// Shadow of the v1 `Node` layout (with the now-removed `owner`/`path`
/// fields), used only for decoding pre-migration storage. Declared under the
/// old `Nodes` storage prefix via [`storage_alias`].
#[derive(Encode, Decode, MaxEncodedLen)]
struct OldNode<AccountId>
where
    AccountId: MaxEncodedLen + Debug,
{
    parent: Option<NodeId>,
    owner: AccountId,
    #[allow(dead_code)]
    path: BoundedVec<NodeId, MaxTreeDepth>,
    meta: Option<NodeData>,
    payload: Option<NodeData>,
}

#[storage_alias]
type Nodes<T: Config> = StorageMap<
    Pallet<T>,
    Blake2_128Concat,
    NodeId,
    OldNode<<T as frame_system::Config>::AccountId>,
>;

/// Versioned migration from storage version 1 to 2.
pub type MigrationToV2<T> = VersionedMigration<
    1,
    2,
    UncheckedMigrationToV2<T>,
    Pallet<T>,
    <T as frame_system::Config>::DbWeight,
>;

pub struct UncheckedMigrationToV2<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for UncheckedMigrationToV2<T> {
    fn on_runtime_upgrade() -> Weight {
        // Collect every old node into memory first. This avoids any
        // dependency on trie iteration order between already-migrated and
        // not-yet-migrated entries when looking up a parent's old owner.
        let old_nodes: sp_std::vec::Vec<(NodeId, OldNode<T::AccountId>)> =
            Nodes::<T>::iter().collect();
        let old_owner_of: BTreeMap<u64, T::AccountId> = old_nodes
            .iter()
            .map(|(id, old)| (id.0, old.owner.clone()))
            .collect();

        let reads: u64 = old_nodes.len() as u64;
        let mut writes: u64 = 0;

        // Migrated boundary nodes need to resolve their newly allocated
        // Scope even before their children are processed, so remember the
        // freshly allocated ScopeId per boundary NodeId.
        let mut scope_of: BTreeMap<u64, ScopeId> = BTreeMap::new();
        let mut next_scope_id: ScopeId = NextScopeId::<T>::get();

        for (id, old) in old_nodes.iter() {
            let is_boundary = match old.parent {
                None => true,
                Some(parent_id) => old_owner_of.get(&parent_id.0) != Some(&old.owner),
            };

            if is_boundary {
                let scope_id = next_scope_id;
                next_scope_id = next_scope_id.saturating_add(1);

                ScopeRoot::<T>::insert(scope_id, id);
                ScopeOwner::<T>::insert(scope_id, old.owner.clone());
                ActiveScope::<T>::insert(id, scope_id);
                scope_of.insert(id.0, scope_id);

                writes = writes.saturating_add(3);
            }

            Parent::<T>::insert(id, old.parent);
            writes = writes.saturating_add(1);

            if let Some(meta) = old.meta.clone() {
                Meta::<T>::insert(id, meta);
                writes = writes.saturating_add(1);
            }
            if let Some(payload) = old.payload.clone() {
                Payload::<T>::insert(id, payload);
                writes = writes.saturating_add(1);
            }
        }

        NextScopeId::<T>::put(next_scope_id);
        writes = writes.saturating_add(1);

        // Purge the old, now-obsolete `Nodes` storage.
        let _ = Nodes::<T>::clear(u32::MAX, None);
        writes = writes.saturating_add(old_nodes.len() as u64);

        T::DbWeight::get().reads_writes(reads, writes)
    }
}
