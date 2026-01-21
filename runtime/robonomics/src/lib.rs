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
//! The Robonomics runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

use alloc::{vec, vec::Vec};
use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{
    construct_runtime, derive_impl,
    dispatch::DispatchClass,
    genesis_builder_helper::{build_state, get_preset},
    parameter_types,
    traits::{
        fungible, tokens::imbalance::ResolveTo, AsEnsureOriginWithArg, ConstBool, ConstU128,
        ConstU32, ConstU64, Imbalance, OnUnbalanced, WithdrawReasons,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_REF_TIME_PER_SECOND},
        ConstantMultiplier, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
        WeightToFeePolynomial,
    },
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureSigned,
};
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment_rpc_runtime_api::{FeeDetails, RuntimeDispatchInfo};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256};
use sp_runtime::{
    impl_opaque_keys,
    traits::{BlakeTwo256, Block as BlockT, Bounded, ConvertInto},
    transaction_validity::{TransactionSource, TransactionValidity},
    BoundedVec, FixedPointNumber, Perbill, Permill, Perquintill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

mod common;
use common::{currency::*, time::*, *};

mod genesis_config_presets;
mod rws_transaction_extension;
pub use rws_transaction_extension::ChargeRwsTransaction;
//pub mod xcm_config;

/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included into the
/// relay chain.
const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;
/// How many parachain blocks are processed by the relay chain per parent. Limits the number of
/// blocks authored per slot.
const BLOCK_PROCESSING_VELOCITY: u32 = 1;
/// Relay chain slot duration, in milliseconds.
const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

/// Robonomics parachain runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("robonomics"),
    impl_name: alloc::borrow::Cow::Borrowed("robonomics-airalab"),
    authoring_version: 1,
    spec_version: 42,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 2,
    system_version: 1,
};

/// The version infromation used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
    }
}

pub struct BaseFilter;
impl frame_support::traits::Contains<RuntimeCall> for BaseFilter {
    fn contains(call: &RuntimeCall) -> bool {
        match call {
            // These modules are not allowed to be called by transactions:
            // Other modules should works:
            _ => true,
        }
    }
}

/// Fungible implementation of `OnUnbalanced` that deals with the fees.
pub struct DealWithFees;
impl OnUnbalanced<fungible::Credit<AccountId, Balances>> for DealWithFees {
    fn on_unbalanceds(
        mut fees_then_tips: impl Iterator<Item = fungible::Credit<AccountId, Balances>>,
    ) {
        use pallet_collator_selection::StakingPotAccountId;

        if let Some(mut fees) = fees_then_tips.next() {
            if let Some(tips) = fees_then_tips.next() {
                tips.merge_into(&mut fees);
            }
            ResolveTo::<StakingPotAccountId<Runtime>, Balances>::on_unbalanced(fees)
        }
    }
}

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 1 second of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND,
    cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub SS58Prefix: u8 = 32;
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Runtime {
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type AccountId = AccountId;
    type Nonce = Nonce;
    type Hash = Hash;
    type Block = Block;
    type BlockHashCount = BlockHashCount;
    type Version = Version;
    type AccountData = pallet_balances::AccountData<Balance>;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type MultiBlockMigrator = MultiBlockMigrations;
}

impl cumulus_pallet_weight_reclaim::Config for Runtime {
    type WeightInfo = ();
}

impl pallet_utility::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = ();
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<0>;
    type WeightInfo = ();
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = (CollatorSelection,);
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * COASE;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type AccountStore = System;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type ExistentialDeposit = ExistentialDeposit;
    type WeightInfo = ();
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<50>;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 1 * XRT;
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
        WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    // `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
    // highest number of schedules that encodes less than 2^10.
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1 * COASE;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
    pub MaximumMultiplier: Multiplier = Bounded::max_value();
    pub OperationalFeeMultiplier: u8 = 5;
}

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - [0, MAXIMUM_BLOCK_WEIGHT]
///   - [Balance::min, Balance::max]
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;
    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        // extrinsic base weight (smallest non-zero weight) is mapped to 1/10 COASE:
        let p = COASE;
        let q = 10 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
        smallvec::smallvec![WeightToFeeCoefficient {
            degree: 1,
            negative: false,
            coeff_frac: Perbill::from_rational(p % q, q),
            coeff_integer: p / q,
        }]
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, DealWithFees>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type WeightToFee = WeightToFee;
    type FeeMultiplierUpdate = TargetedFeeAdjustment<
        Self,
        TargetBlockFullness,
        AdjustmentVariable,
        MinimumMultiplier,
        MaximumMultiplier,
    >;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightInfo = ();
}

parameter_types! {
    pub const AssetDeposit: Balance = 10 * XRT; // 10 XRT deposit to create asset
    pub const AssetsStringLimit: u32 = 50;
    /// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
    // https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
    pub const MetadataDepositBase: Balance = deposit(1, 68);
    pub const MetadataDepositPerByte: Balance = deposit(0, 1);
}

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type AssetIdParameter = parity_scale_codec::Compact<u32>;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = ConstU128<XRT>;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ExistentialDeposit;
    type StringLimit = AssetsStringLimit;
    type Freezer = ();
    type Holder = ();
    type Extra = ();
    type WeightInfo = ();
    type RemoveItemsLimit = ConstU32<1000>;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type CallbackHandle = ();
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = ConstU32<100>;
    type WeightInfo = ();
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

/// Proxy type for delegating RWS subscription usage.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    parity_scale_codec::Encode,
    parity_scale_codec::Decode,
    parity_scale_codec::DecodeWithMemTracking,
    sp_runtime::RuntimeDebug,
    parity_scale_codec::MaxEncodedLen,
    scale_info::TypeInfo,
)]
pub enum ProxyType {
    /// Allow all calls
    Any,
    /// RWS subscription user - allows using a specific subscription via RWS::call
    /// The parameter is the subscription_id that the proxy can use
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
                    RuntimeCall::RWS(pallet_robonomics_rws_auction::Call::call {
                        subscription_id, ..
                    }) => subscription_id == allowed_subscription_id,
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

parameter_types! {
    // One storage item; key size 32, value size 8; .
    pub const ProxyDepositBase: Balance = deposit(1, 40);
    // Additional storage item size of 33 bytes.
    pub const ProxyDepositFactor: Balance = deposit(0, 33);
    pub const AnnouncementDepositBase: Balance = deposit(1, 48);
    pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

impl pallet_proxy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = ConstU32<32>;
    type MaxPending = ConstU32<32>;
    type CallHasher = BlakeTwo256;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
    type WeightInfo = ();
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

parameter_types! {
    pub ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
    pub ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
    pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
    Runtime,
    RELAY_CHAIN_SLOT_DURATION_MILLIS,
    BLOCK_PROCESSING_VELOCITY,
    UNINCLUDED_SEGMENT_CAPACITY,
>;

impl cumulus_pallet_parachain_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type DmpQueue = frame_support::traits::EnqueueWithOrigin<(), RelayOrigin>;
    type OutboundXcmpMessageSource = ();
    type XcmpMessageHandler = ();
    type ReservedDmpWeight = ReservedDmpWeight;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber =
        cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
    type ConsensusHook = ConsensusHook;
    type RelayParentOffset = ConstU32<0>;
    type WeightInfo = ();
}

impl parachain_info::Config for Runtime {}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = CollatorSelection;
    // Essentially just Aura, but let's be pedantic.
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type DisablingStrategy = ();
    type WeightInfo = ();
    type Currency = Balances;
    type KeyDeposit = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<100>;
    type AllowMultipleBlocksPerSlot = ConstBool<true>;
    type SlotDuration = ConstU64<MILLISECS_PER_BLOCK>;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const SessionLength: BlockNumber = 1 * HOURS;
}

pub type CollatorSelectionUpdateOrigin = EnsureRoot<AccountId>;

impl pallet_collator_selection::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UpdateOrigin = CollatorSelectionUpdateOrigin;
    type PotId = PotId;
    type MaxCandidates = ConstU32<50>;
    type MinEligibleCollators = ConstU32<4>;
    type MaxInvulnerables = ConstU32<20>;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = ();
}

parameter_types! {
    pub const WindowSize: u64 = 128;
    pub const MaximumMessageSize: u32 = 512;
}

impl pallet_robonomics_datalog::Config for Runtime {
    type Time = Timestamp;
    type Record = BoundedVec<u8, MaximumMessageSize>;
    type RuntimeEvent = RuntimeEvent;
    type WindowSize = WindowSize;
    type WeightInfo = pallet_robonomics_datalog::weights::SubstrateWeight<Runtime>;
}

impl pallet_robonomics_launch::Config for Runtime {
    type Parameter = H256;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_robonomics_launch::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ReferenceCallWeight: u64 = 70_952_000;  // let it be transfer call weight
    pub const AuctionDuration: u64 = 60_000;  // 60,000 milliseconds (i.e., 60 seconds)
    pub const MinimalBid: Balance = 1 * XRT;
    pub const AssetToTpsRatio: Permill = Permill::from_parts(1);
    pub const RwsId: PalletId = PalletId(*b"RwsStake");
}

impl pallet_robonomics_rws_auction::Config for Runtime {
    type Assets = Assets;
    type LifetimeAssetId = ConstU32<42>;
    type AssetToTpsRatio = AssetToTpsRatio;
    type PalletId = RwsId;
    type Call = RuntimeCall;
    type Time = Timestamp;
    type Moment = u64;
    type AuctionCurrency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type ReferenceCallWeight = ReferenceCallWeight;
    type AuctionDuration = AuctionDuration;
    type MinimalBid = MinimalBid;
    type StartAuctionOrigin = EnsureRoot<AccountId>;
    type WeightInfo = pallet_robonomics_rws_auction::weights::SubstrateWeight<Runtime>;
}

impl pallet_robonomics_digital_twin::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_robonomics_digital_twin::weights::SubstrateWeight<Runtime>;
}

impl pallet_robonomics_liability::Config for Runtime {
    type Agreement = pallet_robonomics_liability::SignedAgreement<
        pallet_robonomics_liability::technics::IPFS,
        pallet_robonomics_liability::economics::SimpleMarket<Self::AccountId, Balances>,
        Self::AccountId,
        sp_runtime::MultiSignature,
    >;
    type Report = pallet_robonomics_liability::SignedReport<
        Self::Nonce,
        Self::AccountId,
        sp_runtime::MultiSignature,
        pallet_robonomics_liability::technics::IPFS,
    >;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_robonomics_liability::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub MbmServiceWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
}

impl pallet_migrations::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Migrations = ();
    // Benchmarks need mocked migrations to guarantee that they succeed.
    #[cfg(feature = "runtime-benchmarks")]
    type Migrations = pallet_migrations::mock_helpers::MockedMigrations;
    type CursorMaxLen = ConstU32<65_536>;
    type IdentifierMaxLen = ConstU32<256>;
    type MigrationStatusHandler = ();
    type FailedMigrationHandler = frame_support::migrations::FreezeChainOnFailedMigration;
    type MaxServiceWeight = MbmServiceWeight;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

construct_runtime! {
    pub enum Runtime {
        // Basic stuff.
        System: frame_system = 10,
        Utility: pallet_utility = 11,
        Timestamp: pallet_timestamp = 12,
        Multisig: pallet_multisig = 15,
        Proxy: pallet_proxy = 14,
        MultiBlockMigrations: pallet_migrations = 16,
        StateTrieMigration: pallet_state_trie_migration = 17,

        // Parachain systems.
        ParachainSystem: cumulus_pallet_parachain_system = 21,
        ParachainInfo: parachain_info = 22,
        WeightReclaim: cumulus_pallet_weight_reclaim = 23,

        // Native currency and accounts.
        Balances: pallet_balances = 31,
        TransactionPayment: pallet_transaction_payment = 32,
        Vesting: pallet_vesting = 33,
        Assets: pallet_assets = 34,

        // Governance staff.
        //Treasury: pallet_treasury = 40,
        //Scheduler: pallet_scheduler = 41,
        //TechnicalCommittee: pallet_collective::<Instance2> = 42,
        //TechnicalMembership: pallet_membership::<Instance1> = 43,
        //Democracy: pallet_democracy = 44,
        //Preimage: pallet_preimage = 45,

        // Robonomics Network pallets.
        Datalog: pallet_robonomics_datalog = 51,
        Launch: pallet_robonomics_launch = 52,
        DigitalTwin: pallet_robonomics_digital_twin = 54,
        RWS: pallet_robonomics_rws_auction = 55,
        Liability: pallet_robonomics_liability = 56,

        // XCM support.
        //XcmpQueue: cumulus_pallet_xcmp_queue = 70,
        //PolkadotXcm: pallet_xcm = 71,
        //CumulusXcm: cumulus_pallet_xcm = 72,
        //DmpQueue: cumulus_pallet_dmp_queue = 73,
        //XcmInfo: pallet_xcm_info = 74,

        // Elastic scaling consensus
        Authorship: pallet_authorship = 80,
        CollatorSelection: pallet_collator_selection = 81,
        Session: pallet_session = 82,
        Aura: pallet_aura = 83,
        AuraExt: cumulus_pallet_aura_ext = 84,


        // TODO: remove when democracy enabled
        Sudo: pallet_sudo = 99,
    }
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The extension to the basic transaction logic.
pub type TxExtension = cumulus_pallet_weight_reclaim::StorageWeightReclaim<
    Runtime,
    (
        frame_system::CheckNonZeroSender<Runtime>,
        frame_system::CheckSpecVersion<Runtime>,
        frame_system::CheckTxVersion<Runtime>,
        frame_system::CheckGenesis<Runtime>,
        frame_system::CheckEra<Runtime>,
        frame_system::CheckNonce<Runtime>,
        frame_system::CheckWeight<Runtime>,
        pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
        frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
    ),
>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

/// Migrations to apply on runtime upgrade.
pub type Migrations = (
    pallet_multisig::migrations::v1::MigrateToV1<Runtime>,
    //cumulus_pallet_xcmp_queue::migration::v4::MigrationToV4<Runtime>,
    //cumulus_pallet_xcmp_queue::migration::v5::MigrateV4ToV5<Runtime>,
    InitMigrationStorage,
    // permanent
    //pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
);

pub struct InitMigrationStorage;
impl frame_support::traits::OnRuntimeUpgrade for InitMigrationStorage {
    fn on_runtime_upgrade() -> Weight {
        use sp_core::crypto::Ss58Codec;
        use sp_keyring::Sr25519Keyring;

        // setup sudo
        if let Ok(sudo_key) =
            AccountId::from_ss58check("5Cakru1BpXPiezeD2LRZh3pJamHcbX9yZ13KLBxuqdTpgnYF")
        {
            let _ = Sudo::set_key(RuntimeOrigin::root(), sudo_key.into());
        }

        // setup collators
        use pallet_collator_selection::{CandidacyBond, DesiredCandidates, Invulnerables};
        let desired_candidates = 25;
        let candidacy_bond = 32 * XRT;
        let invulnerables: Vec<AccountId> = vec![
            Sr25519Keyring::Alice.to_account_id(),
            Sr25519Keyring::Bob.to_account_id(),
        ];

        let bounded_invulnerables: BoundedVec<
            _,
            <Runtime as pallet_collator_selection::Config>::MaxInvulnerables,
        > = invulnerables
            .clone()
            .try_into()
            .expect("genesis invulnerables are more than T::MaxInvulnerables");

        DesiredCandidates::<Runtime>::put(desired_candidates);
        CandidacyBond::<Runtime>::put(candidacy_bond);
        Invulnerables::<Runtime>::put(bounded_invulnerables);

        // Genesis init pallet_session
        use pallet_session::{NextKeys, QueuedKeys, SessionHandler, SessionManager, Validators};

        // insert genesis keys
        NextKeys::<Runtime>::insert(
            Sr25519Keyring::Alice.to_account_id(),
            SessionKeys {
                aura: Sr25519Keyring::Alice.public().into(),
            },
        );
        NextKeys::<Runtime>::insert(
            Sr25519Keyring::Bob.to_account_id(),
            SessionKeys {
                aura: Sr25519Keyring::Bob.public().into(),
            },
        );

        // init validators
        let initial_validators_0 =
            <Runtime as pallet_session::Config>::SessionManager::new_session_genesis(0)
                .unwrap_or_else(|| {
                    frame_support::print(
                        "No initial validator provided by `SessionManager`, use \
                    session config keys to generate initial validator set.",
                    );
                    invulnerables
                });

        let initial_validators_1 =
            <Runtime as pallet_session::Config>::SessionManager::new_session_genesis(1)
                .unwrap_or_else(|| initial_validators_0.clone());

        let queued_keys: Vec<_> = initial_validators_1
            .into_iter()
            .filter_map(|v| pallet_session::Pallet::<Runtime>::load_keys(&v).map(|k| (v, k)))
            .collect();

        <Runtime as pallet_session::Config>::SessionHandler::on_genesis_session::<
            <Runtime as pallet_session::Config>::Keys,
        >(&queued_keys);

        Validators::<Runtime>::put(initial_validators_0);
        QueuedKeys::<Runtime>::put(queued_keys);

        <Runtime as pallet_session::Config>::SessionManager::start_session(0);

        Default::default()
    }
}

#[cfg(feature = "runtime-benchmarks")]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
frame_benchmarking::define_benchmarks!(
    [frame_system, SystemBench::<Runtime>]
    // Robonomics pallets
    [pallet_robonomics_datalog, Datalog]
    [pallet_robonomics_digital_twin, DigitalTwin]
    [pallet_robonomics_launch, Launch]
    [pallet_robonomics_liability, Liability]
    [pallet_robonomics_rws_auction, RWS]
);

// Implement our runtime API endpoints. This is just a bunch of proxying.
impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
        fn can_build_upon(
            included_hash: <Block as BlockT>::Hash,
            slot: cumulus_primitives_aura::Slot,
        ) -> bool {
            ConsensusHook::can_build_upon(included_hash, slot)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(MILLISECS_PER_BLOCK)
        }

        fn authorities() -> Vec<AuraId> {
            pallet_aura::Authorities::<Runtime>::get().into_inner()
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> sp_runtime::ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info(header)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, genesis_config_presets::get_preset)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            genesis_config_presets::preset_names()
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect,
        ) -> Weight {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here.
            Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::BenchmarkList;
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();

            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
            use frame_benchmarking::BenchmarkBatch;
            use frame_support::traits::TrackedStorageKey;
            use frame_system_benchmarking::Pallet as SystemBench;
            use hex_literal::hex;

            #[allow(non_local_definitions)]
            impl frame_system_benchmarking::Config for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![
                // Block Number
                hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // Total Issuance
                hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // Execution Phase
                hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // Event Count
                hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // System Events
                hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            Ok(batches)
        }
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor<Runtime, Executive>,
}

parameter_types! {
    // The deposit configuration for the singed migration. Specially if you want to allow any signed account to do the migration (see `SignedFilter`, these deposits should be high)
    pub const MigrationSignedDepositPerItem: Balance = COASE;
    pub const MigrationSignedDepositBase: Balance = 2_000 * COASE;
    pub const MigrationMaxKeyLen: u32 = 512;
}

impl pallet_state_trie_migration::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type SignedDepositPerItem = MigrationSignedDepositPerItem;
    type SignedDepositBase = MigrationSignedDepositBase;
    // An origin that can control the whole pallet: should be Root, or a part of your council.
    type ControlOrigin = frame_system::EnsureSignedBy<RootMigController, AccountId>;
    // specific account for the migration, can trigger the signed migrations.
    type SignedFilter = frame_system::EnsureSignedBy<MigController, AccountId>;

    // Replace this with weight based on your runtime.
    type WeightInfo = pallet_state_trie_migration::weights::SubstrateWeight<Runtime>;

    type MaxKeyLen = MigrationMaxKeyLen;
}

frame_support::ord_parameter_types! {
    pub const MigController: AccountId = AccountId::from(hex_literal::hex!("16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"));
    pub const RootMigController: AccountId = AccountId::from(hex_literal::hex!("16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"));
}
