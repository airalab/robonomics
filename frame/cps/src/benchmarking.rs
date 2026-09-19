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

/// Use the full bound so reads and writes account for maximum encoded data.
fn maximum_data() -> NodeData {
    BoundedVec::try_from(vec![1u8; MAX_DATA_SIZE as usize]).unwrap()
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

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_node() {
        let caller: T::AccountId = whitelisted_caller();
        let (_, parent) = create_chain::<T>(&caller, MAX_TREE_DEPTH - 1);
        fill_siblings::<T>(&caller, parent, MAX_CHILDREN_PER_NODE - 1);
        let node = NextNodeId::<T>::get();
        let meta = Some(maximum_data());
        let payload = Some(maximum_data());

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
    #[benchmark]
    fn set_meta() {
        let caller: T::AccountId = whitelisted_caller();
        let accessor: T::AccountId = account("accessor", 0, 0);

        let (root, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller).into(),
            root,
            accessor.clone(),
            Capability::Write,
            true,
        ));
        Meta::<T>::insert(node, maximum_data());
        let meta = Some(maximum_data());

        #[extrinsic_call]
        _(RawOrigin::Signed(accessor), node, meta.clone());

        assert_eq!(Meta::<T>::get(node), meta);
    }

    /// Worst case: `sender` is not the Scope owner and is authorized through
    /// an `inherited = true` `Write` `Access` granted at the Scope root,
    /// requiring a full `MAX_TREE_DEPTH` walk to be validated.
    #[benchmark]
    fn set_payload() {
        let caller: T::AccountId = whitelisted_caller();
        let accessor: T::AccountId = account("accessor", 0, 0);

        let (root, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller).into(),
            root,
            accessor.clone(),
            Capability::Write,
            true,
        ));
        Meta::<T>::insert(node, maximum_data());
        Payload::<T>::insert(node, maximum_data());
        let payload = Some(maximum_data());

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
            Some(maximum_data()),
            Some(maximum_data()),
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

    /// Worst case: `node` is at `MAX_TREE_DEPTH`, exercising the full
    /// `resolve_scope` walk before the new Scope is allocated.
    #[benchmark]
    fn create_scope() {
        let caller: T::AccountId = whitelisted_caller();
        let (_, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), node);

        let scope_id = ActiveScope::<T>::get(node).expect("scope just created");
        assert_eq!(ScopeOwner::<T>::get(scope_id), Some(caller));
    }

    #[benchmark]
    fn delete_scope() {
        let caller: T::AccountId = whitelisted_caller();
        let (_, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::create_scope(
            RawOrigin::Signed(caller.clone()).into(),
            node,
        ));

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
            true,
        );

        let scope_id = Pallet::<T>::resolve_scope(node).expect("scope resolves");
        assert_eq!(
            Access::<T>::get(scope_id, (node, principal, Capability::Write)),
            Some(true)
        );
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
            true,
        ));

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            node,
            principal.clone(),
            Capability::Write,
        );

        let scope_id = Pallet::<T>::resolve_scope(node).expect("scope resolves");
        assert!(!Access::<T>::contains_key(
            scope_id,
            (node, principal, Capability::Write)
        ));
    }

    /// Diagnostic (non-dispatchable) benchmark measuring the worst-case cost
    /// of [`Pallet::resolve_scope`] alone: a `MAX_TREE_DEPTH` walk with no
    /// active Scope until the root.
    #[benchmark]
    fn resolve_scope_worst_case() {
        let caller: T::AccountId = whitelisted_caller();
        let (_, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);

        #[block]
        {
            assert!(Pallet::<T>::resolve_scope(node).is_ok());
        }
    }

    /// Diagnostic (non-dispatchable) benchmark measuring the worst-case cost
    /// of an `Access` traversal: an `inherited = true` `Write` grant at the
    /// Scope root, checked from the deepest descendant.
    #[benchmark]
    fn access_traversal_worst_case() {
        let caller: T::AccountId = whitelisted_caller();
        let accessor: T::AccountId = account("accessor", 0, 0);
        let (root, node) = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller).into(),
            root,
            accessor.clone(),
            Capability::Write,
            true,
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

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Runtime);
}
