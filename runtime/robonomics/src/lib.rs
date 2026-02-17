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

// Make development WASM binary available in dedicated module.
#[cfg(feature = "std")]
pub mod dev {
    include!(concat!(env!("OUT_DIR"), "/development_wasm_binary.rs"));
}

extern crate alloc;

use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    genesis_builder_helper::{build_state, get_preset},
    parameter_types,
    traits::{
        fungible, tokens::imbalance::ResolveTo, ConstBool, ConstU32, ConstU64, Imbalance,
        OnUnbalanced, WithdrawReasons,
    },
    weights::{ConstantMultiplier, Weight},
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot,
};
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment_rpc_runtime_api::{FeeDetails, RuntimeDispatchInfo};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256};
use sp_runtime::{
    impl_opaque_keys,
    traits::{BlakeTwo256, Block as BlockT, Bounded, ConvertInto},
    transaction_validity::{TransactionSource, TransactionValidity},
    BoundedVec, FixedPointNumber, Perbill, Perquintill,
};
use sp_std::prelude::*;

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub mod common;
pub mod xcm_config;
pub use common::{consensus::*, currency::*, fee::*, time::*, *};

mod genesis_config_presets;
mod weights;
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

/// Robonomics parachain runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("robonomics"),
    impl_name: alloc::borrow::Cow::Borrowed("robonomics-airalab"),
    authoring_version: 1,
    spec_version: 41,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 3,
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
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The nonce type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The block type.
    type Block = Block;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Runtime version.
    type Version = Version;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
    /// Weight information for the extensions of this pallet.
    type ExtensionsWeightInfo = weights::frame_system_extensions::WeightInfo<Runtime>;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    /// The action to take on a Runtime Upgrade
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type SingleBlockMigrations = SingleBlockMigrations;
}

impl cumulus_pallet_weight_reclaim::Config for Runtime {
    type WeightInfo = weights::cumulus_pallet_weight_reclaim::WeightInfo<Runtime>;
}

impl pallet_utility::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<0>;
    type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
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
    type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
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
    type WeightInfo = weights::pallet_vesting::WeightInfo<Runtime>;
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
    type WeightInfo = weights::pallet_transaction_payment::WeightInfo<Runtime>;
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
    type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
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
    type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
    type OutboundXcmpMessageSource = cumulus_pallet_xcmp_queue::Pallet<Runtime>;
    type XcmpMessageHandler = cumulus_pallet_xcmp_queue::Pallet<Runtime>;
    type ReservedDmpWeight = ReservedDmpWeight;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber =
        cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
    type ConsensusHook = ConsensusHook;
    type RelayParentOffset = ConstU32<0>;
    type WeightInfo = weights::cumulus_pallet_parachain_system::WeightInfo<Runtime>;
}

parameter_types! {
    pub MessageQueueServiceWeight: Weight =
        Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

pub struct NarrowOriginToSibling<Inner>(sp_std::marker::PhantomData<Inner>);
impl<Inner: frame_support::traits::QueuePausedQuery<cumulus_primitives_core::ParaId>>
    frame_support::traits::QueuePausedQuery<AggregateMessageOrigin>
    for NarrowOriginToSibling<Inner>
{
    fn is_paused(origin: &AggregateMessageOrigin) -> bool {
        match origin {
            AggregateMessageOrigin::Sibling(id) => Inner::is_paused(id),
            _ => false,
        }
    }
}

impl<Inner: pallet_message_queue::OnQueueChanged<cumulus_primitives_core::ParaId>>
    pallet_message_queue::OnQueueChanged<AggregateMessageOrigin> for NarrowOriginToSibling<Inner>
{
    fn on_queue_changed(origin: AggregateMessageOrigin, fp: frame_support::traits::QueueFootprint) {
        if let AggregateMessageOrigin::Sibling(id) = origin {
            Inner::on_queue_changed(id, fp)
        }
    }
}

impl pallet_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    #[cfg(feature = "runtime-benchmarks")]
    type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
        cumulus_primitives_core::AggregateMessageOrigin,
    >;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MessageProcessor = xcm_builder::ProcessXcmMessage<
        AggregateMessageOrigin,
        xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
        RuntimeCall,
    >;
    type Size = u32;
    // The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
    type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
    type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
    type HeapSize = sp_core::ConstU32<{ 103 * 1024 }>;
    type MaxStale = sp_core::ConstU32<8>;
    type ServiceWeight = MessageQueueServiceWeight;
    type IdleMaxServiceWeight = MessageQueueServiceWeight;
    type WeightInfo = weights::pallet_message_queue::WeightInfo<Runtime>;
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
    type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
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
    type WeightInfo = weights::pallet_collator_selection::WeightInfo<Runtime>;
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
    type WeightInfo = weights::pallet_robonomics_datalog::WeightInfo<Runtime>;
}

impl pallet_robonomics_launch::Config for Runtime {
    type Parameter = H256;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_robonomics_launch::WeightInfo<Runtime>;
}

parameter_types! {
    pub const ReferenceCallWeight: u64 = 70_952_000;  // let it be transfer call weight
    pub const AuctionDuration: BlockNumber = 10;
    pub const AuctionCost: Balance = 25000 * XRT;  // start subscription auction when amount locked
    pub const MinimalBid: Balance = 1 * XRT;
}

impl pallet_robonomics_rws::Config for Runtime {
    type Call = RuntimeCall;
    type Time = Timestamp;
    type Moment = u64;
    type AuctionIndex = u32;
    type AuctionCurrency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type ReferenceCallWeight = ReferenceCallWeight;
    type AuctionDuration = AuctionDuration;
    type AuctionCost = AuctionCost;
    type MinimalBid = MinimalBid;
    type MaxDevicesAmount = ConstU32<32>;
    type MaxAuctionIndexesAmount = ConstU32<4096>;
    type WeightInfo = weights::pallet_robonomics_rws::WeightInfo<Runtime>;
}

parameter_types! {
    pub ClaimMessagePrefix: &'static [u8] = b"Claim ERC20 XRT to account:";
    pub ClaimPalletId: PalletId = PalletId(*b"ClaimXrt");
}

impl pallet_robonomics_claim::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletId = ClaimPalletId;
    type Prefix = ClaimMessagePrefix;
    type WeightInfo = weights::pallet_robonomics_claim::WeightInfo<Runtime>;
}

#[cfg(any(feature = "dev-runtime", feature = "runtime-benchmarks"))]
parameter_types! {
    pub const MaxTreeDepth: u32 = 32;
    pub const MaxChildrenPerNode: u32 = 100;
    pub const MaxRootNodes: u32 = 100;
}

#[cfg(any(feature = "dev-runtime", feature = "runtime-benchmarks"))]
impl pallet_robonomics_cps::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxTreeDepth = MaxTreeDepth;
    type MaxChildrenPerNode = MaxChildrenPerNode;
    type MaxRootNodes = MaxRootNodes;
    type EncryptedData = pallet_robonomics_cps::DefaultEncryptedData;
    type OnPayloadSet = ();
    type WeightInfo = weights::pallet_robonomics_cps::WeightInfo<Runtime>;
}

impl pallet_robonomics_digital_twin::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_robonomics_digital_twin::WeightInfo<Runtime>;
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
    type WeightInfo = weights::pallet_robonomics_liability::WeightInfo<Runtime>;
}

impl pallet_sudo::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

#[frame_support::runtime(legacy_ordering)]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask,
        RuntimeViewFunction
    )]
    pub struct Runtime;

    //
    // Basic pallets
    //

    #[runtime::pallet_index(10)]
    pub type System = frame_system;

    #[runtime::pallet_index(11)]
    pub type Utility = pallet_utility;

    #[runtime::pallet_index(12)]
    pub type Timestamp = pallet_timestamp;

    #[runtime::pallet_index(15)]
    pub type Multisig = pallet_multisig;

    //
    // Parachain core pallets
    //

    #[runtime::pallet_index(21)]
    pub type ParachainSystem = cumulus_pallet_parachain_system;

    #[runtime::pallet_index(22)]
    pub type ParachainInfo = parachain_info;

    #[runtime::pallet_index(23)]
    pub type WeightReclaim = cumulus_pallet_weight_reclaim;

    //
    // Finance pallets
    //

    #[runtime::pallet_index(31)]
    pub type Balances = pallet_balances;

    #[runtime::pallet_index(32)]
    pub type TransactionPayment = pallet_transaction_payment;

    #[runtime::pallet_index(33)]
    pub type Vesting = pallet_vesting;

    #[runtime::pallet_index(35)]
    pub type ClaimXRT = pallet_robonomics_claim;

    //
    // Robonomics Network pallets.
    //

    #[runtime::pallet_index(51)]
    pub type Datalog = pallet_robonomics_datalog;

    #[runtime::pallet_index(52)]
    pub type Launch = pallet_robonomics_launch;

    #[runtime::pallet_index(54)]
    pub type DigitalTwin = pallet_robonomics_digital_twin;

    #[runtime::pallet_index(55)]
    pub type RWS = pallet_robonomics_rws;

    #[runtime::pallet_index(56)]
    pub type Liability = pallet_robonomics_liability;

    #[cfg(any(feature = "dev-runtime", feature = "runtime-benchmarks"))]
    #[runtime::pallet_index(57)]
    pub type CPS = pallet_robonomics_cps;

    //
    // XCM support pallets.
    //

    #[runtime::pallet_index(70)]
    pub type XcmpQueue = cumulus_pallet_xcmp_queue;

    #[runtime::pallet_index(71)]
    pub type PolkadotXcm = pallet_xcm;

    #[runtime::pallet_index(72)]
    pub type CumulusXcm = cumulus_pallet_xcm;

    #[runtime::pallet_index(74)]
    pub type XcmInfo = pallet_xcm_info;

    #[runtime::pallet_index(75)]
    pub type MessageQueue = pallet_message_queue;

    //
    // Elastic scaling consensus pallets.
    //

    #[runtime::pallet_index(80)]
    pub type Authorship = pallet_authorship;

    #[runtime::pallet_index(81)]
    pub type CollatorSelection = pallet_collator_selection;

    #[runtime::pallet_index(82)]
    pub type Session = pallet_session;

    #[runtime::pallet_index(83)]
    pub type Aura = pallet_aura;

    #[runtime::pallet_index(84)]
    pub type AuraExt = cumulus_pallet_aura_ext;

    // TODO: remove when democracy enabled
    #[runtime::pallet_index(99)]
    pub type Sudo = pallet_sudo;
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
>;

parameter_types! {
    pub const AssetsName: &'static str = "Assets";
    pub const StateTrieMigrationName: &'static str = "StateTrieMigration";
    pub const MultiBlockMigrationsName: &'static str = "MultiBlockMigrations";
}

/// Migrations to apply on runtime upgrade.
type SingleBlockMigrations = (
    // Cleanup old pallets
    frame_support::migrations::RemovePallet<AssetsName, RocksDbWeight>,
    frame_support::migrations::RemovePallet<StateTrieMigrationName, RocksDbWeight>,
    frame_support::migrations::RemovePallet<MultiBlockMigrationsName, RocksDbWeight>,
    // XCM
    pallet_xcm::migration::v1::MigrateToV1<Runtime>,
    pallet_xcm_info::migration::v1::MigrateToV1<Runtime>,
    cumulus_pallet_xcmp_queue::migration::v4::MigrationToV4<Runtime>,
    cumulus_pallet_xcmp_queue::migration::v5::MigrateV4ToV5<Runtime>,
    // Permanent
    pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
);

#[cfg(feature = "runtime-benchmarks")]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
frame_benchmarking::define_benchmarks!(
    // System pallets
    [frame_system, SystemBench::<Runtime>]
    [frame_system_extensions, SystemExtensionsBench::<Runtime>]
    [pallet_balances, Balances]
    [pallet_timestamp, Timestamp]
    [pallet_utility, Utility]
    [pallet_multisig, Multisig]
    [pallet_vesting, Vesting]
    [pallet_transaction_payment, TransactionPayment]
    [cumulus_pallet_weight_reclaim, WeightReclaim]
    [cumulus_pallet_parachain_system, ParachainSystem]
    // Consensus pallets
    [pallet_collator_selection, CollatorSelection]
    [pallet_session, SessionBench::<Runtime>]
    // Robonomics pallets
    [pallet_robonomics_datalog, Datalog]
    [pallet_robonomics_digital_twin, DigitalTwin]
    [pallet_robonomics_launch, Launch]
    [pallet_robonomics_liability, Liability]
    [pallet_robonomics_rws, RWS]
    [pallet_robonomics_cps, CPS]
    [pallet_robonomics_claim, ClaimXRT]
    // XCM pallets
    [cumulus_pallet_xcmp_queue, XcmpQueue]
    [pallet_message_queue, MessageQueue]
    [pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
    [pallet_xcm_benchmarks::fungible, XcmBalances]
    [pallet_xcm_benchmarks::generic, XcmGeneric]
    [pallet_xcm_info, XcmInfo]
);

// Implement our runtime API endpoints. This is just a bunch of proxying.
impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: <Block as BlockT>::LazyBlock) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
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

        fn check_inherents(block: <Block as BlockT>::LazyBlock, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
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

    // XCM Runtime APIs - providing standard XCM functionality
    impl xcm_runtime_apis::fees::XcmPaymentApi<Block> for Runtime {
        fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<xcm::VersionedAssetId>, xcm_runtime_apis::fees::Error> {
            use xcm::latest::prelude::*;
            // Currently only the native token (LocalLocation) is accepted for XCM fees
            // Additional assets can be added here when multi-asset fee payment is supported
            let acceptable_assets = vec![AssetId(xcm_config::LocalLocation::get())];
            PolkadotXcm::query_acceptable_payment_assets(xcm_version, acceptable_assets)
        }

        fn query_weight_to_asset_fee(weight: Weight, asset: xcm::VersionedAssetId) -> Result<u128, xcm_runtime_apis::fees::Error> {
            type Trader = <xcm_config::XcmConfig as xcm_executor::Config>::Trader;
            PolkadotXcm::query_weight_to_asset_fee::<Trader>(weight, asset)
        }

        fn query_xcm_weight(message: xcm::VersionedXcm<()>) -> Result<Weight, xcm_runtime_apis::fees::Error> {
            PolkadotXcm::query_xcm_weight(message)
        }

        fn query_delivery_fees(destination: xcm::VersionedLocation, message: xcm::VersionedXcm<()>, asset_id: xcm::VersionedAssetId) -> Result<xcm::VersionedAssets, xcm_runtime_apis::fees::Error> {
            type AssetExchanger = <xcm_config::XcmConfig as xcm_executor::Config>::AssetExchanger;
            PolkadotXcm::query_delivery_fees::<AssetExchanger>(destination, message, asset_id)
        }
    }

    // Note: DryRunApi uses OriginCaller instead of RuntimeOrigin because OriginCaller
    // implements the required Encode and TypeInfo traits for the XCM runtime API
    impl xcm_runtime_apis::dry_run::DryRunApi<Block, RuntimeCall, RuntimeEvent, OriginCaller> for Runtime {
        fn dry_run_call(origin: OriginCaller, call: RuntimeCall, result_xcms_version: xcm::Version) -> Result<xcm_runtime_apis::dry_run::CallDryRunEffects<RuntimeEvent>, xcm_runtime_apis::dry_run::Error> {
            PolkadotXcm::dry_run_call::<Runtime, xcm_config::XcmRouter, _, _>(origin, call, result_xcms_version)
        }

        fn dry_run_xcm(origin_location: xcm::VersionedLocation, xcm: xcm::VersionedXcm<RuntimeCall>) -> Result<xcm_runtime_apis::dry_run::XcmDryRunEffects<RuntimeEvent>, xcm_runtime_apis::dry_run::Error> {
            PolkadotXcm::dry_run_xcm::<xcm_config::XcmRouter>(origin_location, xcm)
        }
    }

    impl xcm_runtime_apis::conversions::LocationToAccountApi<Block, AccountId> for Runtime {
        fn convert_location(location: xcm::VersionedLocation) -> Result<AccountId, xcm_runtime_apis::conversions::Error> {
            xcm_runtime_apis::conversions::LocationToAccountHelper::<
                AccountId,
                xcm_config::LocationToAccountId,
            >::convert_location(location)
        }
    }

    impl xcm_runtime_apis::trusted_query::TrustedQueryApi<Block> for Runtime {
        fn is_trusted_reserve(asset: xcm::VersionedAsset, location: xcm::VersionedLocation) -> xcm_runtime_apis::trusted_query::XcmTrustedQueryResult {
            PolkadotXcm::is_trusted_reserve(asset, location)
        }

        fn is_trusted_teleporter(asset: xcm::VersionedAsset, location: xcm::VersionedLocation) -> xcm_runtime_apis::trusted_query::XcmTrustedQueryResult {
            PolkadotXcm::is_trusted_teleporter(asset, location)
        }
    }

    impl xcm_runtime_apis::authorized_aliases::AuthorizedAliasersApi<Block> for Runtime {
        fn authorized_aliasers(target: xcm::VersionedLocation) -> Result<Vec<xcm_runtime_apis::authorized_aliases::OriginAliaser>, xcm_runtime_apis::authorized_aliases::Error> {
            PolkadotXcm::authorized_aliasers(target)
        }

        fn is_authorized_alias(origin: xcm::VersionedLocation, target: xcm::VersionedLocation) -> Result<bool, xcm_runtime_apis::authorized_aliases::Error> {
            PolkadotXcm::is_authorized_alias(origin, target)
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
            use frame_system_benchmarking::extensions::Pallet as SystemExtensionsBench;
            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
            use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;

            type XcmBalances = pallet_xcm_benchmarks::fungible::Pallet::<Runtime>;
            type XcmGeneric = pallet_xcm_benchmarks::generic::Pallet::<Runtime>;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();

            (list, storage_info)
        }

        #[allow(non_local_definitions)]
        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
            use frame_benchmarking::{BenchmarkBatch, BenchmarkError};
            use frame_support::traits::TrackedStorageKey;
            use frame_system_benchmarking::Pallet as SystemBench;
            use frame_system_benchmarking::extensions::Pallet as SystemExtensionsBench;

            impl frame_system_benchmarking::Config for Runtime {
                fn setup_set_code_requirements(code: &alloc::vec::Vec<u8>) -> Result<(), BenchmarkError> {
                    ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
                    Ok(())
                }
                fn verify_set_code() {
                    System::assert_last_event(
                        cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into()
                    );
                }
            }

            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
            impl cumulus_pallet_session_benchmarking::Config for Runtime {}

            use xcm::latest::prelude::*;
            use xcm_config::{AssetHubParaId, AssetHubLocation, LocalLocation, PriceForSiblingParachainDelivery};

            use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;

            parameter_types! {
                pub ExistentialDepositAsset: Option<Asset> = Some((
                    LocalLocation::get(),
                    ExistentialDeposit::get()
                ).into());
            }

            impl pallet_xcm::benchmarking::Config for Runtime {
                type DeliveryHelper =
                    polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
                        xcm_config::XcmConfig,
                        ExistentialDepositAsset,
                        PriceForSiblingParachainDelivery,
                        AssetHubParaId,
                        ParachainSystem,
                    >;

                fn reachable_dest() -> Option<Location> {
                    Some(AssetHubLocation::get())
                }

                fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
                    // Relay/native token can be teleported between AH and Relay.
                    Some((
                        Asset {
                            fun: Fungible(ExistentialDeposit::get()),
                            id: AssetId(LocalLocation::get())
                        },
                        AssetHubLocation::get(),
                    ))
                }

                fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
                    None
                }

                fn set_up_complex_asset_transfer() -> Option<(Assets, u32, Location, alloc::boxed::Box<dyn FnOnce()>)> {
                    let native_location = Here.into();
                    let dest = AssetHubLocation::get();

                    pallet_xcm::benchmarking::helpers::native_teleport_as_asset_transfer::<Runtime>(
                        native_location,
                        dest,
                    )
                }

                fn get_asset() -> Asset {
                    Asset {
                        id: AssetId(LocalLocation::get()),
                        fun: Fungible(ExistentialDeposit::get()),
                    }
                }
            }

            impl pallet_xcm_benchmarks::Config for Runtime {
                type XcmConfig = xcm_config::XcmConfig;
                type DeliveryHelper = polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
                        xcm_config::XcmConfig,
                        ExistentialDepositAsset,
                        PriceForSiblingParachainDelivery,
                        AssetHubParaId,
                        ParachainSystem,
                    >;
                type AccountIdConverter = xcm_config::LocationToAccountId;
                fn valid_destination() -> Result<Location, BenchmarkError> {
                    Ok(AssetHubLocation::get())
                }
                fn worst_case_holding(_depositable_count: u32) -> Assets {
                    let assets: Vec<Asset> = vec![
                        Asset {
                            id: AssetId(LocalLocation::get()),
                            fun: Fungible(1_000_000 * XRT),
                        }
                    ];
                    assets.into()
                }
            }

            parameter_types! {
                pub TrustedTeleporter: Option<(Location, Asset)> = Some((
                    AssetHubLocation::get(),
                    Asset { fun: Fungible(XRT), id: AssetId(LocalLocation::get()) },
                ));
                pub const CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = None;
                pub const TrustedReserve: Option<(Location, Asset)> = None;
            }

            impl pallet_xcm_benchmarks::fungible::Config for Runtime {
                type TransactAsset = Balances;

                type CheckedAccount = CheckedAccount;
                type TrustedTeleporter = TrustedTeleporter;
                type TrustedReserve = TrustedReserve;

                fn get_asset() -> Asset {
                    Asset {
                        id: AssetId(LocalLocation::get()),
                        fun: Fungible(XRT),
                    }
                }
            }

            impl pallet_xcm_benchmarks::generic::Config for Runtime {
                type RuntimeCall = RuntimeCall;
                type TransactAsset = Balances;

                fn worst_case_response() -> (u64, Response) {
                    (0u64, Response::Version(Default::default()))
                }

                fn worst_case_asset_exchange() -> Result<(Assets, Assets), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }

                fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }

                fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
                    Ok((AssetHubLocation::get(), frame_system::Call::remark_with_event { remark: vec![] }.into()))
                }

                fn subscribe_origin() -> Result<Location, BenchmarkError> {
                    Ok(AssetHubLocation::get())
                }

                fn claimable_asset() -> Result<(Location, Location, Assets), BenchmarkError> {
                    let origin = AssetHubLocation::get();
                    let assets: Assets = (AssetId(LocalLocation::get()), 1_000 * XRT).into();
                    let ticket = Location { parents: 0, interior: Here };
                    Ok((origin, ticket, assets))
                }

                fn worst_case_for_trader() -> Result<(Asset, WeightLimit), BenchmarkError> {
                    Ok((Asset {
                        id: AssetId(LocalLocation::get()),
                        fun: Fungible(1_000_000 * XRT),
                    }, WeightLimit::Limited(Weight::from_parts(5000, 5000))))
                }

                fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }

                fn export_message_origin_and_destination(
                ) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }

                fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
                    Err(BenchmarkError::Skip)
                }
            }

            type XcmBalances = pallet_xcm_benchmarks::fungible::Pallet::<Runtime>;
            type XcmGeneric = pallet_xcm_benchmarks::generic::Pallet::<Runtime>;

            use frame_support::traits::WhitelistedStorageKeys;
            let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            Ok(batches)
        }
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
