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
//! Storage migrations of `pallet-robonomics-cps`.
//!
//! ## Version 1 to 2
//!
//! [`MigrationToV2`] converts the version 1 layout, where every node was a
//! single `Node { parent, owner, path, meta, payload }` value in `Nodes`,
//! into the version 2 layout: `Nodes` holding [`NodeInfo`], separate `Meta`
//! and `Payload` maps, `Scopes`, and `Children`.
//!
//! ### Scopes
//!
//! The migration keeps the owner of every node, as the owner of the Scope
//! the node resolves to:
//!
//! - a root node gets a new Scope owned by its old owner;
//! - a non-root node gets a new Scope only if its old owner differs from its
//!   parent's old owner; otherwise it resolves to its parent's Scope.
//!
//! `ScopeId`s are allocated from `NextScopeId` in node iteration order, and
//! every new Scope starts with `access_count = 0`. Version 1 has no `Access`
//! storage; if `Access` entries are nevertheless present, each Scope's
//! `access_count` is set to the number of entries stored under it. A Scope
//! with more than [`MAX_ACCESS_ENTRIES_PER_SCOPE`] entries is reported with
//! [`frame_support::defensive!`] and its count is left at `0`.
//!
//! The migration also removes the `RootNodes` storage value (a list of up to
//! 100 root `NodeId`s), in case it exists.
//!
//! ### `Children`
//!
//! `Children` is rebuilt from the legacy `parent` links, so that
//! `Children[parent]` contains `child` exactly when
//! `Nodes[child].parent == Some(parent)`.
//!
//! If a legacy node has more than
//! [`MAX_CHILDREN_PER_NODE`](crate::MAX_CHILDREN_PER_NODE) children, its
//! `Children` entry is left unset instead of being truncated, and the
//! condition is reported with [`frame_support::defensive!`]: an error log in
//! release builds and a panic when `debug_assertions` are enabled (tests,
//! try-runtime). Such a tree should be ruled out on the target chain before
//! the migration is applied.
//!
//! ### `Meta` bound
//!
//! Version 1's `meta`/`payload` fields shared a 2048-byte bound. In version
//! 2, `Payload` allows 8 KiB, so every legacy `payload` value migrates
//! unchanged, while `Meta` allows 1 KiB. A legacy `meta` value longer than
//! [`crate::MAX_META_SIZE`] is truncated to exactly that many bytes before
//! it is written into the new `Meta` map; the trailing bytes are dropped.

use crate::{
    Access, Children, Config, MaxChildrenPerNode, MaxScopeDepth, Meta, NextScopeId, NodeId,
    NodeInfo, NodeMeta, Pallet, Payload, ScopeId, ScopeInfo, Scopes, MAX_ACCESS_ENTRIES_PER_SCOPE,
    MAX_META_SIZE,
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

/// Shared bound for v1's `meta`/`payload` fields (2048 bytes), used only to
/// decode pre-migration storage. Kept separate from [`crate::MaxMetaSize`]
/// / [`crate::MaxPayloadSize`] since it reflects the legacy on-chain layout,
/// not the current, now-independent, bounds.
type OldDataSize = ConstU32<2048>;
/// Shadow of the shared v1 `meta`/`payload` bounded-vector type.
type OldNodeData = BoundedVec<u8, OldDataSize>;

/// Shadow of the v1 `Node` layout (with the now-removed `owner`/`path`
/// fields), used only for decoding pre-migration storage. Declared under the
/// old `Nodes` storage prefix via [`storage_alias`].
///
/// This intentionally shares the Rust identifier `Nodes` with the current
/// pallet's own `Nodes` storage item (`crate::Nodes`): both target the exact
/// same on-chain prefix name, since [`storage_alias`] derives the prefix
/// from the type's identifier. Referring to the *current* `Nodes` storage
/// from within this module therefore always uses the fully-qualified
/// `crate::Nodes::<T>` path, never a bare `Nodes`, to avoid ambiguity with
/// this legacy shadow.
#[derive(Encode, Decode, MaxEncodedLen)]
struct OldNode<AccountId>
where
    AccountId: MaxEncodedLen + Debug,
{
    parent: Option<NodeId>,
    owner: AccountId,
    #[allow(dead_code)]
    path: BoundedVec<NodeId, MaxScopeDepth>,
    meta: Option<OldNodeData>,
    payload: Option<OldNodeData>,
}

#[storage_alias]
type Nodes<T: Config> = StorageMap<
    Pallet<T>,
    Blake2_128Concat,
    NodeId,
    OldNode<<T as frame_system::Config>::AccountId>,
>;

/// Truncate a legacy `meta` value down to [`MAX_META_SIZE`] bytes if it
/// exceeds the new, shrunk `Meta` bound (see module docs). Values already
/// within the bound are passed through unchanged.
fn truncate_meta(meta: OldNodeData) -> NodeMeta {
    let mut bytes = meta.into_inner();
    bytes.truncate(MAX_META_SIZE as usize);
    BoundedVec::try_from(bytes)
        .expect("just truncated to MAX_META_SIZE, so this always fits the bound; qed")
}

/// Shadow of the now-removed `RootNodes` index (a bare `StorageValue`
/// holding up to 100 root `NodeId`s). Declared only so this migration can
/// unconditionally kill it, regardless of whether it was ever populated.
#[storage_alias]
type RootNodes<T: Config> = StorageValue<Pallet<T>, BoundedVec<NodeId, ConstU32<100>>>;

/// Migration from storage version 1 to 2, applied only when the on-chain
/// storage version is 1. See the [module docs](self).
pub type MigrationToV2<T> = VersionedMigration<
    1,
    2,
    UncheckedMigrationToV2<T>,
    Pallet<T>,
    <T as frame_system::Config>::DbWeight,
>;

/// Migration from storage version 1 to 2 without the storage version check.
/// Use [`MigrationToV2`] in runtimes.
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

        let mut reads: u64 = old_nodes.len() as u64;
        let mut writes: u64 = 0;

        // Migrated boundary nodes need to resolve their newly allocated
        // Scope even before their children are processed, so remember the
        // freshly allocated ScopeId per boundary NodeId.
        let mut scope_of: BTreeMap<u64, ScopeId> = BTreeMap::new();
        let mut next_scope_id: ScopeId = NextScopeId::<T>::get();

        // Reverse index rebuilt from the legacy `parent` links: every node
        // with a parent is appended to that parent's child list, in the same
        // (deterministic, since `old_nodes` is a materialized `Vec`) order
        // it is encountered. Grouped by parent up front so the
        // `MAX_CHILDREN_PER_NODE` bound is checked once per parent below,
        // rather than re-reading/re-writing `Children` once per child.
        let mut children_of: BTreeMap<u64, sp_std::vec::Vec<NodeId>> = BTreeMap::new();

        for (id, old) in old_nodes.iter() {
            let is_boundary = match old.parent {
                None => true,
                Some(parent_id) => old_owner_of.get(&parent_id.0) != Some(&old.owner),
            };

            let scope = if is_boundary {
                let scope_id = next_scope_id;
                next_scope_id = next_scope_id.saturating_add(1);

                Scopes::<T>::insert(
                    scope_id,
                    ScopeInfo {
                        owner: old.owner.clone(),
                        access_count: 0,
                    },
                );
                scope_of.insert(id.0, scope_id);
                writes = writes.saturating_add(1);

                Some(scope_id)
            } else {
                None
            };

            crate::Nodes::<T>::insert(
                id,
                NodeInfo {
                    parent: old.parent,
                    scope,
                },
            );
            writes = writes.saturating_add(1);

            if let Some(parent_id) = old.parent {
                children_of.entry(parent_id.0).or_default().push(*id);
            }

            if let Some(meta) = old.meta.clone() {
                // Truncate down to the new, shrunk `MAX_META_SIZE` bound if
                // needed (see module docs).
                Meta::<T>::insert(id, truncate_meta(meta));
                writes = writes.saturating_add(1);
            }
            if let Some(payload) = old.payload.clone() {
                // The `Payload` bound only grew (2048 -> 8192), so every
                // legacy value fits unchanged.
                let payload = BoundedVec::try_from(payload.into_inner())
                    .expect("old payload bound (2048) fits the new, larger bound (8192); qed");
                Payload::<T>::insert(id, payload);
                writes = writes.saturating_add(1);
            }
        }

        NextScopeId::<T>::put(next_scope_id);
        writes = writes.saturating_add(1);

        // Rebuild `Children` from the grouped legacy `parent` links so
        // `Nodes[child].parent == Some(parent) iff Children[parent]
        // contains child` holds immediately after migration (see module
        // docs). A parent with more children than `MAX_CHILDREN_PER_NODE`
        // must never be *silently* truncated: `BoundedVec::try_from`
        // failing here means the legacy tree already violates an invariant
        // every post-migration node is required to uphold. This is
        // reported loudly via [`frame_support::defensive`] (an error log
        // in production, in addition to panicking under `debug_assertions`
        // so tests/try-runtime runs catch it immediately) and that
        // parent's `Children` entry is left unset rather than holding
        // a truncated, corrupted subset of its real children - callers
        // must not be given a partial answer that looks complete.
        for (parent_id, children) in children_of {
            match BoundedVec::<NodeId, MaxChildrenPerNode>::try_from(children) {
                Ok(bounded) => {
                    Children::<T>::insert(NodeId(parent_id), bounded);
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

        // Purge the old, now-obsolete `Nodes` storage. Removed one key at a
        // time (rather than via `clear`/`clear_prefix`) because the legacy
        // `Nodes` alias and the new `crate::Nodes` storage share the exact
        // same on-chain prefix (see this module's doc comment on the
        // `Nodes` alias) - a prefix-level clear would also wipe out the
        // `NodeInfo` entries just written above, since `clear_prefix`
        // removes every child key under a prefix regardless of which
        // hasher produced it.
        for (id, _) in old_nodes.iter() {
            Nodes::<T>::remove(id);
        }
        writes = writes.saturating_add(old_nodes.len() as u64);

        // Purge any leftover `RootNodes` index bytes (see module docs).
        RootNodes::<T>::kill();
        writes = writes.saturating_add(1);

        // Derive each Scope's `access_count` from whatever `Access` entries
        // already exist at upgrade time (see module docs). Version 1 never
        // had any, so this is a defensive no-op on a chain migrating
        // straight from v1; it only matters for a chain that already ran
        // intermediate code with `Access` entries present.
        let mut counts: BTreeMap<u64, u32> = BTreeMap::new();
        let mut access_reads: u64 = 0;
        for (scope_id, _key, _flags) in Access::<T>::iter() {
            access_reads = access_reads.saturating_add(1);
            *counts.entry(scope_id.0).or_default() += 1;
        }
        reads = reads.saturating_add(access_reads);

        for (scope_id, count) in counts {
            if count > MAX_ACCESS_ENTRIES_PER_SCOPE {
                frame_support::defensive!(
                    "CPS v1->v2 migration: Scope already exceeds MAX_ACCESS_ENTRIES_PER_SCOPE",
                    (scope_id, count, MAX_ACCESS_ENTRIES_PER_SCOPE)
                );
                continue;
            }
            Scopes::<T>::mutate(ScopeId(scope_id), |maybe_info| {
                if let Some(info) = maybe_info {
                    info.access_count = count;
                } else {
                    frame_support::defensive!(
                        "CPS v1->v2 migration: Access entries reference a Scope with no ScopeInfo",
                        scope_id
                    );
                }
            });
            writes = writes.saturating_add(1);
        }

        T::DbWeight::get().reads_writes(reads, writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<sp_std::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
        // Snapshot every legacy parent/child edge so `post_upgrade` can
        // verify the rebuilt `Children` reverse index reflects every
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

        // Every edge must be resolvable both ways: `Nodes[child].parent ==
        // Some(parent)` (the migration always sets this) and
        // `Children[parent]` contains `child` exactly once.
        let mut seen_per_parent: BTreeMap<u64, u32> = BTreeMap::new();
        for (parent, child) in edges.iter() {
            frame_support::ensure!(
                crate::Nodes::<T>::get(child).and_then(|info| info.parent) == Some(*parent),
                "CPS v1->v2 migration: Nodes[child].parent does not match legacy parent link"
            );
            let children = Children::<T>::get(parent);
            let occurrences = children.iter().filter(|c| *c == child).count();
            frame_support::ensure!(
                occurrences == 1,
                "CPS v1->v2 migration: Children[parent] does not contain child exactly once"
            );
            *seen_per_parent.entry(parent.0).or_default() += 1;
        }

        // No extra/orphaned entries: every parent's `Children` length
        // must equal exactly the number of legacy edges pointing at it -
        // anything more would mean a stray or duplicated entry that the
        // per-edge check above could not catch (e.g. a child appearing
        // under the wrong parent in addition to the right one).
        for (parent, count) in seen_per_parent {
            let stored_len = Children::<T>::get(NodeId(parent)).len() as u32;
            frame_support::ensure!(
                stored_len == count,
                "CPS v1->v2 migration: Children[parent] length does not match the number \
                 of legacy edges for that parent"
            );
        }

        // Every Scope's `access_count` must exactly match the number of
        // physical `Access` entries actually stored under its `ScopeId`,
        // and must never exceed the configured bound (see module docs).
        let mut actual_counts: BTreeMap<u64, u32> = BTreeMap::new();
        for (scope_id, _key, _flags) in Access::<T>::iter() {
            *actual_counts.entry(scope_id.0).or_default() += 1;
        }
        for (scope_id, count) in actual_counts {
            frame_support::ensure!(
                count <= MAX_ACCESS_ENTRIES_PER_SCOPE,
                "CPS v1->v2 migration: a Scope exceeds MAX_ACCESS_ENTRIES_PER_SCOPE after migration"
            );
            frame_support::ensure!(
                Scopes::<T>::get(ScopeId(scope_id)).map(|info| info.access_count) == Some(count),
                "CPS v1->v2 migration: ScopeInfo.access_count does not match actual Access entry count"
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

    /// Like [`insert_old_node`], but with an explicit legacy `meta` value
    /// (up to the old, shared 2048-byte bound).
    fn insert_old_node_with_meta(
        id: u64,
        parent: Option<u64>,
        owner: u64,
        meta: sp_std::vec::Vec<u8>,
    ) {
        Nodes::<Runtime>::insert(
            NodeId(id),
            OldNode::<u64> {
                parent: parent.map(NodeId),
                owner,
                path: BoundedVec::try_from(sp_std::vec![]).unwrap(),
                meta: Some(BoundedVec::try_from(meta).unwrap()),
                payload: None,
            },
        );
    }

    /// After migrating a tree with several parents each holding multiple
    /// children, every `Nodes[child].parent == Some(parent)` link must be
    /// mirrored by `Children[parent]` containing exactly that child.
    #[test]
    fn children_rebuilt_and_consistent_with_nodes() {
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
                    crate::Nodes::<Runtime>::get(NodeId(child)).and_then(|info| info.parent),
                    Some(NodeId(parent))
                );
                assert!(Children::<Runtime>::get(NodeId(parent)).contains(&NodeId(child)));
            }
            assert_eq!(
                crate::Nodes::<Runtime>::get(NodeId(0)).and_then(|info| info.parent),
                None
            );
            assert!(!Children::<Runtime>::get(NodeId(0)).is_empty());
            assert_eq!(Children::<Runtime>::get(NodeId(0)).len(), 2);
            assert_eq!(Children::<Runtime>::get(NodeId(1)).len(), 2);
            // Leaves have no children of their own.
            assert!(Children::<Runtime>::get(NodeId(2)).is_empty());
            assert!(Children::<Runtime>::get(NodeId(3)).is_empty());
            assert!(Children::<Runtime>::get(NodeId(4)).is_empty());

            // The old `Nodes` storage is fully purged.
            assert_eq!(Nodes::<Runtime>::iter().count(), 0);
        });
    }

    /// A parent with more children than `MAX_CHILDREN_PER_NODE` must never
    /// end up with a truncated `Children` entry: the `defensive!` in
    /// `on_runtime_upgrade` panics under `debug_assertions` (enabled in
    /// tests) rather than silently truncating, so this is asserted via
    /// `catch_unwind`; in a release build the same condition would instead
    /// only emit an error log and leave the entry unset.
    #[test]
    fn children_never_silently_truncated_on_overflow() {
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

    /// A legacy `meta` value longer than `MAX_META_SIZE` (see module docs)
    /// must be truncated to exactly `MAX_META_SIZE` bytes rather than
    /// rejected or dropped.
    #[test]
    fn oversized_legacy_meta_is_truncated_to_new_bound() {
        new_test_ext().execute_with(|| {
            let oversized = sp_std::vec![7u8; 2048];
            insert_old_node_with_meta(0, None, 1, oversized.clone());

            UncheckedMigrationToV2::<Runtime>::on_runtime_upgrade();

            let migrated = Meta::<Runtime>::get(NodeId(0)).expect("meta migrated");
            assert_eq!(migrated.len(), MAX_META_SIZE as usize);
            assert_eq!(migrated.into_inner(), oversized[..MAX_META_SIZE as usize]);
        });
    }

    /// A legacy `meta` value already within the new `MAX_META_SIZE` bound
    /// must migrate byte-for-byte, unchanged.
    #[test]
    fn undersized_legacy_meta_is_preserved_exactly() {
        new_test_ext().execute_with(|| {
            let small = sp_std::vec![9u8; 100];
            insert_old_node_with_meta(0, None, 1, small.clone());

            UncheckedMigrationToV2::<Runtime>::on_runtime_upgrade();

            let migrated = Meta::<Runtime>::get(NodeId(0)).expect("meta migrated");
            assert_eq!(migrated.into_inner(), small);
        });
    }
}
