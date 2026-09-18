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

/// Build the longest inherited-ownership path used by the measured operation.
fn create_chain<T: Config>(caller: &T::AccountId, depth: u32) -> NodeId {
    let mut parent = None;
    for _ in 0..=depth {
        let id = NextNodeId::<T>::get();
        assert_ok!(Pallet::<T>::create_node(
            RawOrigin::Signed(caller.clone()).into(),
            parent,
            None,
            None,
        ));
        parent = Some(id);
    }
    parent.unwrap()
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
        let parent = create_chain::<T>(&caller, MAX_TREE_DEPTH - 1);
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

        assert_eq!(Parent::<T>::get(node), Some(Some(parent)));
        assert_eq!(Meta::<T>::get(node), meta);
        assert_eq!(Payload::<T>::get(node), payload);
        assert_eq!(
            NodesByParent::<T>::get(parent).len(),
            MAX_CHILDREN_PER_NODE as usize
        );
    }

    #[benchmark]
    fn set_meta() {
        let caller: T::AccountId = whitelisted_caller();

        let node = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        let meta = Some(maximum_data());
        Meta::<T>::insert(node, maximum_data());

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), node, meta.clone());

        assert_eq!(Meta::<T>::get(node), meta);
    }

    #[benchmark]
    fn set_payload() {
        let caller: T::AccountId = whitelisted_caller();

        let node = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        Meta::<T>::insert(node, maximum_data());
        Payload::<T>::insert(node, maximum_data());
        let payload = Some(maximum_data());

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), node, payload.clone());

        assert_eq!(Payload::<T>::get(node), payload);
    }

    #[benchmark]
    fn transfer_ownership() {
        let caller: T::AccountId = whitelisted_caller();
        let new_owner: T::AccountId = account("new_owner", 0, 0);

        let node = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::transfer_ownership(
            RawOrigin::Signed(caller.clone()).into(),
            node,
            caller.clone(),
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), node, new_owner.clone());

        assert_eq!(
            <PendingOwnershipTransfer<T>>::get(node),
            Some(PendingTransfer {
                proposed_owner: new_owner,
                boundary: NodeId(0),
                generation: 0,
            })
        );
    }

    #[benchmark]
    fn accept_ownership() {
        let caller: T::AccountId = whitelisted_caller();
        let new_owner: T::AccountId = account("new_owner", 0, 0);

        let node = create_chain::<T>(&caller, MAX_TREE_DEPTH);
        assert_ok!(Pallet::<T>::transfer_ownership(
            RawOrigin::Signed(caller).into(),
            node,
            new_owner.clone(),
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(new_owner.clone()), node);

        assert_eq!(<Ownership<T>>::get(node), Some(new_owner));
        assert_eq!(OwnershipGeneration::<T>::get(node), 1);
        assert!(!PendingOwnershipTransfer::<T>::contains_key(node));
    }

    #[benchmark]
    fn delete_node() {
        let caller: T::AccountId = whitelisted_caller();

        let parent = create_chain::<T>(&caller, MAX_TREE_DEPTH - 1);
        fill_siblings::<T>(&caller, parent, MAX_CHILDREN_PER_NODE - 1);
        let node = NextNodeId::<T>::get();
        assert_ok!(Pallet::<T>::create_node(
            RawOrigin::Signed(caller.clone()).into(),
            Some(parent),
            Some(maximum_data()),
            Some(maximum_data()),
        ));
        assert_ok!(Pallet::<T>::transfer_ownership(
            RawOrigin::Signed(caller.clone()).into(),
            node,
            caller.clone(),
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), node);

        assert!(!Parent::<T>::contains_key(node));
        assert!(!Meta::<T>::contains_key(node));
        assert!(!Payload::<T>::contains_key(node));
        assert!(!PendingOwnershipTransfer::<T>::contains_key(node));
        assert_eq!(
            NodesByParent::<T>::get(parent).len(),
            (MAX_CHILDREN_PER_NODE - 1) as usize
        );
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Runtime);
}
