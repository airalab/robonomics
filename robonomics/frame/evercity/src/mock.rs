use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use crate::account::*;
use crate::bond::BondPeriodNumber;
use crate::{
    BondInnerStructOf, BondPeriod, BondStructOf, Config, EverUSDBalance, EvercityAccountStructT,
    DEFAULT_DAY_DURATION,
};

impl_outer_origin! {
    pub enum Origin for TestRuntime {}
}

// Configure a mock runtime to test the pallet.
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
pub const EVERUSD_MAX_MINT_AMOUNT: EverUSDBalance = 60_000_000_000_000_000; // =60 million dollar

#[derive(Clone, Eq, PartialEq)]
pub struct TestRuntime;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const ExistentialDeposit: u64 = 0;
}

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}
parameter_types! {
    pub const BurnRequestTtl: u32 = DEFAULT_DAY_DURATION as u32 * 7 * 1000;
    pub const MintRequestTtl: u32 = DEFAULT_DAY_DURATION as u32 * 7 * 1000;
    pub const MaxMintAmount: EverUSDBalance = EVERUSD_MAX_MINT_AMOUNT;
    pub const TimeStep: BondPeriod = DEFAULT_DAY_DURATION;
}

impl Config for TestRuntime {
    type Event = ();
    type BurnRequestTtl = BurnRequestTtl;
    type MintRequestTtl = MintRequestTtl;
    type MaxMintAmount = MaxMintAmount;
    type TimeStep = TimeStep;
    type WeightInfo = ();
    type OnAddAccount = ();
    type OnAddBond = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for TestRuntime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for TestRuntime {
    type Balance = u64;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
}
// (AccountId, role)
static ROLES: [(u64, u8); 8] = [
    (1_u64, MASTER_ROLE_MASK),
    (2_u64, CUSTODIAN_ROLE_MASK),
    (3_u64, ISSUER_ROLE_MASK),
    (4_u64, INVESTOR_ROLE_MASK),
    (5_u64, AUDITOR_ROLE_MASK),
    (6_u64, INVESTOR_ROLE_MASK),
    (7_u64, ISSUER_ROLE_MASK | INVESTOR_ROLE_MASK),
    (8_u64, MANAGER_ROLE_MASK),
];

pub type System = frame_system::Module<TestRuntime>;
// pub type Evercity = Module<TestRuntime>;
// pub type Balances = pallet_balances::Module<TestRuntime>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<TestRuntime>()
        .unwrap();
    pallet_balances::GenesisConfig::<TestRuntime> {
        // Provide some initial balances
        balances: ROLES.iter().map(|x| (x.0, 100000)).collect(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    super::GenesisConfig::<TestRuntime> {
        // Accounts for tests
        genesis_account_registry: ROLES
            .iter()
            .map(|(acc, role)| {
                (
                    *acc,
                    EvercityAccountStructT::<u64> {
                        roles: *role,
                        identity: 0,
                        create_time: 0,
                    },
                )
            })
            .collect(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}

type BondInnerStruct = BondInnerStructOf<TestRuntime>;
type BondStruct = BondStructOf<TestRuntime>;

pub fn get_test_bond() -> BondStruct {
    const PERIODS: usize = 12;
    BondStruct {
        inner: BondInnerStruct {
            docs_pack_root_hash_main: Default::default(),
            docs_pack_root_hash_legal: Default::default(),
            docs_pack_root_hash_finance: Default::default(),
            docs_pack_root_hash_tech: Default::default(),

            impact_data_type: Default::default(),
            impact_data_baseline: vec![20000_u64; PERIODS],
            impact_data_max_deviation_cap: 30000_u64,
            impact_data_max_deviation_floor: 14000_u64,
            interest_rate_penalty_for_missed_report: 400, // +0.4%

            interest_rate_base_value: 2000,   // 2.0%
            interest_rate_margin_cap: 4000,   // 4.0%
            interest_rate_margin_floor: 1000, // 1.0%
            interest_rate_start_period_value: 1900,
            start_period: 120 * DEFAULT_DAY_DURATION,
            payment_period: 30 * DEFAULT_DAY_DURATION, // every month (30 days)
            interest_pay_period: 7 * DEFAULT_DAY_DURATION, // up to 7 days after  new period started
            mincap_deadline: (20 * DEFAULT_DAY_DURATION * 1000) as u64,
            impact_data_send_period: 10 * DEFAULT_DAY_DURATION, // 10 days before next period
            bond_duration: PERIODS as BondPeriodNumber,         // PERIODS periods for 30 days
            bond_finishing_period: 14 * DEFAULT_DAY_DURATION,   // 14 days after mature date

            bond_units_mincap_amount: 1000,
            bond_units_maxcap_amount: 1800,
            bond_units_base_price: 4_000_000_000_000,
        },

        issuer: 0,
        manager: 0,
        auditor: 0,
        impact_reporter: 0,

        issued_amount: 0,
        booking_start_date: Default::default(),
        active_start_date: Default::default(),
        creation_date: Default::default(),
        state: Default::default(),

        bond_debit: 0,
        bond_credit: 0,
        coupon_yield: 0,
        nonce: 0,
    }
}
