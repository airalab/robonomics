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
// Benchmarks for Subscription Pallet v2.0

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::{
    assert_ok,
    pallet_prelude::Get,
    traits::{
        fungibles::{Inspect, Mutate},
        Currency, Time,
    },
};
use frame_system::RawOrigin;
use sp_std::prelude::*;

type AssetBalanceOf<T> =
    <<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

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
        // Advance block number which will also advance timestamp in the runtime
        frame_system::Pallet::<T>::set_block_number(1000u32.into());

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), auction_id, None);
    }

    // NOTE: The `call` benchmark is omitted because it requires constructing a value of
    // type `<T as Config>::Call`, which is typically `RuntimeCall` at the runtime level.
    // This type cannot be constructed generically at the pallet level without runtime-specific
    // knowledge. The benchmark should be implemented at the runtime level using
    // frame-benchmarking-cli if needed for production weight calculations.
    //
    // For reference, the call extrinsic signature is:
    // pub fn call(origin: OriginFor<T>, subscription_id: u32, call: Box<<T as Config>::Call>)
    //
    // To benchmark this in a specific runtime:
    // 1. Use the runtime's RuntimeCall type directly
    // 2. Or add a BenchmarkHelper trait to the pallet Config that provides a method to
    //    construct sample Call values for benchmarking

    #[benchmark]
    fn start_lifetime() {
        // Get a whitelisted caller
        let caller: T::AccountId = whitelisted_caller();
        let amount: AssetBalanceOf<T> = 1000u32.into();

        // Get the asset ID from config
        let asset_id = T::LifetimeAssetId::get();

        // Mint assets to the caller using the Assets trait
        // This assumes the runtime benchmark environment has the asset created
        let _ = T::Assets::mint_into(asset_id, &caller, amount * 10u32.into());

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), amount);
    }

    #[benchmark]
    fn stop_lifetime() {
        // Setup: Create asset-locked subscription first
        let caller: T::AccountId = whitelisted_caller();
        let amount: AssetBalanceOf<T> = 1000u32.into();
        let asset_id = T::LifetimeAssetId::get();

        // Mint assets to the caller
        let _ = T::Assets::mint_into(asset_id, &caller, amount * 10u32.into());

        // Create the subscription via start_lifetime
        assert_ok!(Pallet::<T>::start_lifetime(
            RawOrigin::Signed(caller.clone()).into(),
            amount
        ));

        let subscription_id = 0u32;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subscription_id);
    }

    #[benchmark]
    fn grant_access() {
        // Setup: Create a subscription first
        let caller: T::AccountId = whitelisted_caller();
        let delegate: T::AccountId = account("delegate", 0, SEED);
        let amount: AssetBalanceOf<T> = 1000u32.into();
        let asset_id = T::LifetimeAssetId::get();

        // Mint assets and create subscription
        let _ = T::Assets::mint_into(asset_id, &caller, amount * 10u32.into());
        assert_ok!(Pallet::<T>::start_lifetime(
            RawOrigin::Signed(caller.clone()).into(),
            amount
        ));

        let subscription_id = 0u32;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subscription_id, delegate);
    }

    #[benchmark]
    fn revoke_access() {
        // Setup: Create a subscription and grant access first
        let caller: T::AccountId = whitelisted_caller();
        let delegate: T::AccountId = account("delegate", 0, SEED);
        let amount: AssetBalanceOf<T> = 1000u32.into();
        let asset_id = T::LifetimeAssetId::get();

        // Mint assets and create subscription
        let _ = T::Assets::mint_into(asset_id, &caller, amount * 10u32.into());
        assert_ok!(Pallet::<T>::start_lifetime(
            RawOrigin::Signed(caller.clone()).into(),
            amount
        ));

        let subscription_id = 0u32;

        // Grant access first
        assert_ok!(Pallet::<T>::grant_access(
            RawOrigin::Signed(caller.clone()).into(),
            subscription_id,
            delegate.clone()
        ));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subscription_id, delegate);
    }

    impl_benchmark_test_suite!(Rws, crate::tests::new_test_ext(), crate::tests::Runtime);
}
