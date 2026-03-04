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
//! Benchmarks for Robonomics Teleport Pallet
//!
//! This module provides runtime benchmarks for the `send` extrinsic to determine
//! appropriate weight values for XCM teleport operations.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_runtime::traits::StaticLookup;
use xcm::prelude::*;

#[benchmarks(
    where
        T: pallet_balances::Config,
        <T as pallet_balances::Config>::Balance: From<u128>,
        <<T as frame_system::Config>::Lookup as StaticLookup>::Source: From<T::AccountId>,
)]
mod benchmarks {
    use super::*;

    /// Benchmark the `send` extrinsic
    ///
    /// This measures the computational cost of:
    /// - Constructing the XCM message
    /// - Validating and sending the message to Asset Hub
    /// - Emitting the Teleported event
    #[benchmark]
    fn send() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let beneficiary_id = [1u8; 32];
        let beneficiary = Location::new(
            0,
            [AccountId32 {
                network: None,
                id: beneficiary_id,
            }],
        );
        let amount: u128 = 1_000_000_000;

        // set beneficiary balance
        let _ = pallet_balances::Pallet::<T>::force_set_balance(
            RawOrigin::Root.into(),
            caller.clone().into(),
            (amount * 2).into(),
        );

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), beneficiary, amount);

        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Runtime);
}
