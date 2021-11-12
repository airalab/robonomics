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
//! Robonomics Web Services pallet tests.

use crate::{self as rws, *};
use frame_support::{assert_err, assert_ok, parameter_types, weights::Weight};
use pallet_robonomics_datalog as datalog;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchError, Perbill};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;
type Balance = u128;

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
        RWS: rws::{Pallet, Call, Storage, Event<T>},
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
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type PalletInfo = PalletInfo;
    type OnSetCode = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
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
    pub const WeightLimit: Weight = 1_000_000_000_000;
    pub const TotalBandwidth: u64 = 100;
    pub const PointsLimit: u64 = 1_000_000_000_000_000;
}

impl Config for Runtime {
    type TotalBandwidth = TotalBandwidth;
    type WeightLimit = WeightLimit;
    type PointsLimit = PointsLimit;
    type AuctionCurrency = Balances;
    type Time = Timestamp;
    type Event = Event;
    type Call = Call;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let storage = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();
    storage.into()
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

        assert_ok!(RWS::set_oracle(Origin::root(), oracle),);
        assert_eq!(RWS::oracle(), Some(oracle));
    })
}

#[test]
fn test_set_bandwidth() {
    let oracle = 1;
    let alice = 2;
    new_test_ext().execute_with(|| {
        assert_ok!(RWS::set_oracle(Origin::root(), oracle));

        assert_err!(
            RWS::set_bandwidth(Origin::none(), alice, Default::default()),
            DispatchError::BadOrigin,
        );

        assert_ok!(RWS::set_bandwidth(
            Origin::signed(oracle),
            alice,
            Perbill::from_percent(1),
        ));
        assert_eq!(RWS::bandwidth(alice), Some(Perbill::from_percent(1)));
    })
}

#[test]
fn test_subscription() {
    let oracle = 1;
    let alice = 2;
    let bob = 3;

    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1600438152000);

        assert_ok!(RWS::set_oracle(Origin::root(), oracle));

        let call = Call::from(datalog::Call::record("true".into()));

        assert_eq!(RWS::quota(alice), None);
        assert_err!(
            RWS::call(Origin::signed(bob), alice, call.clone().into()),
            Error::<Runtime>::NoSubscription,
        );

        assert_ok!(RWS::set_subscription(Origin::signed(alice), vec![bob]));
        assert_err!(
            RWS::call(Origin::signed(bob), alice, call.clone().into()),
            Error::<Runtime>::NoQuota,
        );

        assert_ok!(RWS::set_bandwidth(
            Origin::signed(oracle),
            alice,
            Perbill::from_percent(1),
        ),);
        assert_eq!(RWS::quota(alice), None);
        assert_ok!(RWS::call(Origin::signed(bob), alice, call.clone().into()));
        assert_eq!(RWS::quota(alice), Some((1600438152000, 0)));
    })
}

#[test]
fn test_transaction() {
    let oracle = 1;
    let alice = 2;

    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(1600438152000);

        assert_ok!(RWS::set_oracle(Origin::root(), oracle));

        let call = Call::from(datalog::Call::record("true".into()));

        assert_eq!(RWS::quota(alice), None);
        assert_err!(
            RWS::call(Origin::signed(alice), alice, call.clone().into()),
            Error::<Runtime>::NoSubscription,
        );

        assert_ok!(RWS::set_subscription(Origin::signed(alice), vec![alice]));
        assert_err!(
            RWS::call(Origin::signed(alice), alice, call.clone().into()),
            Error::<Runtime>::NoQuota,
        );

        assert_ok!(RWS::set_bandwidth(
            Origin::signed(oracle),
            alice,
            Perbill::from_percent(1),
        ),);
        assert_eq!(RWS::quota(alice), None);
        assert_ok!(RWS::call(Origin::signed(alice), alice, call.clone().into()));
        assert_eq!(RWS::quota(alice), Some((1600438152000, 0)));

        Timestamp::set_timestamp(1600438156000);

        assert_ok!(RWS::call(Origin::signed(alice), alice, call.clone().into()));
        assert_eq!(RWS::quota(alice), Some((1600438156000, 3 * CALL_COST)));

        assert_ok!(RWS::call(Origin::signed(alice), alice, call.clone().into()));
        assert_eq!(RWS::quota(alice), Some((1600438156000, 2 * CALL_COST)));

        assert_ok!(RWS::call(Origin::signed(alice), alice, call.clone().into()));
        assert_eq!(RWS::quota(alice), Some((1600438156000, 1 * CALL_COST)));

        assert_ok!(RWS::call(Origin::signed(alice), alice, call.clone().into()));
        assert_eq!(RWS::quota(alice), Some((1600438156000, 0)));

        assert_err!(
            RWS::call(Origin::signed(alice), alice, call.into()),
            Error::<Runtime>::NoQuota,
        );
    })
}
