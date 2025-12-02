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
// Benchmarks for RWS Pallet v2.0

#![cfg(feature = "runtime-benchmarks")]

use super::{Pallet as Rws, *};
use frame_benchmarking::v2::*;
use frame_support::{assert_ok, pallet_prelude::Get, traits::Currency};
use frame_system::RawOrigin;
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

    #[benchmark]
    fn start_auction() {
        #[extrinsic_call]
        _(RawOrigin::Root, SubscriptionMode::Lifetime { tps: 1000 });
    }

    #[benchmark]
    fn bid() {
        // Start an auction first
        let mode = SubscriptionMode::Lifetime { tps: 1000 };
        assert_ok!(Pallet::<T>::start_auction(RawOrigin::Root.into(), mode));

        let caller = funded_account::<T>("caller", 0);
        let auction_id = 0u32; // First auction
        let amount = T::MinimalBid::get() * 10u32.into();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), auction_id, amount);
    }

    #[benchmark]
    fn claim() {
        // Start an auction and place a bid
        let mode = SubscriptionMode::Lifetime { tps: 1000 };
        assert_ok!(Pallet::<T>::start_auction(RawOrigin::Root.into(), mode));

        let caller = funded_account::<T>("caller", 0);
        let auction_id = 0u32;
        let amount = T::MinimalBid::get() * 10u32.into();
        assert_ok!(Pallet::<T>::bid(
            RawOrigin::Signed(caller.clone()).into(),
            auction_id,
            amount
        ));

        // Fast forward time past auction duration
        let now = T::Time::now();
        <T::Time as frame_support::traits::Time>::set_timestamp(
            now + T::AuctionDuration::get() + 1u32.into(),
        );

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), auction_id, None);
    }

    #[benchmark]
    fn call() {
        // Create a subscription first via auction
        let mode = SubscriptionMode::Lifetime { tps: 100_000 };
        assert_ok!(Pallet::<T>::start_auction(RawOrigin::Root.into(), mode));

        let caller = funded_account::<T>("caller", 0);
        let auction_id = 0u32;
        let amount = T::MinimalBid::get() * 10u32.into();
        assert_ok!(Pallet::<T>::bid(
            RawOrigin::Signed(caller.clone()).into(),
            auction_id,
            amount
        ));

        // Fast forward and claim
        let now = T::Time::now();
        <T::Time as frame_support::traits::Time>::set_timestamp(
            now + T::AuctionDuration::get() + 1u32.into(),
        );
        assert_ok!(Pallet::<T>::claim(
            RawOrigin::Signed(caller.clone()).into(),
            auction_id,
            None
        ));

        let subscription_id = 0u32;
        let inner_call = frame_system::Call::<T>::remark {
            remark: vec![0u8; 100],
        };
        let boxed_call = Box::new(<T as Config>::RuntimeCall::from(inner_call));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subscription_id, boxed_call);
    }

    #[benchmark]
    fn start_lifetime() {
        // This is a placeholder benchmark - actual implementation would need
        // proper setup of Assets pallet and funded accounts
        let caller = funded_account::<T>("caller", 0);
        let amount = 1000u32.into();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), amount);
    }

    #[benchmark]
    fn stop_lifetime() {
        // This is a placeholder benchmark - actual implementation would need
        // proper setup with an existing asset-locked subscription
        let caller = funded_account::<T>("caller", 0);
        let subscription_id = 0u32;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subscription_id);
    }

    impl_benchmark_test_suite!(Rws, crate::tests::new_test_ext(), crate::tests::Runtime);
}
