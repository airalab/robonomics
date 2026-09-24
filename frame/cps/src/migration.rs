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
//! into separate maps (`Parents`, `Meta`, `Payload`) and removes the per-node
//! `owner`/`path` fields entirely, replacing them with the Scope/Access
//! architecture: a node that starts a new administrative/economic boundary
//! gets a freshly allocated `ScopeId`, stored together with its owner in a
//! single `ActiveScope` entry (`(ScopeId, AccountId)`); every other node
//! resolves its Scope from the nearest such ancestor.
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
//!
//! This migration also opportunistically clears the now-removed `RootNodes`
//! index (a `StorageValue` that used to hold the list of parentless nodes,
//! bounded to 100 entries). It was never populated by this migration in the
//! first place, but chains that had already run newer code with `RootNodes`
//! present could have leftover bytes; clearing it here ensures no orphaned
//! storage remains regardless of upgrade path.
//!
//! ## `NodesByParent` reverse index
//!
//! Version 2 also introduces `NodesByParent`, the reverse-lookup index used
//! for O(1) child enumeration. The migration rebuilds it from the legacy
//! `parent` links so the invariant
//! `Parents[child] == Some(parent) iff NodesByParent[parent] contains child`
//! holds immediately after migration, exactly as it does for every node
//! created afterwards via [`Pallet::create_node`](crate::Pallet::create_node).
//!
//! If any legacy node has more than [`MAX_CHILDREN_PER_NODE`](crate::MAX_CHILDREN_PER_NODE)
//! children, that parent's `NodesByParent` entry is deliberately left unset
//! rather than silently populated with a truncated, corrupted subset of its
//! real children - a `BoundedVec` that dropped entries past its bound would
//! violate the invariant above for the truncated children (they would
//! still resolve via `Parents`, but would no longer be reachable via
//! `NodesByParent`). This is reported via [`frame_support::defensive`]: an
//! error log in production (so the condition is never missed), and a panic
//! under `debug_assertions` (so it is caught immediately by tests and
//! try-runtime runs rather than shipped). Runtimes should still prove ahead
//! of time (e.g. by inspecting live/representative state) that this cannot
//! happen before applying this migration to a chain where it might.
//!
//! With `try-runtime` checks enabled, [`UncheckedMigrationToV2::pre_upgrade`]
//! and [`UncheckedMigrationToV2::post_upgrade`] additionally verify, from a
//! snapshot of every legacy parent/child edge, that after migration every
//! edge is reflected on both sides of the reverse index exactly once (no
//! missing entries, no duplicates), i.e. the invariant above holds for the
//! whole tree, not just spot checks.

use crate::{
    ActiveScope, Config, MaxChildrenPerNode, MaxTreeDepth, Meta, NextScopeId, NodeData, NodeId,
    NodesByParent, Pallet, Parents, Payload, ScopeId,
};
use core::fmt::Debug;
use frame_support::{
    migrations::VersionedMigration,
    pallet_prelude::{ConstU32, PhantomData},
    storage_alias,
    traits::{Get, UncheckedOnRuntimeUpgrade},
    weights::Weight,
    Blake2_128Concat, BoundedVec,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sp_std::collections::btree_map::BTreeMap;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

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

/// Shadow of the now-removed `RootNodes` index (a bare `StorageValue`
/// holding up to 100 root `NodeId`s). Declared only so this migration can
/// unconditionally kill it, regardless of whether it was ever populated.
#[storage_alias]
type RootNodes<T: Config> = StorageValue<Pallet<T>, BoundedVec<NodeId, ConstU32<100>>>;

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

        // Reverse index rebuilt alongside `Parents`: every node with a
        // parent is appended to that parent's child list, in the same
        // (deterministic, since `old_nodes` is a materialized `Vec`) order
        // it is encountered. Grouped by parent up front so the
        // `MAX_CHILDREN_PER_NODE` bound is checked once per parent below,
        // rather than re-reading/re-writing `NodesByParent` once per child.
        let mut children_of: BTreeMap<u64, sp_std::vec::Vec<NodeId>> = BTreeMap::new();

        for (id, old) in old_nodes.iter() {
            let is_boundary = match old.parent {
                None => true,
                Some(parent_id) => old_owner_of.get(&parent_id.0) != Some(&old.owner),
            };

            if is_boundary {
                let scope_id = next_scope_id;
                next_scope_id = next_scope_id.saturating_add(1);

                ActiveScope::<T>::insert(id, (scope_id, old.owner.clone()));
                scope_of.insert(id.0, scope_id);

                writes = writes.saturating_add(1);
            }

            Parents::<T>::insert(id, old.parent);
            writes = writes.saturating_add(1);

            if let Some(parent_id) = old.parent {
                children_of.entry(parent_id.0).or_default().push(*id);
            }

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

        // Rebuild `NodesByParent` from the grouped legacy `parent` links so
        // `Parents[child] == Some(parent) iff NodesByParent[parent]
        // contains child` holds immediately after migration (see module
        // docs). A parent with more children than `MAX_CHILDREN_PER_NODE`
        // must never be *silently* truncated: `BoundedVec::try_from`
        // failing here means the legacy tree already violates an invariant
        // every post-migration node is required to uphold. This is
        // reported loudly via [`frame_support::defensive`] (an error log
        // in production, in addition to panicking under `debug_assertions`
        // so tests/try-runtime runs catch it immediately) and that
        // parent's `NodesByParent` entry is left unset rather than holding
        // a truncated, corrupted subset of its real children - callers
        // must not be given a partial answer that looks complete.
        for (parent_id, children) in children_of {
            match BoundedVec::<NodeId, MaxChildrenPerNode>::try_from(children) {
                Ok(bounded) => {
                    NodesByParent::<T>::insert(NodeId(parent_id), bounded);
                    writes = writes.saturating_add(1);
                }
                Err(children) => {
                    frame_support::defensive!(
                        "CPS v1->v2 migration: legacy node exceeds MAX_CHILDREN_PER_NODE",
                        (
                            parent_id,
                            children.len(),
                            <MaxChildrenPerNode as Get<u32>>::get()
                        ),
                    );
                }
            }
        }

        // Purge the old, now-obsolete `Nodes` storage.
        let _ = Nodes::<T>::clear(u32::MAX, None);
        writes = writes.saturating_add(old_nodes.len() as u64);

        // Purge any leftover `RootNodes` index bytes (see module docs).
        RootNodes::<T>::kill();
        writes = writes.saturating_add(1);

        T::DbWeight::get().reads_writes(reads, writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<sp_std::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
        // Snapshot every legacy parent/child edge so `post_upgrade` can
        // verify the rebuilt `NodesByParent` reverse index reflects every
        // one of them exactly once, with no duplicates and none missing.
        let mut edges: Vec<(NodeId, NodeId)> = Nodes::<T>::iter()
            .filter_map(|(child, old)| old.parent.map(|parent| (parent, child)))
            .collect();
        edges.sort_by_key(|(parent, child)| (parent.0, child.0));

        Ok(edges.encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: sp_std::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        let edges: Vec<(NodeId, NodeId)> = Decode::decode(&mut state.as_slice())
            .map_err(|_| "CPS v1->v2 migration: failed to decode pre_upgrade state")?;

        // Every edge must be resolvable both ways: `Parents[child] ==
        // Some(parent)` (the migration always sets this) and
        // `NodesByParent[parent]` contains `child` exactly once.
        let mut seen_per_parent: BTreeMap<u64, u32> = BTreeMap::new();
        for (parent, child) in edges.iter() {
            frame_support::ensure!(
                Parents::<T>::get(child) == Some(Some(*parent)),
                "CPS v1->v2 migration: Parents[child] does not match legacy parent link"
            );
            let children = NodesByParent::<T>::get(parent);
            let occurrences = children.iter().filter(|c| *c == child).count();
            frame_support::ensure!(
                occurrences == 1,
                "CPS v1->v2 migration: NodesByParent[parent] does not contain child exactly once"
            );
            *seen_per_parent.entry(parent.0).or_default() += 1;
        }

        // No extra/orphaned entries: every parent's `NodesByParent` length
        // must equal exactly the number of legacy edges pointing at it -
        // anything more would mean a stray or duplicated entry that the
        // per-edge check above could not catch (e.g. a child appearing
        // under the wrong parent in addition to the right one).
        for (parent, count) in seen_per_parent {
            let stored_len = NodesByParent::<T>::get(NodeId(parent)).len() as u32;
            frame_support::ensure!(
                stored_len == count,
                "CPS v1->v2 migration: NodesByParent[parent] length does not match the number \
                 of legacy edges for that parent"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{new_test_ext, Runtime};

    /// Build a v1 `OldNode` entry directly in storage (bypassing the v2
    /// pallet API entirely, which no longer knows how to write this shape),
    /// mirroring what a pre-migration chain's state would look like.
    fn insert_old_node(id: u64, parent: Option<u64>, owner: u64) {
        Nodes::<Runtime>::insert(
            NodeId(id),
            OldNode::<u64> {
                parent: parent.map(NodeId),
                owner,
                path: BoundedVec::try_from(sp_std::vec![]).unwrap(),
                meta: None,
                payload: None,
            },
        );
    }

    /// After migrating a tree with several parents each holding multiple
    /// children, every `Parents[child] == Some(parent)` link must be
    /// mirrored by `NodesByParent[parent]` containing exactly that child.
    #[test]
    fn nodes_by_parent_rebuilt_and_consistent_with_parents() {
        new_test_ext().execute_with(|| {
            // 0 (root, owner 1)
            // ├── 1 (owner 1, same Scope)
            // │   ├── 3 (owner 1, same Scope)
            // │   └── 4 (owner 2, new nested Scope)
            // └── 2 (owner 1, same Scope)
            insert_old_node(0, None, 1);
            insert_old_node(1, Some(0), 1);
            insert_old_node(2, Some(0), 1);
            insert_old_node(3, Some(1), 1);
            insert_old_node(4, Some(1), 2);

            UncheckedMigrationToV2::<Runtime>::on_runtime_upgrade();

            // Every edge is reflected on both sides of the index.
            for (child, parent) in [(1u64, 0u64), (2, 0), (3, 1), (4, 1)] {
                assert_eq!(
                    Parents::<Runtime>::get(NodeId(child)),
                    Some(Some(NodeId(parent)))
                );
                assert!(NodesByParent::<Runtime>::get(NodeId(parent)).contains(&NodeId(child)));
            }
            assert_eq!(Parents::<Runtime>::get(NodeId(0)), Some(None));
            assert!(NodesByParent::<Runtime>::get(NodeId(0)).is_empty() == false);
            assert_eq!(NodesByParent::<Runtime>::get(NodeId(0)).len(), 2);
            assert_eq!(NodesByParent::<Runtime>::get(NodeId(1)).len(), 2);
            // Leaves have no children of their own.
            assert!(NodesByParent::<Runtime>::get(NodeId(2)).is_empty());
            assert!(NodesByParent::<Runtime>::get(NodeId(3)).is_empty());
            assert!(NodesByParent::<Runtime>::get(NodeId(4)).is_empty());

            // The old `Nodes` storage is fully purged.
            assert_eq!(Nodes::<Runtime>::iter().count(), 0);
        });
    }

    /// A parent with more children than `MAX_CHILDREN_PER_NODE` must never
    /// end up with a truncated `NodesByParent` entry: the `defensive!` in
    /// `on_runtime_upgrade` panics under `debug_assertions` (enabled in
    /// tests) rather than silently truncating, so this is asserted via
    /// `catch_unwind`; in a release build the same condition would instead
    /// only emit an error log and leave the entry unset.
    #[test]
    fn nodes_by_parent_never_silently_truncated_on_overflow() {
        new_test_ext().execute_with(|| {
            insert_old_node(0, None, 1);
            let max_children: u32 = <MaxChildrenPerNode as Get<u32>>::get();
            for i in 1..=(max_children + 1) {
                insert_old_node(i as u64, Some(0), 1);
            }

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                UncheckedMigrationToV2::<Runtime>::on_runtime_upgrade();
            }));
            assert!(
                result.is_err(),
                "expected the defensive overflow check to panic under debug_assertions"
            );
        });
    }
}
