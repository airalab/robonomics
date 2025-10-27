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
// Benchmarks for RWS Pallet

#![cfg(feature = "runtime-benchmarks")]

use super::{Pallet as Rws, *};
use frame_benchmarking::v2::*;
use frame_support::{assert_ok, pallet_prelude::Get, traits::Currency};
use frame_system::RawOrigin;
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

const SEED: u32 = 0;

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);
    T::AuctionCurrency::make_free_balance_be(&caller, T::MinimalBid::get() * 100u32.into());
    caller
}

#[benchmarks]
mod benchmarks {
    use super::*;
    #[cfg(test)]
    use frame_system::RawOrigin;

    #[benchmark]
    fn bid() {
        let caller = funded_account::<T>("caller", 0);
        let _ = Pallet::<T>::start_auction(RawOrigin::Root.into(), Default::default());
        let queue = Pallet::<T>::auction_queue();
        let index = queue.first().unwrap();
        let amount = T::MinimalBid::get() * 10u32.into();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), index.clone(), amount);
    }

    #[benchmark]
    fn set_devices() {
        let caller: T::AccountId = whitelisted_caller();
        let device: T::AccountId = account("device", 2, SEED);
        let mut devices = frame_support::BoundedVec::new();
        assert_ok!(devices.try_push(device));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), devices);
    }

    #[benchmark]
    fn set_oracle() {
        let oracle = T::Lookup::unlookup(whitelisted_caller());

        #[extrinsic_call]
        _(RawOrigin::Root, oracle);
    }

    #[benchmark]
    fn set_subscription() {
        let oracle: T::AccountId = whitelisted_caller();
        let target: T::AccountId = account("target", 3, SEED);
        let oracle_lookup = T::Lookup::unlookup(oracle.clone());

        assert_ok!(Rws::<T>::set_oracle(RawOrigin::Root.into(), oracle_lookup));

        #[extrinsic_call]
        set_subscription(RawOrigin::Signed(oracle), target, Default::default());
    }

    #[benchmark]
    fn start_auction() {
        #[extrinsic_call]
        _(RawOrigin::Root, Default::default());
    }

    impl_benchmark_test_suite!(Rws, crate::tests::new_test_ext(), crate::tests::Runtime,);
}
