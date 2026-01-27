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
//! Robonomics Web Services pallet tests.

use crate::{self as rws, *};
use frame_support::{
    assert_err, assert_ok, parameter_types, traits::OnInitialize, weights::Weight,
};
use pallet_robonomics_datalog as datalog;
use pallet_robonomics_staking as staking;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchError, Perbill};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;
type Balance = u128;
type BlockNumber = u64;
type Moment = u64;

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
        Timestamp: pallet_timestamp::{Pallet, Storage},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Datalog: datalog::{Pallet, Call, Storage, Event<T>},
        Staking: staking::{Pallet, Call, Storage, Event<T>},
        RWS: rws::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = Call;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = frame_support::traits::Everything;
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type PalletInfo = PalletInfo;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = ();
    type WeightInfo = ();
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
    pub const WindowSize: u64 = 128;
    pub const MaximumMessageSize: usize = 512;
}

impl datalog::Config for Runtime {
    type Record = Vec<u8>;
    type Event = Event;
    type Time = Timestamp;
    type WindowSize = WindowSize;
    type MaximumMessageSize = MaximumMessageSize;
    type WeightInfo = ();
}

parameter_types! {
    pub const BondingDuration: BlockNumber = 1;
    pub const StakeReward: Perbill = Perbill::from_parts(40);
    pub const BonusReward: Perbill = Perbill::from_parts(200);
}

impl pallet_robonomics_staking::Config for Runtime {
    type Currency = Balances;
    type Event = Event;
    type BondingDuration = BondingDuration;
    type StakeReward = StakeReward;
    type BonusReward = BonusReward;
    type OnBond = RWS;
}

parameter_types! {
    pub const ReferenceCallWeight: Weight = 70_952_000;  // let it be transfer call weight
    pub const WeightLimit: Weight = Weight::max_value() / 2;
    pub const AuctionDuration: BlockNumber = 10;
    pub const AuctionCost: Balance = 5_000;
    pub const MinimalBid: Balance = 100;
}

impl Config for Runtime {
    type Call = Call;
    type Time = Timestamp;
    type Moment = Moment;
    type AuctionIndex = u32;
    type AuctionCurrency = Balances;
    type Event = Event;
    type ReferenceCallWeight = ReferenceCallWeight;
    type WeightLimit = WeightLimit;
    type AuctionDuration = AuctionDuration;
    type AuctionCost = AuctionCost;
    type MinimalBid = MinimalBid;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let _ = pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(ALICE, 10_000_000), (BOB, 42_000), (CHARLIE, 10_000)],
    }
    .assimilate_storage(&mut storage);

    storage.into()
}

fn next_block(mut count: u32) {
    let init_time = 1600438152000;
    while count > 0 {
        let block_number = System::block_number() + 1;
        System::set_block_number(block_number);
        Timestamp::set_timestamp(init_time + block_number * 12_000);
        RWS::on_initialize(block_number);
        count -= 1;
    }
}

#[test]
fn test_set_oracle() {
    let oracle = 1;
    new_test_ext().execute_with(|| {
        assert_err!(
            RWS::set_oracle(Origin::none(), oracle),
            DispatchError::BadOrigin
        );

        assert_err!(
            RWS::set_oracle(Origin::signed(oracle), oracle),
            DispatchError::BadOrigin
        );

        assert_ok!(RWS::set_oracle(Origin::root(), oracle));
        assert_eq!(RWS::oracle(), Some(oracle));
    })
}

#[test]
fn test_set_devices() {
    let phone = 10;
    let car = 11;
    new_test_ext().execute_with(|| {
        let devices = vec![phone, car];
        assert_ok!(RWS::set_devices(Origin::signed(ALICE), devices.clone()));
        assert_eq!(RWS::devices(ALICE), devices);
    })
}

#[test]
fn test_set_subscription() {
    let oracle = CHARLIE;
    new_test_ext().execute_with(|| {
        assert_ok!(RWS::set_oracle(Origin::root(), oracle));

        assert_err!(
            RWS::set_subscription(Origin::none(), ALICE, Default::default()),
            DispatchError::BadOrigin,
        );

        assert_err!(
            RWS::set_subscription(Origin::signed(ALICE), ALICE, Default::default()),
            Error::<Runtime>::OracleOnlyCall,
        );

        let lifetime = Subscription::Lifetime { tps: 10 };
        assert_ok!(RWS::set_subscription(
            Origin::signed(oracle),
            ALICE,
            lifetime.clone(),
        ));
        assert_eq!(RWS::ledger(ALICE).unwrap().kind, lifetime);
    })
}

#[test]
fn test_simple_subscription() {
    let oracle = CHARLIE;
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1600438152000);

        assert_ok!(RWS::set_oracle(Origin::root(), oracle));
        assert_ok!(RWS::set_subscription(
            Origin::signed(oracle),
            ALICE,
            Subscription::Lifetime { tps: 500_000 },
        ));
        assert_ok!(RWS::set_devices(Origin::signed(ALICE), vec![BOB]));

        assert_eq!(RWS::ledger(ALICE).unwrap().issue_time, 1600438152000);
        assert_eq!(RWS::ledger(ALICE).unwrap().free_weight, 0);

        let call = Call::from(datalog::Call::record {
            record: "true".into(),
        });
        assert_err!(
            RWS::call(Origin::signed(BOB), ALICE, Box::new(call.clone())),
            Error::<Runtime>::FreeWeightIsNotEnough,
        );

        Timestamp::set_timestamp(1600438153000);
        assert_err!(
            RWS::call(Origin::signed(BOB), ALICE, Box::new(call.clone())),
            Error::<Runtime>::FreeWeightIsNotEnough,
        );
        assert_eq!(RWS::ledger(ALICE).unwrap().free_weight, 35476000);

        Timestamp::set_timestamp(1600438155000);
        assert_err!(
            RWS::call(Origin::signed(BOB), ALICE, Box::new(call.clone())),
            Error::<Runtime>::FreeWeightIsNotEnough,
        );
        assert_eq!(RWS::ledger(ALICE).unwrap().free_weight, 106428000);

        Timestamp::set_timestamp(1600438165000);
        assert_ok!(RWS::call(Origin::signed(BOB), ALICE, Box::new(call)));
        assert_eq!(RWS::ledger(ALICE).unwrap().free_weight, 110188000);
    });
}

#[test]
fn test_subscription_auction() {
    new_test_ext().execute_with(|| {
        next_block(1);

        assert_eq!(RWS::auction_queue().len(), 0);
        assert_ok!(Staking::bond(Origin::signed(ALICE), ALICE_C, 10_500));
        assert_eq!(RWS::auction_queue().len(), 2);
        assert_eq!(RWS::unspend_bond_value(), 500);

        assert_eq!(
            RWS::auction(0).unwrap(),
            AuctionLedger {
                winner: None,
                best_price: 0,
                kind: Subscription::Daily { days: 30 },
            },
        );
        assert_eq!(
            RWS::auction(1).unwrap(),
            AuctionLedger {
                winner: None,
                best_price: 0,
                kind: Subscription::Daily { days: 30 },
            },
        );
        assert_eq!(RWS::auction(2), None);

        next_block(2);

        assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 2_500));
        assert_eq!(RWS::auction_queue().len(), 2);
        assert_eq!(RWS::unspend_bond_value(), 3_000);

        next_block(2);

        assert_ok!(Staking::bond(Origin::signed(CHARLIE), CHARLIE_C, 2_100));
        assert_eq!(RWS::auction_queue().len(), 3);
        assert_eq!(RWS::unspend_bond_value(), 100);

        next_block(3);

        assert_err!(
            RWS::bid(Origin::signed(ALICE), 8, 50),
            Error::<Runtime>::NotLiveAuction,
        );
        assert_err!(
            RWS::bid(Origin::signed(ALICE), 0, 50),
            Error::<Runtime>::TooSmallBid,
        );
        assert_ok!(RWS::bid(Origin::signed(ALICE), 0, 150));
        assert_eq!(RWS::auction(0).unwrap().winner, Some(ALICE));

        next_block(1);

        assert_eq!(RWS::auction_queue().len(), 3);
        assert_ok!(RWS::bid(Origin::signed(BOB), 0, 151));
        assert_eq!(RWS::auction(0).unwrap().winner, Some(BOB));

        next_block(1);

        assert_eq!(RWS::auction_queue().len(), 2);
        assert_eq!(
            RWS::ledger(BOB).unwrap().kind,
            Subscription::Daily { days: 30 }
        );
    })
}
