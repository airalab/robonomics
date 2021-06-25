#![allow(clippy::from_over_into)]
use crate::{self as pallet_evercity_transfer, *};

use crate::*;
use frame_support::{
    assert_err, assert_ok, parameter_types,
    sp_runtime::{
        testing::Header,
        traits::{BlakeTwo256, IdentityLookup},
    },
};
use sp_core::H256;

pub type Balance = u64;
const UNIT: Balance = 1_000_000;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
        pub enum Test where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Module, Call, Config, Storage, Event<T>},
            //Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
            Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
            //Evercity: pallet_evercity::{Module, Call, Storage, Event<T>},
            EvercityTransfer: pallet_evercity_transfer::{Module, Call, Storage, Event<T>},
        }
);

parameter_types! {
    pub const BlockHashCount: u64 = 2400;
}

impl frame_system::Config for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
}

parameter_types! {
    pub const MaximumTransferValue: Balance = 10_000_000_000_000;
}

impl Config for Test {
    type Event = Event;
    type MaximumTransferValue = MaximumTransferValue;
    type Currency = Balances;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 0;
    pub const MaxLocks: u32 = 5;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
}

fn new_test_ext() -> frame_support::sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        // Provide some initial balances
        balances: vec![(1, UNIT)],
    }
    .assimilate_storage(&mut storage)
    .unwrap();

    storage.into()
}

#[test]
fn test_spend_endowed_balance() {
    new_test_ext().execute_with(|| {
        assert_ok!(EvercityTransfer::transfer(Origin::signed(1), 2, 1000));
        assert_eq!(Balances::free_balance(2), 1000);
        assert_err!(
            Balances::transfer(Origin::signed(2), 3, 10),
            pallet_balances::Error::<Test, _>::LiquidityRestrictions
        );
        assert_ok!(EvercityTransfer::transfer(Origin::signed(1), 2, 1000));
        assert_eq!(Balances::free_balance(2), 2000);
        assert_ok!(Balances::transfer(Origin::signed(2), 3, 10));
    })
}
