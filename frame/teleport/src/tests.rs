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
//! Unit tests for the Robonomics Teleport pallet.
//!
//! These tests verify the core functionality of the `send` extrinsic using mock
//! XCM components. For comprehensive XCM delivery testing with actual cross-chain
//! message delivery, see XCM_SIMULATOR_TESTING.md.

use crate as pallet_robonomics_teleport;
use frame_support::{
    assert_ok, derive_impl, parameter_types,
    traits::ConstU64,
    dispatch::PostDispatchInfo,
};
use sp_runtime::{traits::IdentityLookup, BuildStorage, DispatchError, DispatchErrorWithPostInfo};
use xcm::prelude::*;
use xcm_builder::{ExecuteController, SendController};

type Block = frame_system::mocking::MockBlock<Runtime>;
type Balance = u64;

// Mock constants for XCM testing
const MOCK_XCM_HASH: XcmHash = [0u8; 32];
const MOCK_REF_TIME: u64 = 1000;
const MOCK_PROOF_SIZE: u64 = 1000;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Runtime
    {
        System: frame_system,
        Balances: pallet_balances,
        RobonomicsTeleport: pallet_robonomics_teleport,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type ExistentialDeposit = ConstU64<1>;
    type AccountStore = System;
}

// Mock XCM controller that supports both execute and send
pub struct MockXcmController;

impl SendXcm for MockXcmController {
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

impl<Call> ExecuteXcm<Call> for MockXcmController {
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

impl SendController<RuntimeOrigin> for MockXcmController {
    type WeightInfo = ();

    fn send(
        _origin: RuntimeOrigin,
        _dest: Box<VersionedLocation>,
        _message: Box<VersionedXcm<()>>,
    ) -> Result<XcmHash, DispatchError> {
        Ok(MOCK_XCM_HASH)
    }
}

impl ExecuteController<RuntimeOrigin, RuntimeCall> for MockXcmController {
    type WeightInfo = ();

    fn execute(
        _origin: RuntimeOrigin,
        _message: Box<VersionedXcm<RuntimeCall>>,
        _max_weight: Weight,
    ) -> Result<Weight, DispatchErrorWithPostInfo<PostDispatchInfo>> {
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

parameter_types! {
    /// Target location for teleports in tests (e.g. an asset hub parachain).
    pub TargetLocationTest: Location = Location::new(1, [Parachain(1000)]);

    /// Default fee asset used by the teleport pallet in tests.
    /// Amount: 1000 units of relay chain asset for testing
    pub FeeAssetTest: Asset = Asset {
        id: AssetId(Location::parent()),
        fun: Fungibility::Fungible(1000),
    };

    /// Parachain location for refund tests
    pub ParachainLocationTest: Location = Location::new(1, [Parachain(2000)]);

    /// Universal location for asset reanchoring
    pub UniversalLocationTest: InteriorLocation = [GlobalConsensus(NetworkId::Kusama), Parachain(2000)].into();

    /// Native asset ID (here means native asset)
    pub NativeAssetIdTest: AssetId = AssetId(Location::here());

    /// Max weight for local XCM execution
    pub MaxWeightTest: Weight = Weight::from_parts(10_000_000, 10_000);
}

impl pallet_robonomics_teleport::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmPallet = MockXcmController;
    type WeightInfo = crate::weights::TestWeightInfo;
    type MaxWeight = MaxWeightTest;
    type AssetId = NativeAssetIdTest;
    type FeeAsset = FeeAssetTest;
    type TargetLocation = TargetLocationTest;
    type ParachainLocation = ParachainLocationTest;
    type UniversalLocation = UniversalLocationTest;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(1, 1000), (2, 2000), (3, 3000)],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}

#[test]
fn test_send_success() {
    new_test_ext().execute_with(|| {
        // Initialize block to avoid events warning
        System::set_block_number(1);

        let origin = 1u64;
        let beneficiary_id = [2u8; 32];
        let beneficiary = Location::new(
            0,
            [AccountId32 {
                network: None,
                id: beneficiary_id,
            }],
        );
        let amount: u128 = 100;

        // Execute send extrinsic
        assert_ok!(RobonomicsTeleport::send(
            RuntimeOrigin::signed(origin),
            beneficiary.clone(),
            amount,
        ));

        // Verify the Teleported event was emitted with correct parameters
        System::assert_last_event(
            pallet_robonomics_teleport::Event::Teleported {
                origin,
                beneficiary,
                amount,
                xcm_hash: MOCK_XCM_HASH,
            }
            .into(),
        );
    });
}

#[test]
fn test_send_with_maximum_balance() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let origin = 1u64;
        let beneficiary_id = [2u8; 32];
        let beneficiary = Location::new(
            0,
            [AccountId32 {
                network: None,
                id: beneficiary_id,
            }],
        );
        let amount = 1000u128; // Entire balance

        // Send entire balance should succeed (balance validation happens in XCM execution)
        assert_ok!(RobonomicsTeleport::send(
            RuntimeOrigin::signed(origin),
            beneficiary,
            amount,
        ));
    });
}

#[test]
fn test_send_with_different_beneficiaries() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let origin = 1u64;
        let amount = 50u128;

        // Test with various beneficiary addresses
        let beneficiary_ids = vec![[0u8; 32], [1u8; 32], [255u8; 32]];

        for beneficiary_id in beneficiary_ids {
            let beneficiary = Location::new(
                0,
                [AccountId32 {
                    network: None,
                    id: beneficiary_id,
                }],
            );
            assert_ok!(RobonomicsTeleport::send(
                RuntimeOrigin::signed(origin),
                beneficiary,
                amount,
            ));
        }
    });
}

#[test]
fn test_send_with_varying_amounts() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let origin = 2u64;
        let beneficiary_id = [3u8; 32];
        let beneficiary = Location::new(
            0,
            [AccountId32 {
                network: None,
                id: beneficiary_id,
            }],
        );

        // Test with different amounts
        let amounts = vec![10u128, 50u128, 100u128, 500u128];

        for amount in amounts {
            assert_ok!(RobonomicsTeleport::send(
                RuntimeOrigin::signed(origin),
                beneficiary.clone(),
                amount,
            ));
        }
    });
}
