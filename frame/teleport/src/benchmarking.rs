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
use xcm::prelude::*;

use frame_support::{dispatch::DispatchErrorWithPostInfo, traits::Get};
use frame_system::RawOrigin;
use sp_runtime::DispatchError;
use sp_std::boxed::Box;
use xcm_builder::{ExecuteController, SendController};
use xcm_executor::traits::TransactAsset;

#[benchmarks(where Location: From<<T as frame_system::Config>::AccountId>)]
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

        let asset = T::AssetId::get().into_asset(Fungibility::Fungible(amount));
        let caller_location: Location = caller.clone().into();
        T::AssetTransactor::deposit_asset(&asset, &caller_location, None).unwrap();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), beneficiary, amount);

        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Runtime);
}

// Mock constants for XCM benchmarks
const MOCK_XCM_HASH: XcmHash = [0u8; 32];
const MOCK_REF_TIME: u64 = 0;
const MOCK_PROOF_SIZE: u64 = 0;

// Mock XCM controller that supports both execute and send
pub struct BenchmarkingXcmController;

impl SendXcm for BenchmarkingXcmController {
    type Ticket = ();

    fn validate(
        _dest: &mut Option<Location>,
        _msg: &mut Option<Xcm<()>>,
    ) -> SendResult<Self::Ticket> {
        // Always succeed for testing
        Ok(((), Assets::new()))
    }

    fn deliver(_ticket: Self::Ticket) -> Result<XcmHash, SendError> {
        Ok(MOCK_XCM_HASH)
    }
}

impl<Call> ExecuteXcm<Call> for BenchmarkingXcmController {
    type Prepared = MockPreparedMessage;

    fn prepare_and_execute(
        _origin: impl Into<Location>,
        _message: Xcm<Call>,
        _id: &mut XcmHash,
        _weight_limit: Weight,
        _weight_credit: Weight,
    ) -> Outcome {
        // Return success for testing
        Outcome::Complete {
            used: Weight::from_parts(1000, 1000),
        }
    }

    fn prepare(
        _message: Xcm<Call>,
        _weight_limit: Weight,
    ) -> Result<Self::Prepared, InstructionError> {
        Ok(MockPreparedMessage)
    }

    fn execute(
        _origin: impl Into<Location>,
        _prepared: Self::Prepared,
        _id: &mut XcmHash,
        _weight_credit: Weight,
    ) -> Outcome {
        Outcome::Complete {
            used: Weight::from_parts(1000, 1000),
        }
    }

    fn charge_fees(_: impl Into<Location>, _: Assets) -> Result<(), xcm::v5::Error> {
        Ok(())
    }
}

impl<RuntimeOrigin> SendController<RuntimeOrigin> for BenchmarkingXcmController {
    type WeightInfo = ();

    fn send(
        _origin: RuntimeOrigin,
        _dest: Box<VersionedLocation>,
        _message: Box<VersionedXcm<()>>,
    ) -> Result<XcmHash, DispatchError> {
        Ok(MOCK_XCM_HASH)
    }
}

impl<RuntimeOrigin, RuntimeCall> ExecuteController<RuntimeOrigin, RuntimeCall>
    for BenchmarkingXcmController
{
    type WeightInfo = ();

    fn execute(
        _origin: RuntimeOrigin,
        _message: Box<VersionedXcm<RuntimeCall>>,
        _max_weight: Weight,
    ) -> Result<Weight, DispatchErrorWithPostInfo> {
        Ok(Weight::from_parts(MOCK_REF_TIME, MOCK_PROOF_SIZE))
    }
}

// Mock XCM executor (unused in current implementation but kept for testing infrastructure)
pub struct MockPreparedMessage;
impl PreparedMessage for MockPreparedMessage {
    fn weight_of(&self) -> Weight {
        Weight::from_parts(MOCK_REF_TIME, MOCK_PROOF_SIZE)
    }
}

pub struct MockXcmExecutor;
impl<Call> ExecuteXcm<Call> for MockXcmExecutor {
    type Prepared = MockPreparedMessage;

    fn prepare_and_execute(
        _origin: impl Into<Location>,
        _message: Xcm<Call>,
        _id: &mut XcmHash,
        _weight_limit: Weight,
        _weight_credit: Weight,
    ) -> Outcome {
        Outcome::Complete {
            used: Weight::from_parts(MOCK_REF_TIME, MOCK_PROOF_SIZE),
        }
    }

    fn prepare(
        _message: Xcm<Call>,
        _weight_limit: Weight,
    ) -> Result<Self::Prepared, InstructionError> {
        Ok(MockPreparedMessage)
    }

    fn execute(
        _origin: impl Into<Location>,
        _prepared: Self::Prepared,
        _id: &mut XcmHash,
        _weight_credit: Weight,
    ) -> Outcome {
        Outcome::Complete {
            used: Weight::from_parts(MOCK_REF_TIME, MOCK_PROOF_SIZE),
        }
    }

    fn charge_fees(_location: impl Into<Location>, _fees: Assets) -> Result<(), xcm::v5::Error> {
        Ok(())
    }
}
