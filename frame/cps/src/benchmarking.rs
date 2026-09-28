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
use frame_support::{assert_ok, traits::Get, BoundedVec};
use frame_system::RawOrigin;
use sp_std::vec;

/// Build a chain of `depth + 1` nodes (a root plus `depth` descendants), all
/// created by `caller`, who becomes the owner of the root's freshly
/// allocated Scope. Returns `(root, deepest_node)`.
fn create_chain<T: Config>(caller: &T::AccountId, depth: u32) -> (NodeId, NodeId) {
    let mut parent = None;
    let mut root = None;
    for _ in 0..=depth {
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
    (root.unwrap(), parent.unwrap())
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

/// Fill the parent's index to its limit, leaving the target as its last child.
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

/// Grant `Capability::Write` to `T::MaxAccessEntriesPerScope::get()` distinct
/// principals at `node`, filling that Scope's `Access` entries to its bound -
/// the worst case for any operation that must synchronously clear a Scope's
/// entire `Access` prefix (`delete_scope`, `create_scope` replacement).
fn fill_access_to_limit<T: Config>(caller: &T::AccountId, node: NodeId) {
    let limit = T::MaxAccessEntriesPerScope::get();
    for i in 0..limit {
        let principal = account::<T::AccountId>("access-limit", i, 0);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller.clone()).into(),
            node,
            principal,
            Capability::Write,
            GrantMode::Node,
        ));
    }
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_node(m: Linear<0, MAX_META_SIZE>, p: Linear<0, MAX_PAYLOAD_SIZE>) {
        let caller: T::AccountId = whitelisted_caller();
        let (_, parent) = create_chain::<T>(&caller, MAX_TREE_DEPTH - 1);
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

        assert_eq!(Parents::<T>::get(node), Some(Some(parent)));
        assert_eq!(Meta::<T>::get(node), meta);
        assert_eq!(Payload::<T>::get(node), payload);
        assert_eq!(
            NodesByParent::<T>::get(parent).len(),
            MAX_CHILDREN_PER_NODE as usize
        );
    }

    /// Worst case: `sender` is not the Scope owner and is authorized through
    /// an `inherited = true` `Write` `Access` granted at the Scope root,
    /// requiring a full `MAX_TREE_DEPTH` walk to be validated.
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

        let (root, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
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
    /// an `inherited = true` `Write` `Access` granted at the Scope root,
    /// requiring a full `MAX_TREE_DEPTH` walk to be validated.
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

        let (root, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
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

    #[benchmark]
    fn delete_node() {
        let caller: T::AccountId = whitelisted_caller();

        let (_, parent) = create_chain::<T>(&caller, MAX_TREE_DEPTH - 1);
        fill_siblings::<T>(&caller, parent, MAX_CHILDREN_PER_NODE - 1);
        let node = NextNodeId::<T>::get();
        assert_ok!(Pallet::<T>::create_node(
            RawOrigin::Signed(caller.clone()).into(),
            Some(parent),
            Some(max_meta()),
            Some(max_payload()),
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), node);

        assert!(!Parents::<T>::contains_key(node));
        assert!(!Meta::<T>::contains_key(node));
        assert!(!Payload::<T>::contains_key(node));
        assert_eq!(
            NodesByParent::<T>::get(parent).len(),
            (MAX_CHILDREN_PER_NODE - 1) as usize
        );
    }

    /// Worst case: `sender` is not the Scope owner and is authorized
    /// through an `inherited = true` `CreateScope` `Access` granted at the
    /// Scope root, requiring both a full `resolve_scope` walk from `node`
    /// up to the root Scope (to find the grant's Scope in the first
    /// place), and a full `authorize` walk back from `node` towards that
    /// same root (to find the `Subtree` grant, which only lives at the
    /// root and is never found before the last hop).
    #[benchmark]
    fn create_scope() {
        let caller: T::AccountId = whitelisted_caller();
        let accessor: T::AccountId = account("accessor", 0, 0);
        let (root, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller).into(),
            root,
            accessor.clone(),
            Capability::CreateScope,
            GrantMode::Subtree,
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(accessor.clone()), node);

        assert_eq!(
            ActiveScope::<T>::get(node).map(|(_, owner)| owner),
            Some(accessor)
        );
    }

    /// Worst case: `node`'s Scope holds `MaxAccessEntriesPerScope` physical
    /// `Access` entries, all of which must be synchronously cleared (in a
    /// single bounded `clear_prefix` call) as part of this extrinsic.
    #[benchmark]
    fn delete_scope() {
        let caller: T::AccountId = whitelisted_caller();
        let (_, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::create_scope(
            RawOrigin::Signed(caller.clone()).into(),
            node,
        ));
        fill_access_to_limit::<T>(&caller, node);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), node);

        assert!(!ActiveScope::<T>::contains_key(node));
    }

    /// Worst case: `node` is at `MAX_TREE_DEPTH`, exercising the full
    /// `resolve_scope` walk before the `Access` entry is written.
    #[benchmark]
    fn grant_access() {
        let caller: T::AccountId = whitelisted_caller();
        let principal: T::AccountId = account("principal", 0, 0);
        let (_, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);

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
    }

    /// Worst case: `node` is at `MAX_TREE_DEPTH`, exercising the full
    /// `resolve_scope` walk before the `Access` entry is removed.
    #[benchmark]
    fn revoke_access() {
        let caller: T::AccountId = whitelisted_caller();
        let principal: T::AccountId = account("principal", 0, 0);
        let (_, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
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
    /// of [`Pallet::resolve_scope`] alone: a `MAX_TREE_DEPTH` walk with no
    /// active Scope until the root.
    #[benchmark(extra)]
    fn resolve_scope_worst_case() {
        let caller: T::AccountId = whitelisted_caller();
        let (_, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);

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
        let (root, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
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
    /// (the cost `create_scope` replacement and `delete_scope` add on top
    /// of their other bookkeeping): a Scope filled to
    /// `MaxAccessEntriesPerScope` entries, cleared via
    /// `Pallet::clear_scope_access` in a single `clear_prefix` call.
    #[benchmark(extra)]
    fn clear_scope_access_worst_case() {
        let caller: T::AccountId = whitelisted_caller();
        let (root, _) = create_chain::<T>(&caller, 0);
        let (scope_id, _) = ActiveScope::<T>::get(root).expect("root has a Scope");
        fill_access_to_limit::<T>(&caller, root);
        assert_eq!(
            AccessCount::<T>::get(scope_id),
            T::MaxAccessEntriesPerScope::get()
        );

        #[block]
        {
            Pallet::<T>::clear_scope_access(scope_id);
        }

        assert_eq!(Access::<T>::iter_prefix(scope_id).count(), 0);
        assert_eq!(AccessCount::<T>::get(scope_id), 0);
    }

    /// Diagnostic (non-dispatchable) benchmark measuring the worst-case cost
    /// of `create_scope` on the *replacement* path (`node` already roots an
    /// active Scope, filled to `MaxAccessEntriesPerScope` entries), as
    /// opposed to the dispatchable `create_scope()` benchmark above, which
    /// covers the (also worst-case, but structurally different) brand-new
    /// nested Scope path requiring a full authorization walk. The two
    /// worst cases are mutually exclusive within a single call - replacing
    /// an existing Scope resolves and authorizes in O(1) at the exact root,
    /// while establishing a brand-new Scope on a deep descendant has no old
    /// Scope to clear - so both are benchmarked independently.
    #[benchmark(extra)]
    fn create_scope_replace_worst_case() {
        let caller: T::AccountId = whitelisted_caller();
        let (_, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::create_scope(
            RawOrigin::Signed(caller.clone()).into(),
            node,
        ));
        fill_access_to_limit::<T>(&caller, node);

        #[block]
        {
            assert_ok!(Pallet::<T>::create_scope(
                RawOrigin::Signed(caller).into(),
                node,
            ));
        }
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Runtime);
}
