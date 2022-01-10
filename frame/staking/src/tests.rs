///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! Pallet test cases.

use frame_support::{assert_err, assert_ok, parameter_types, traits::GenesisBuild};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};

use crate::{self as staking, *};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;
type Balance = u128;

const XRT: Balance = 1_000_000_000;

const ALICE: u64 = 1;
const ALICE_C: u64 = 10;

const BOB: u64 = 2;
const BOB_C: u64 = 20;

const CHARLIE: u64 = 3;
const CHARLIE_C: u64 = 30;

frame_support::construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Staking: staking::{Pallet, Call, Storage, Event<T>, Config<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = Call;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = frame_support::traits::Everything;
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

parameter_types! {
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
    pub const ExistentialDeposit: Balance = 10;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const BondingDuration: u64 = 32;
    pub const StakeReward: Perbill = Perbill::from_parts(40);
    pub const BonusReward: Perbill = Perbill::from_parts(200);
}

impl Config for Runtime {
    type Currency = Balances;
    type Event = Event;
    type OnBond = ();

    type BondingDuration = BondingDuration;
    type StakeReward = StakeReward;
    type BonusReward = BonusReward;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let _ = pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(ALICE, 10 * XRT), (BOB, 42 * XRT), (CHARLIE, 10_000 * XRT)],
    }
    .assimilate_storage(&mut storage);

    let _ = staking::GenesisConfig::<Runtime> {
        bonus: vec![(BOB, 30 * XRT)],
    }
    .assimilate_storage(&mut storage);

    storage.into()
}

fn events() -> Vec<Event> {
    System::events().iter().map(|e| e.event.clone()).collect()
}

#[test]
fn bond_should_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_eq!(System::account(ALICE).data.free, 10 * XRT);

        assert_ok!(Staking::bond(Origin::signed(ALICE), ALICE_C, 5 * XRT));
        assert_eq!(<Bonded<Runtime>>::get(ALICE), Some(ALICE_C));
        assert_eq!(
            events(),
            vec![staking::Event::Bonded(ALICE, 5 * XRT).into(),]
        );
        assert_eq!(
            <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.claimed_rewards),
            Some(1),
        );
        assert_eq!(Balances::locks(ALICE)[0].amount, 5 * XRT);
    })
}

#[test]
fn unbond_should_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Staking::bond(Origin::signed(ALICE), ALICE_C, 5 * XRT));
        assert_eq!(Balances::locks(ALICE)[0].amount, 5 * XRT);

        System::set_block_number(2);
        assert_ok!(Staking::unbond(Origin::signed(ALICE_C), 2 * XRT));
        assert_eq!(<Bonded<Runtime>>::get(ALICE), Some(ALICE_C));
        assert_eq!(
            <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.active),
            Some(3 * XRT),
        );
        assert_eq!(
            <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.unlocking),
            Some(vec![UnlockChunk {
                value: 2 * XRT,
                moment: 34
            }]),
        );
        assert_eq!(Balances::locks(ALICE)[0].amount, 5 * XRT);

        System::set_block_number(20);
        assert_ok!(Staking::withdraw_unbonded(Origin::signed(ALICE_C)));
        assert_eq!(
            <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.unlocking),
            Some(vec![UnlockChunk {
                value: 2 * XRT,
                moment: 34
            }]),
        );

        System::set_block_number(35);
        assert_ok!(Staking::withdraw_unbonded(Origin::signed(ALICE_C)));
        assert_eq!(
            <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.unlocking),
            Some(vec![]),
        );
        assert_eq!(Balances::locks(ALICE)[0].amount, 3 * XRT);

        assert_ok!(Staking::unbond(Origin::signed(ALICE_C), 3 * XRT));
        System::set_block_number(100);
        assert_ok!(Staking::withdraw_unbonded(Origin::signed(ALICE_C)));
        assert_eq!(Balances::locks(ALICE), vec![]);
        assert_eq!(<Bonded<Runtime>>::get(ALICE), None);
    })
}

#[test]
fn bond_extra_should_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 1 * XRT));
        System::set_block_number(10);
        assert_eq!(System::account(BOB).data.free, 42000000000);

        assert_ok!(Staking::bond_extra(Origin::signed(BOB), 41 * XRT));
        assert_eq!(System::account(BOB).data.free, 42000001800);

        System::set_block_number(1_000);
        assert_ok!(Staking::claim_rewards(Origin::signed(BOB_C)));
        assert_eq!(System::account(BOB).data.free, 42006417000);
    })
}

#[test]
fn reward_should_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 42 * XRT));
        assert_ok!(Staking::bond(
            Origin::signed(CHARLIE),
            CHARLIE_C,
            10_000 * XRT
        ));

        System::set_block_number(1_000);
        assert_ok!(Staking::claim_rewards(Origin::signed(BOB_C)));
        assert_ok!(Staking::claim_rewards(Origin::signed(CHARLIE_C)));

        assert_eq!(
            <Ledger<Runtime>>::get(BOB_C).map(|ledger| ledger.claimed_rewards),
            Some(1_000),
        );
        assert_eq!(
            <Ledger<Runtime>>::get(CHARLIE_C).map(|ledger| ledger.claimed_rewards),
            Some(1_000),
        );
        assert_eq!(System::account(BOB).data.free, 42006473520);
        assert_eq!(System::account(CHARLIE).data.free, 10000399600000);

        System::set_block_number(5_000_000);
        assert_ok!(Staking::claim_rewards(Origin::signed(CHARLIE_C)));
        assert_eq!(System::account(CHARLIE).data.free, 11999999600000);
    })
}

#[test]
fn extend_bonus_should_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_err!(
            Staking::extend_bonus(Origin::signed(BOB), Default::default()),
            sp_runtime::traits::BadOrigin,
        );
        assert_eq!(Staking::bonus(BOB), Some(30000000000));

        assert_ok!(Staking::extend_bonus(Origin::root(), vec![(BOB, 100_000)]));
        assert_eq!(Staking::bonus(BOB), Some(30000100000));

        assert_ok!(Staking::extend_bonus(Origin::root(), vec![(BOB, 100_000)]));
        assert_eq!(Staking::bonus(BOB), Some(30000200000));
    })
}

#[test]
fn fail_double_bonding() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 1 * XRT));
        assert_err!(
            Staking::bond(Origin::signed(ALICE), BOB_C, 1 * XRT),
            staking::Error::<Runtime>::AlreadyPaired,
        );
        assert_err!(
            Staking::bond(Origin::signed(BOB), ALICE_C, 1 * XRT),
            staking::Error::<Runtime>::AlreadyBonded,
        );
    })
}

#[test]
fn fail_controller_calls() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 1 * XRT));
        assert_err!(
            Staking::claim_rewards(Origin::signed(BOB)),
            staking::Error::<Runtime>::NotController,
        );
        assert_err!(
            Staking::unbond(Origin::signed(BOB), 1 * XRT),
            staking::Error::<Runtime>::NotController,
        );
    })
}
