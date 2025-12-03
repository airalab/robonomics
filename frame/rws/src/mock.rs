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
//! Mock runtime for testing RWS pallet.

use crate::{self as pallet_rws};
use frame_support::{derive_impl, parameter_types, traits::ConstU64};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{traits::{BlakeTwo256, IdentityLookup}, BuildStorage, RuntimeDebug};

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type AssetId = u32;
type AssetBalance = u128;
type Moment = u64;

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;
const LIFETIME_ASSET_ID: AssetId = 1;

/// Proxy type for testing
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    MaxEncodedLen,
    TypeInfo,
)]
pub enum ProxyType {
    Any,
    RwsUser(u32),
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}

impl frame_support::traits::InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::RwsUser(allowed_subscription_id) => {
                // Only allow RWS::call operations for the specific subscription
                match c {
                    RuntimeCall::RWS(pallet_rws::Call::call { subscription_id, .. }) => {
                        subscription_id == allowed_subscription_id
                    }
                    _ => false,
                }
            }
        }
    }
    
    fn is_superset(&self, o: &Self) -> bool {
        match (self, o) {
            (ProxyType::Any, _) => true,
            (_, ProxyType::Any) => false,
            (ProxyType::RwsUser(a), ProxyType::RwsUser(b)) => a == b,
        }
    }
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Assets: pallet_assets,
        Proxy: pallet_proxy,
        RWS: pallet_rws,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type AccountData = pallet_balances::AccountData<Balance>;
    type DbWeight = frame_support::weights::constants::RocksDbWeight;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

impl pallet_assets::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = AssetBalance;
    type AssetId = AssetId;
    type AssetIdParameter = AssetId;
    type Currency = Balances;
    type CreateOrigin = frame_support::traits::AsEnsureOriginWithArg<frame_system::EnsureSigned<u64>>;
    type ForceOrigin = frame_system::EnsureRoot<u64>;
    type AssetDeposit = ConstU128<0>;
    type AssetAccountDeposit = ConstU128<0>;
    type MetadataDepositBase = ConstU128<0>;
    type MetadataDepositPerByte = ConstU128<0>;
    type ApprovalDeposit = ConstU128<0>;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
    type RemoveItemsLimit = ConstU32<1000>;
    type CallbackHandle = ();
    type Holder = ();
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

impl pallet_timestamp::Config for Test {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}

parameter_types! {
    pub const ProxyDepositBase: Balance = 1;
    pub const ProxyDepositFactor: Balance = 1;
    pub const AnnouncementDepositBase: Balance = 1;
    pub const AnnouncementDepositFactor: Balance = 1;
}

impl pallet_proxy::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = frame_support::traits::ConstU32<32>;
    type MaxPending = frame_support::traits::ConstU32<32>;
    type CallHasher = BlakeTwo256;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
    type WeightInfo = ();
    type BlockNumberProvider = System;
}

parameter_types! {
    pub const ReferenceCallWeight: u64 = 70_952_000;
    pub const AuctionDuration: u64 = 100_000; // 100 seconds in milliseconds
    pub const MinimalBid: u128 = 100;
    pub const LifetimeAssetId: AssetId = LIFETIME_ASSET_ID;
    // Ratio: Permill::from_parts(1_000) = 1000 μTPS per 10 tokens = 100 μTPS per 1 token
    pub AssetToTpsRatio: Permill = Permill::from_parts(1_000);
    pub const RwsPalletId: frame_support::PalletId = frame_support::PalletId(*b"rws/lock");
}

impl pallet_rws::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Call = RuntimeCall;
    type Time = Timestamp;
    type Moment = u64;
    type AuctionCurrency = Balances;
    type Assets = Assets;
    type PalletId = RwsPalletId;
    type LifetimeAssetId = LifetimeAssetId;
    type AssetToTpsRatio = AssetToTpsRatio;
    type ReferenceCallWeight = ReferenceCallWeight;
    type AuctionDuration = AuctionDuration;
    type MinimalBid = MinimalBid;
    type StartAuctionOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ALICE, 10_000_000), (BOB, 5_000_000), (CHARLIE, 5_000_000)],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        Timestamp::set_timestamp(1000);
        
        // Create the lifetime asset
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            LIFETIME_ASSET_ID,
            ALICE, // admin
            true,  // is_sufficient
            1      // min_balance
        ));
        
        // Mint assets to test accounts
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            LIFETIME_ASSET_ID,
            ALICE,
            10_000_000
        ));
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            LIFETIME_ASSET_ID,
            BOB,
            5_000_000
        ));
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            LIFETIME_ASSET_ID,
            CHARLIE,
            5_000_000
        ));
    });
    ext
}
