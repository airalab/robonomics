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
//! Benchmarking for pallet-robonomics-cps

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_node() {
        let caller: T::AccountId = whitelisted_caller();
        let meta = Some(NodeData::Plain(
            BoundedVec::try_from(vec![1u8; 100]).unwrap(),
        ));
        let payload = Some(NodeData::Plain(
            BoundedVec::try_from(vec![2u8; 100]).unwrap(),
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), None, meta, payload);

        assert_eq!(<NextNodeId<T>>::get(), 1);
    }

    #[benchmark]
    fn set_meta() {
        let caller: T::AccountId = whitelisted_caller();

        // Setup: create a node first
        let _ =
            Pallet::<T>::create_node(RawOrigin::Signed(caller.clone()).into(), None, None, None);

        let meta = Some(NodeData::Plain(
            BoundedVec::try_from(vec![1u8; 100]).unwrap(),
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), 0, meta);

        assert!(<Nodes<T>>::get(0).unwrap().meta.is_some());
    }

    #[benchmark]
    fn set_payload() {
        let caller: T::AccountId = whitelisted_caller();

        // Setup: create a node first
        let _ =
            Pallet::<T>::create_node(RawOrigin::Signed(caller.clone()).into(), None, None, None);

        let payload = Some(NodeData::Plain(
            BoundedVec::try_from(vec![1u8; 100]).unwrap(),
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), 0, payload);

        assert!(<Nodes<T>>::get(0).unwrap().payload.is_some());
    }

    #[benchmark]
    fn move_node() {
        let caller: T::AccountId = whitelisted_caller();

        // Setup: create parent node
        let _ =
            Pallet::<T>::create_node(RawOrigin::Signed(caller.clone()).into(), None, None, None);

        // Create child node
        let _ = Pallet::<T>::create_node(
            RawOrigin::Signed(caller.clone()).into(),
            Some(0),
            None,
            None,
        );

        // Create new parent
        let _ =
            Pallet::<T>::create_node(RawOrigin::Signed(caller.clone()).into(), None, None, None);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), 1, 2);

        assert_eq!(<Nodes<T>>::get(1).unwrap().parent, Some(2));
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Runtime);
}
