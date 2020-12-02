///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life>
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
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(feature = "std")]
/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
pub fn wasm_binary_unwrap() -> &'static [u8] {
    WASM_BINARY.expect(
        "Development wasm binary is not available. This means the client is \
                        built with `BUILD_DUMMY_WASM_BINARY` flag and it is only usable for \
                        production chains. Please rebuild with the flag disabled.",
    )
}

pub mod constants;

use codec::Encode;
use frame_support::{
    construct_runtime, debug, parameter_types,
    traits::{LockIdentifier, Randomness, U128CurrencyToVote},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        IdentityFee, Weight,
    },
};
use node_primitives::{
    AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, Moment, Signature,
};
use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use sp_api::impl_runtime_apis;
use sp_core::{
    crypto::KeyTypeId,
    u32_trait::{_1, _2, _3, _5},
    OpaqueMetadata,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{self, BlakeTwo256, Block as BlockT, SaturatedConversion, Saturating, StaticLookup},
    transaction_validity::{TransactionSource, TransactionValidity},
    FixedPointNumber, ModuleId, Perbill, Percent, Permill, Perquintill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use constants::{currency::*, time::*};

/// Standalone runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("robonomics"),
    impl_name: create_runtime_str!("robonomics-airalab"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
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
    }
}

const AVERAGE_ON_INITIALIZE_WEIGHT: Perbill = Perbill::from_percent(10);
parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    /// We allow for 4 seconds of compute with a 12 second average block time.
    pub const MaximumBlockWeight: Weight = 4 * WEIGHT_PER_SECOND;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    /// Assume 10% of weight for average on_initialize calls
    pub MaximumExtrinsicWeight: Weight =
        AvailableBlockRatio::get().saturating_sub(AVERAGE_ON_INITIALIZE_WEIGHT)
        * MaximumBlockWeight::get();
    pub const MaximumBlockLength: u32 = 10 * 1024 * 1024;
    pub const Version: RuntimeVersion = VERSION;
}

impl frame_system::Config for Runtime {
    type Call = Call;
    type BaseCallFilter = ();
    type Version = Version;
    type AccountId = AccountId;
    type Lookup = Indices;
    type Index = Index;
    type BlockNumber = BlockNumber;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    type Event = Event;
    type Origin = Origin;
    type DbWeight = RocksDbWeight;
    type BlockHashCount = BlockHashCount;
    type BlockExecutionWeight = BlockExecutionWeight;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
    type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
    type AvailableBlockRatio = AvailableBlockRatio;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = ();
}

impl pallet_utility::Config for Runtime {
    type Call = Call;
    type Event = Event;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: Moment = MILLISECS_PER_BLOCK / 2;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const IndexDeposit: Balance = 1 * GLUSHKOV;
}

impl pallet_indices::Config for Runtime {
    type AccountIndex = AccountIndex;
    type Currency = Balances;
    type Deposit = IndexDeposit;
    type Event = Event;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * COASE;
    pub const TransferFee: Balance = 1 * GLUSHKOV;
    pub const CreationFee: Balance = 1 * GLUSHKOV;
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type MaxLocks = MaxLocks;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Runtime>;
    type WeightInfo = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 100 * COASE;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate =
        TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

parameter_types! {
    pub const BasicDeposit: Balance = 10 * XRT;       // 258 bytes on-chain
    pub const FieldDeposit: Balance = 250 * COASE;    // 66 bytes on-chain
    pub const SubAccountDeposit: Balance = 2 * XRT;   // 53 bytes on-chain
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxAdditionalFields = MaxAdditionalFields;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = ();
    type ForceOrigin = frame_system::EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type RegistrarOrigin = frame_system::EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * MaximumBlockWeight::get();
    pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type Call = Call;
    type PalletsOrigin = OriginCaller;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = ();
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 1 * XRT;
    pub const SpendPeriod: BlockNumber = 1 * DAYS;
    pub const Burn: Permill = Permill::from_percent(50);
    pub const TipCountdown: BlockNumber = 1 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: Balance = 1 * XRT;
    pub const DataDepositPerByte: Balance = 1 * COASE;
    pub const TreasuryModuleId: ModuleId = ModuleId(*b"py/trsry");
    pub const BountyDepositBase: Balance = 1 * XRT;
    pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
    pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
    pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
    pub const BountyValueMinimum: Balance = 1 * XRT;
    pub const MaximumReasonLength: u32 = 16384;
}

impl pallet_treasury::Config for Runtime {
    type ModuleId = TreasuryModuleId;
    type Currency = Balances;
    type ApproveOrigin = frame_system::EnsureOneOf<
        AccountId,
        frame_system::EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
    >;
    type RejectOrigin = frame_system::EnsureOneOf<
        AccountId,
        frame_system::EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
    >;
    type Tippers = Elections;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type Event = Event;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type SpendPeriod = SpendPeriod;
    type OnSlash = ();
    type Burn = Burn;
    type BurnDestination = ();
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type BountyCuratorDeposit = BountyCuratorDeposit;
    type BountyValueMinimum = BountyValueMinimum;
    type MaximumReasonLength = MaximumReasonLength;
    type WeightInfo = ();
}

parameter_types! {
    pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = ();
}

const DESIRED_MEMBERS: u32 = 7;
parameter_types! {
    pub const CandidacyBond: Balance = 32 * XRT;
    pub const VotingBond: Balance = 1 * XRT;
    pub const TermDuration: BlockNumber = 7 * DAYS;
    pub const DesiredMembers: u32 = DESIRED_MEMBERS;
    pub const DesiredRunnersUp: u32 = 5;
    pub const ElectionsPhragmenModuleId: LockIdentifier = *b"phrelect";
}

impl pallet_elections_phragmen::Config for Runtime {
    type ModuleId = ElectionsPhragmenModuleId;
    type Event = Event;
    type Currency = Balances;
    type ChangeMembers = Council;
    // NOTE: this implies that council's genesis members cannot be set directly and must come from
    // this module.
    type InitializeMembers = Council;
    type CurrencyToVote = U128CurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBond = VotingBond;
    type LoserCandidate = ();
    type BadReport = ();
    type KickedMember = ();
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type WeightInfo = ();
}

/*
impl cumulus_message_broker::Config for Runtime {
    type Event = Event;
    type DownwardMessageHandlers = ();
    type UpwardMessage = cumulus_upward_message::RococoUpwardMessage;
    type ParachainId = ParachainInfo;
    type XCMPMessage = pallet_robonomics_launch::XCMPMessage<Self::AccountId, bool>;
    type XCMPMessageHandlers = Launch;
}
*/

impl parachain_info::Config for Runtime {}

impl cumulus_parachain_upgrade::Config for Runtime {
    type Event = Event;
    type OnValidationData = ();
}

impl pallet_robonomics_datalog::Config for Runtime {
    type Time = Timestamp;
    type Record = Vec<u8>;
    type Event = Event;
}

impl pallet_robonomics_launch::Config for Runtime {
    type Parameter = bool;
    type Event = Event;
}

parameter_types! {
    pub const TotalBandwidth: u64 = 100; // 100 TPS allocated for RWS transactions
    pub const WeightLimit: Weight = 1_000_000_000_000_000;
    pub const PointsLimit: u64 = 10_000_000_000; // equal to 10 TPS
}

impl pallet_robonomics_rws::Config for Runtime {
    type TotalBandwidth = TotalBandwidth;
    type WeightLimit = WeightLimit;
    type PointsLimit = PointsLimit;
    type Event = Event;
    type Call = Call;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        public: <Signature as traits::Verify>::Signer,
        account: AccountId,
        nonce: Index,
    ) -> Option<(
        Call,
        <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload,
    )> {
        // take the biggest period possible.
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let current_block = System::block_number()
            .saturated_into::<u64>()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let tip = 0;
        let extra: SignedExtra = (
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra)
            .map_err(|e| {
                debug::warn!("Unable to create signed payload: {:?}", e);
            })
            .ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let address = Indices::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature.into(), extra)))
    }
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as traits::Verify>::Signer;
    type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = UncheckedExtrinsic;
}

construct_runtime! {
    pub enum Runtime where
        Block = Block,
        NodeBlock = node_primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Basic stuff.
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Utility: pallet_utility::{Module, Call, Storage, Event},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Identity: pallet_identity::{Module, Call, Storage, Event<T>},

        // Native currency and accounts.
        Indices: pallet_indices::{Module, Call, Storage, Event<T>, Config<T>},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>, Config<T>},
        TransactionPayment: pallet_transaction_payment::{Module, Storage},
        Multisig: pallet_multisig::{Module, Call, Storage, Event<T>},

        // Randomness.
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},

        // Robonomics Network modules.
        Datalog: pallet_robonomics_datalog::{Module, Call, Storage, Event<T>},
        Launch: pallet_robonomics_launch::{Module, Call, Storage, Event<T>},
        RWS: pallet_robonomics_rws::{Module, Call, Storage, Event<T>},

        // Parachain modules.
        ParachainUpgrade: cumulus_parachain_upgrade::{Module, Call, Storage, Inherent, Event},
        ParachainInfo: parachain_info::{Module, Storage, Config},

        // DAO modules
        Treasury: pallet_treasury::{Module, Call, Storage, Config, Event<T>},
        Elections: pallet_elections_phragmen::{Module, Call, Storage, Event<T>, Config<T>},
        Council: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
        Scheduler: pallet_scheduler::{Module, Call, Storage, Event<T>},

        // Sudo. Usable initially.
        Sudo: pallet_sudo::{Module, Call, Storage, Event<T>, Config<T>},
    }
}

/// The type used as a helper for interpreting the sender of transactions.
pub type Context = frame_system::ChainContext<Runtime>;

/// The address format for describing accounts.
pub type Address = <Indices as StaticLookup>::Source;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<Runtime, Block, Context, Runtime, AllModules>;

// Implement our runtime API endpoints. This is just a bunch of proxying.
impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
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

        fn random_seed() -> <Block as BlockT>::Hash {
            RandomnessCollectiveFlip::random_seed()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
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
}

cumulus_runtime::register_validate_block!(Block, Executive);
