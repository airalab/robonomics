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
//! Benchmarks for Wrapped Asset Pallet

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_std::prelude::*;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn wrap_and_send() {
        // Setup: Create a funded account
        let caller: T::AccountId = whitelisted_caller();
        let amount = 1_000_000u32.into();

        // Fund the account with native tokens
        let _ = T::NativeCurrency::make_free_balance_be(&caller, amount + amount);

        // Set TotalWrapped to ensure there's enough to wrap
        TotalWrapped::<T>::put(amount + amount);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), amount, None);

        // Verify the wrapped amount was decreased
        assert!(TotalWrapped::<T>::get() < amount + amount);
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
