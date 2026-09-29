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
//! Benchmarking for pallet-robonomics-cps

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use sp_std::vec;

/// Build a chain of `len` nodes (a root plus `len - 1` descendants), all
/// created by `caller`, who becomes the owner of the root's freshly
/// allocated Scope. The deepest node's resolved scope path holds exactly
/// `len` nodes, so `len == MAX_SCOPE_DEPTH` yields the deepest node the
/// pallet admits. Returns `(root, deepest_node)`.
fn create_chain<T: Config>(caller: &T::AccountId, len: u32) -> (NodeId, NodeId) {
    assert!(
        (1..=MAX_SCOPE_DEPTH).contains(&len),
        "chain length must be within 1..=MAX_SCOPE_DEPTH"
    );
    let mut parent = None;
    let mut root = None;
    for _ in 0..len {
        let id = NextNodeId::<T>::get();
        assert_ok!(Pallet::<T>::create_node(
            RawOrigin::Signed(caller.clone()).into(),
            parent,
            None,
            None,
        ));
        if root.is_none() {
            root = Some(id);
        }
        parent = Some(id);
    }
    let (root, deepest) = (root.unwrap(), parent.unwrap());
    assert_eq!(
        Pallet::<T>::resolve_scope_path(deepest)
            .expect("chain resolves to its root Scope")
            .path
            .len(),
        len as usize
    );
    (root, deepest)
}

/// Build a maximum-size `NodeMeta` value. Used as the pre-existing value in
/// `set_meta`/`set_payload`'s `b = 0` (removal) benchmark point, so the
/// zero-byte point still measures a real storage-delete path (see issue
/// #671, point 7), and as the worst-case data component wherever a fixed
/// maximum-size value (rather than a size sweep) is appropriate.
fn max_meta() -> NodeMeta {
    BoundedVec::try_from(vec![1u8; MAX_META_SIZE as usize]).unwrap()
}

/// Build a maximum-size `NodePayload` value. See [`max_meta`].
fn max_payload() -> NodePayload {
    BoundedVec::try_from(vec![1u8; MAX_PAYLOAD_SIZE as usize]).unwrap()
}

/// Append `count` children to `parent`'s `Children` index.
fn fill_siblings<T: Config>(caller: &T::AccountId, parent: NodeId, count: u32) {
    for _ in 0..count {
        assert_ok!(Pallet::<T>::create_node(
            RawOrigin::Signed(caller.clone()).into(),
            Some(parent),
            None,
            None,
        ));
    }
}

/// Grant `Capability::Write` at `node` to `count` *distinct* principals, so
/// that `count` new physical `Access` entries (and `access_count` slots) are
/// consumed in `node`'s Scope. Re-granting to the same principal would only
/// update an existing entry and never grow the Scope towards
/// `MAX_ACCESS_ENTRIES_PER_SCOPE`.
///
/// Filling to the bound is the worst case for any operation that must
/// synchronously clear a Scope's entire `Access` prefix (`create_scope`
/// replacement, deleting a Scope-root leaf via `delete_node`).
fn fill_access<T: Config>(caller: &T::AccountId, node: NodeId, count: u32) {
    let scope_id = Pallet::<T>::resolve_scope(node)
        .expect("node resolves to a Scope")
        .id;
    let before = Pallet::<T>::scope_info(scope_id)
        .map(|info| info.access_count)
        .expect("scope exists");
    for i in 0..count {
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller.clone()).into(),
            node,
            account("principal", i, 0),
            Capability::Write,
            GrantMode::Node,
        ));
    }
    assert_eq!(
        Pallet::<T>::scope_info(scope_id).map(|info| info.access_count),
        Some(before + count)
    );
}

#[benchmarks]
mod benchmarks {
    use super::*;

    /// Worst case: `parent` sits at scope-local depth `MAX_SCOPE_DEPTH - 1`
    /// (the full parent path is walked for the owner/depth checks) and
    /// already holds `MAX_CHILDREN_PER_NODE - 1` children, so the new node
    /// lands exactly at `MAX_SCOPE_DEPTH` and fills `Children` to its bound.
    #[benchmark]
    fn create_node(m: Linear<0, MAX_META_SIZE>, p: Linear<0, MAX_PAYLOAD_SIZE>) {
        let caller: T::AccountId = whitelisted_caller();
        let (_, parent) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH - 1);
        fill_siblings::<T>(&caller, parent, MAX_CHILDREN_PER_NODE - 1);
        let node = NextNodeId::<T>::get();
        let meta: Option<NodeMeta> = if m == 0 {
            None
        } else {
            Some(BoundedVec::try_from(vec![1u8; m as usize]).unwrap())
        };
        let payload: Option<NodePayload> = if p == 0 {
            None
        } else {
            Some(BoundedVec::try_from(vec![1u8; p as usize]).unwrap())
        };

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller.clone()),
            Some(parent),
            meta.clone(),
            payload.clone(),
        );

        assert_eq!(
            Nodes::<T>::get(node).and_then(|info| info.parent),
            Some(parent)
        );
        assert_eq!(Meta::<T>::get(node), meta);
        assert_eq!(Payload::<T>::get(node), payload);
        assert_eq!(
            Children::<T>::get(parent).len(),
            MAX_CHILDREN_PER_NODE as usize
        );
        assert_eq!(
            Pallet::<T>::resolve_scope_path(node)
                .expect("new node resolves")
                .path
                .len(),
            MAX_SCOPE_DEPTH as usize
        );
    }

    /// Worst case: `sender` is not the Scope owner and is authorized through
    /// a `GrantMode::Subtree` `Write` `Access` granted at the Scope root,
    /// requiring a full `MAX_SCOPE_DEPTH`-node walk to be validated.
    ///
    /// `node` always starts with a maximum-size existing `Meta` value, so
    /// the `b = 0` point (which sets `meta` to `None`, removing it) still
    /// measures a real deletion rather than an unrealistically cheap no-op
    /// (see issue #671, point 7); every other point (`b > 0`) measures a
    /// same-size-domain replacement.
    #[benchmark]
    fn set_meta(b: Linear<0, MAX_META_SIZE>) {
        let caller: T::AccountId = whitelisted_caller();
        let accessor: T::AccountId = account("accessor", 0, 0);

        let (root, node) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller).into(),
            root,
            accessor.clone(),
            Capability::Write,
            GrantMode::Subtree,
        ));
        Meta::<T>::insert(node, max_meta());
        let meta: Option<NodeMeta> = if b == 0 {
            None
        } else {
            Some(BoundedVec::try_from(vec![1u8; b as usize]).unwrap())
        };

        #[extrinsic_call]
        _(RawOrigin::Signed(accessor), node, meta.clone());

        assert_eq!(Meta::<T>::get(node), meta);
    }

    /// Worst case: `sender` is not the Scope owner and is authorized through
    /// a `GrantMode::Subtree` `Write` `Access` granted at the Scope root,
    /// requiring a full `MAX_SCOPE_DEPTH`-node walk to be validated.
    ///
    /// `node` always starts with a maximum-size existing `Payload` value, so
    /// the `b = 0` point (which sets `payload` to `None`, removing it) still
    /// measures a real deletion rather than an unrealistically cheap no-op
    /// (see issue #671, point 7); every other point (`b > 0`) measures a
    /// same-size-domain replacement.
    #[benchmark]
    fn set_payload(b: Linear<0, MAX_PAYLOAD_SIZE>) {
        let caller: T::AccountId = whitelisted_caller();
        let accessor: T::AccountId = account("accessor", 0, 0);

        let (root, node) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller).into(),
            root,
            accessor.clone(),
            Capability::Write,
            GrantMode::Subtree,
        ));
        Meta::<T>::insert(node, max_meta());
        Payload::<T>::insert(node, max_payload());
        let payload: Option<NodePayload> = if b == 0 {
            None
        } else {
            Some(BoundedVec::try_from(vec![1u8; b as usize]).unwrap())
        };

        #[extrinsic_call]
        _(RawOrigin::Signed(accessor), node, payload.clone());

        assert_eq!(Payload::<T>::get(node), payload);
    }

    /// Worst case: `node` is a Scope-root leaf at scope-local depth
    /// `MAX_SCOPE_DEPTH` of its parent Scope, with maximum metadata and
    /// payload, a full parent `Children` vector (`MAX_CHILDREN_PER_NODE`
    /// entries, the target being the last one), and `a` (up to
    /// `MAX_ACCESS_ENTRIES_PER_SCOPE`) distinct `Access` entries that must be
    /// synchronously deleted together with the Scope boundary.
    #[benchmark]
    fn delete_node(a: Linear<0, MAX_ACCESS_ENTRIES_PER_SCOPE>) {
        let caller: T::AccountId = whitelisted_caller();

        let (_, parent) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH - 1);
        fill_siblings::<T>(&caller, parent, MAX_CHILDREN_PER_NODE - 1);
        let node = NextNodeId::<T>::get();
        assert_ok!(Pallet::<T>::create_node(
            RawOrigin::Signed(caller.clone()).into(),
            Some(parent),
            Some(max_meta()),
            Some(max_payload()),
        ));
        assert_eq!(
            Children::<T>::get(parent).len(),
            MAX_CHILDREN_PER_NODE as usize
        );
        assert_ok!(Pallet::<T>::create_scope(
            RawOrigin::Signed(caller.clone()).into(),
            node,
        ));
        fill_access::<T>(&caller, node, a);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), node);

        assert!(!Nodes::<T>::contains_key(node));
        assert!(!Meta::<T>::contains_key(node));
        assert!(!Payload::<T>::contains_key(node));
        assert_eq!(
            Children::<T>::get(parent).len(),
            (MAX_CHILDREN_PER_NODE - 1) as usize
        );
    }

    /// Worst case replacement path: `node` already roots an active Scope
    /// holding `a` `Access` entries, one of which is the non-owner caller's
    /// `CreateScope` grant; replacing the Scope synchronously clears all of
    /// them. `a` starts at 1 because the caller's own grant always occupies
    /// one of the `MAX_ACCESS_ENTRIES_PER_SCOPE` slots.
    #[benchmark]
    fn create_scope(a: Linear<1, MAX_ACCESS_ENTRIES_PER_SCOPE>) {
        let caller: T::AccountId = whitelisted_caller();
        let accessor: T::AccountId = account("accessor", 0, 0);
        let (_, node) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH);
        assert_ok!(Pallet::<T>::create_scope(
            RawOrigin::Signed(caller.clone()).into(),
            node,
        ));
        let old_scope = Pallet::<T>::node_info(node)
            .and_then(|info| info.scope)
            .expect("node roots a Scope");
        fill_access::<T>(&caller, node, a - 1);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller.clone()).into(),
            node,
            accessor.clone(),
            Capability::CreateScope,
            GrantMode::Node,
        ));
        assert_eq!(
            Pallet::<T>::scope_info(old_scope).map(|info| info.access_count),
            Some(a)
        );

        #[extrinsic_call]
        _(RawOrigin::Signed(accessor.clone()), node);

        assert!(!Scopes::<T>::contains_key(old_scope));
        assert_eq!(Access::<T>::iter_prefix(old_scope).count(), 0);
        assert_eq!(
            Pallet::<T>::node_info(node)
                .and_then(|info| info.scope)
                .and_then(|scope_id| Pallet::<T>::scope_info(scope_id))
                .map(|info| info.owner),
            Some(accessor)
        );
    }

    /// Worst case: `node` is at `MAX_SCOPE_DEPTH`, exercising the full
    /// `resolve_scope` walk, and the grant creates a brand-new entry that
    /// takes the Scope's last free slot, bringing `access_count` to exactly
    /// `MAX_ACCESS_ENTRIES_PER_SCOPE`.
    #[benchmark]
    fn grant_access() {
        let caller: T::AccountId = whitelisted_caller();
        let principal: T::AccountId = account("grantee", 0, 0);
        let (_, node) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH);
        fill_access::<T>(&caller, node, MAX_ACCESS_ENTRIES_PER_SCOPE - 1);

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            node,
            principal.clone(),
            Capability::Write,
            GrantMode::Subtree,
        );

        let resolved = Pallet::<T>::resolve_scope(node).expect("scope resolves");
        assert!(Access::<T>::get(resolved.id, (node, principal)).contains(Capability::Write));
        assert_eq!(
            Pallet::<T>::scope_info(resolved.id).map(|info| info.access_count),
            Some(MAX_ACCESS_ENTRIES_PER_SCOPE)
        );
    }

    /// Worst case: `node` is at `MAX_SCOPE_DEPTH`, exercising the full
    /// `resolve_scope` walk before the `Access` entry is removed.
    #[benchmark]
    fn revoke_access() {
        let caller: T::AccountId = whitelisted_caller();
        let principal: T::AccountId = account("principal", 0, 0);
        let (_, node) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller.clone()).into(),
            node,
            principal.clone(),
            Capability::Write,
            GrantMode::Subtree,
        ));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            node,
            principal.clone(),
            Capability::Write,
        );

        let resolved = Pallet::<T>::resolve_scope(node).expect("scope resolves");
        assert!(!Access::<T>::contains_key(resolved.id, (node, principal)));
    }

    /// Diagnostic (non-dispatchable) benchmark measuring the worst-case cost
    /// of [`Pallet::resolve_scope`] alone: a `MAX_SCOPE_DEPTH` walk with no
    /// active Scope until the root.
    #[benchmark(extra)]
    fn resolve_scope_worst_case() {
        let caller: T::AccountId = whitelisted_caller();
        let (_, node) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH);

        #[block]
        {
            assert!(Pallet::<T>::resolve_scope(node).is_ok());
        }
    }

    /// Diagnostic (non-dispatchable) benchmark measuring the worst-case cost
    /// of an `Access` traversal: a `GrantMode::Subtree` `Write` grant at the
    /// Scope root, checked from the deepest descendant.
    #[benchmark(extra)]
    fn access_traversal_worst_case() {
        let caller: T::AccountId = whitelisted_caller();
        let accessor: T::AccountId = account("accessor", 0, 0);
        let (root, node) = create_chain::<T>(&caller, MAX_SCOPE_DEPTH);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller).into(),
            root,
            accessor.clone(),
            Capability::Write,
            GrantMode::Subtree,
        ));

        #[block]
        {
            assert_ok!(Pallet::<T>::set_meta(
                RawOrigin::Signed(accessor).into(),
                node,
                None,
            ));
        }
    }

    /// Diagnostic (non-dispatchable) benchmark measuring the worst-case cost
    /// of synchronously clearing a Scope's `Access` prefix on invalidation
    /// (the cost `create_scope` replacement and deleting a Scope-root leaf
    /// add on top of their other bookkeeping): a Scope filled to
    /// `MAX_ACCESS_ENTRIES_PER_SCOPE` entries, cleared via
    /// `Pallet::clear_scope_access` in a single `clear_prefix` call.
    #[benchmark(extra)]
    fn clear_scope_access_worst_case() {
        let caller: T::AccountId = whitelisted_caller();
        let (root, _) = create_chain::<T>(&caller, 1);
        let scope_id = Pallet::<T>::node_info(root)
            .and_then(|info| info.scope)
            .expect("root has a Scope");
        fill_access::<T>(&caller, root, MAX_ACCESS_ENTRIES_PER_SCOPE);
        let access_items = Pallet::<T>::scope_info(scope_id)
            .map(|info| info.access_count)
            .expect("scope exists");
        assert_eq!(access_items, MAX_ACCESS_ENTRIES_PER_SCOPE);

        #[block]
        {
            Pallet::<T>::clear_scope_access(scope_id, access_items);
        }

        assert_eq!(Access::<T>::iter_prefix(scope_id).count(), 0);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Runtime);
}
