///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
#![recursion_limit="256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod constants;
pub mod impls;

pub use pallet_staking::StakerStatus;

use sp_std::prelude::*;
use sp_core::OpaqueMetadata;
use frame_system::offchain::{
    SubmitSignedTransaction, TransactionSubmitter,
};
use frame_support::{
    construct_runtime, parameter_types,
    traits::Randomness, weights::Weight,
};
use sp_runtime::{
    ApplyExtrinsicResult, Perbill, MultiSigner,
    generic, create_runtime_str, impl_opaque_keys,
};
use sp_runtime::transaction_validity::TransactionValidity;
use sp_runtime::curve::PiecewiseLinear;
use sp_runtime::traits::{
    self, BlakeTwo256, Block as BlockT, StaticLookup, Verify,
    SaturatedConversion, OpaqueKeys, IdentifyAccount,
};
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use pallet_robonomics_agent::crypto::sr25519::AgentId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use sp_inherents::{InherentData, CheckInherentsResult};
use pallet_grandpa::{fg_primitives, AuthorityList as GrandpaAuthorityList};
use sp_api::impl_runtime_apis;
use impls::{CurrencyToVoteHandler, LinearWeightToFee, TargetedFeeAdjustment};
use node_primitives::{
    Balance, BlockNumber, Index, Hash, AccountId, AccountIndex, Moment, Signature,
};
use sp_version::RuntimeVersion;
#[cfg(feature = "std")]
use sp_version::NativeVersion;

use crate::constants::{time::*, currency::*};

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("robonomics"),
    impl_name: create_runtime_str!("robonomics-airalab"),
    authoring_version: 1,
    // Per convention: if the runtime behavior changes, increment spec_version
    // and set impl_version to equal spec_version. If only runtime
    // implementation changes and behavior does not, then leave spec_version as
    // is and increment impl_version.
    spec_version: 61,
    impl_version: 61,
    apis: RUNTIME_API_VERSIONS,
};

/// The version infromation used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const MaximumBlockWeight: Weight = 1_000_000_000;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    pub const Version: RuntimeVersion = VERSION;
}

impl frame_system::Trait for Runtime {
    type Call = Call;
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
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type ModuleToIndex = ModuleToIndex; 
}

parameter_types! {
    // One storage item; value is size 4+4+16+32 bytes = 56 bytes.
    pub const MultisigDepositBase: Balance = 30 * XRT;
    // Additional storage item size of 32 bytes.
    pub const MultisigDepositFactor: Balance = 5 * XRT;
    pub const MaxSignatories: u16 = 100;
}

impl pallet_utility::Trait for Runtime {
    type Call = Call;
    type Event = Event;
    type Currency = Balances;
    type MultisigDepositBase = MultisigDepositBase;
    type MultisigDepositFactor = MultisigDepositFactor;
    type MaxSignatories = MaxSignatories;
}

parameter_types! {
    pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
    type Moment = Moment;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
    pub const UncleGenerations: BlockNumber = 5;
}

parameter_types! {
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl pallet_babe::Trait for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
}

impl pallet_authorship::Trait for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (Staking, ImOnline);
}

impl pallet_indices::Trait for Runtime {
    /// The type for recording indexing into the account enumeration. If this ever overflows,
    /// there will be problems!
    type AccountIndex = AccountIndex;
    /// Use the standard means of resolving an index hint from an id.
    type ResolveHint = pallet_indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    /// Determine whether an account is dead.
    type IsDeadAccount = Balances;
    /// The uniquitous event type.
    type Event = Event;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * COASE;
    pub const TransferFee: Balance = 1 * GLUSHKOV;
    pub const CreationFee: Balance = 1 * GLUSHKOV;
}

impl pallet_balances::Trait for Runtime {
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// What to do if an account's free balance gets zeroed.
    type OnFreeBalanceZero = (Staking, Session);
    /// What to do if a new account is created.
    type OnNewAccount = Indices;
    /// The uniquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type TransferPayment = ();
    type ExistentialDeposit = ExistentialDeposit;
    type TransferFee = TransferFee;
    type CreationFee = CreationFee;
    type OnReapAccount = System;
}

parameter_types! {
    pub const TransactionBaseFee: Balance = 50 * GLUSHKOV;
    pub const TransactionByteFee: Balance = 0;
    // setting this to zero will disable the weight fee.
    pub const WeightFeeCoefficient: Balance = 0;
    // for a sane configuration, this should always be less than `AvailableBlockRatio`.
    pub const TargetBlockFullness: Perbill = Perbill::from_percent(25);
}

impl pallet_transaction_payment::Trait for Runtime {
    type Currency = Balances;
    type OnTransactionPayment = ();
    type TransactionBaseFee = TransactionBaseFee;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = LinearWeightToFee<WeightFeeCoefficient>;
    type FeeMultiplierUpdate = TargetedFeeAdjustment<TargetBlockFullness>;
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: Babe,
        pub grandpa: Grandpa,
        pub im_online: ImOnline,
        pub authority_discovery: AuthorityDiscovery,
    }
}

parameter_types! {
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Trait for Runtime {
    type SessionManager = Staking;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type ShouldEndSession = Babe;
    type Event = Event;
    type Keys = SessionKeys;
    type ValidatorId = <Self as frame_system::Trait>::AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
}

impl pallet_session::historical::Trait for Runtime {
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

pallet_staking_reward_curve::build! {
    const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
        min_inflation: 0_100_000,
        max_inflation: 0_800_000,
        ideal_stake: 0_666_666,
        falloff: 0_050_000,
        max_piece_count: 0_000_100,
        test_precision: 0_005_000,
    );
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 10;
    pub const BondingDuration: pallet_staking::EraIndex = 24 * 28;
    pub const SlashDeferDuration: pallet_staking::EraIndex = 24 * 7; // 1/4 the bonding duration.
    pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
}

impl pallet_staking::Trait for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type CurrencyToVote = CurrencyToVoteHandler;
    type Event = Event;
    type Slash = ();
    type Reward = ();
    type RewardRemainder = ();
    type SlashDeferDuration = SlashDeferDuration;
    type SlashCancelOrigin = frame_system::EnsureRoot<<Self as frame_system::Trait>::AccountId>;
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SessionInterface = Self;
    type RewardCurve = RewardCurve;
}

impl pallet_authority_discovery::Trait for Runtime {}

impl pallet_grandpa::Trait for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const WindowSize: BlockNumber = 101;
    pub const ReportLatency: BlockNumber = 1000;
}

impl pallet_finality_tracker::Trait for Runtime {
    type OnFinalizationStalled = Grandpa;
    type WindowSize = WindowSize;
    type ReportLatency = ReportLatency;
}

parameter_types! {
    pub const ReservationFee: Balance = 1 * XRT;
    pub const MinLength: usize = 3;
    pub const MaxLength: usize = 16;
}

impl pallet_nicks::Trait for Runtime {
    type Event = Event;
    type Currency = Balances;
    type ReservationFee = ReservationFee;
    type Slashed = ();
    type ForceOrigin = frame_system::EnsureRoot<<Self as frame_system::Trait>::AccountId>;
    type MinLength = MinLength;
    type MaxLength = MaxLength;
}

impl pallet_sudo::Trait for Runtime {
    type Proposal = Call;
    type Event = Event;
}

parameter_types! {
    pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_SLOTS as _;
}

impl pallet_im_online::Trait for Runtime {
    type Call = Call;
    type Event = Event;
    type AuthorityId = ImOnlineId;
    type SubmitTransaction = TransactionSubmitter<ImOnlineId, Runtime, UncheckedExtrinsic>;
    type ReportUnresponsiveness = Offences;
    type SessionDuration = SessionDuration;
}

impl pallet_offences::Trait for Runtime {
    type Event = Event;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
}

impl pallet_robonomics_liability::Trait for Runtime {
    type Event = Event;
    type Technics = pallet_robonomics_liability::technics::PureIPFS;
    type Economics = pallet_robonomics_liability::economics::Communism;
    type Liability = pallet_robonomics_liability::signed::SignedLiability<
        Self::Technics,
        Self::Economics,
        Signature,
        <Signature as Verify>::Signer,
        AccountId,
    >;
}

impl pallet_robonomics_provider::Trait for Runtime {
    type Call = Call;
    type Event = Event;
    type OrderHash = <Self as frame_system::Trait>::Hash;
    type OrderHashing = <Self as frame_system::Trait>::Hashing;
    type SubmitTransaction = TransactionSubmitter<(), Runtime, UncheckedExtrinsic>;
}

impl pallet_robonomics_provider::Agent for Runtime {
    type Call = Call;
    type SubmitTransaction = TransactionSubmitter<AgentId, Runtime, UncheckedExtrinsic>;
}

impl pallet_robonomics_agent::Trait for Runtime {
    type Event = Event;
    type AgentKey = AgentId;
}

impl pallet_robonomics_storage::Trait for Runtime {
    type Time = Timestamp;
    type Record = Vec<u8>;
    type Event = Event;
}

impl pallet_robonomics_storage::Agent for Runtime {
    type Call = Call;
    type SubmitTransaction = TransactionSubmitter<AgentId, Runtime, UncheckedExtrinsic>;
}

impl frame_system::offchain::CreateTransaction<Runtime, UncheckedExtrinsic> for Runtime {
    type Public = <Signature as traits::Verify>::Signer;
    type Signature = Signature;

    fn create_transaction<F: frame_system::offchain::Signer<Self::Public, Self::Signature>>(
        call: Call,
        public: Self::Public,
        account: AccountId,
        index: Index,
    ) -> Option<(Call, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
        // take the biggest period possible.
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let current_block = System::block_number().saturated_into::<u64>();
        let tip = 0;
        let extra: SignedExtra = (
            frame_system::CheckVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
            frame_system::CheckNonce::<Runtime>::from(index),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra).ok()?;
        let signature = F::sign(public, &raw_payload)?;
        let address = Indices::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature, extra)))
    }
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = node_primitives::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Basic stuff.
        System: frame_system::{Module, Call, Storage, Config, Event},
        Utility: pallet_utility::{Module, Call, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Nicks: pallet_nicks::{Module, Call, Storage, Event<T>},

        // Native currency and accounts.
        Indices: pallet_indices,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment::{Module, Storage},

        // Randomness.
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},

        // PoS consensus modules.
        Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
        Authorship: pallet_authorship::{Module, Call, Storage, Inherent},
        Staking: pallet_staking,
        Offences: pallet_offences::{Module, Call, Storage, Event},
        Babe: pallet_babe::{Module, Call, Storage, Config, Inherent(Timestamp)},
        FinalityTracker: pallet_finality_tracker::{Module, Call, Inherent},
        Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
        ImOnline: pallet_im_online::{Module, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
        AuthorityDiscovery: pallet_authority_discovery::{Module, Call, Config},

        // Robonomics Network modules.
        RobonomicsLiability: pallet_robonomics_liability::{Module, Call, Storage, Event<T>, ValidateUnsigned},
        RobonomicsProvider: pallet_robonomics_provider::{Module, Call, Storage, Event<T>},
        RobonomicsStorage: pallet_robonomics_storage::{Module, Call, Storage, Event<T>},
        RobonomicsAgent: pallet_robonomics_agent::{Module, Call, Storage, Event},

        // Sudo. Usable initially.
        Sudo: pallet_sudo,
    }
);

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
    frame_system::CheckVersion<Runtime>,
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
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
            data.check_extrinsics(&block)
        }

        fn random_seed() -> <Block as BlockT>::Hash {
            RandomnessCollectiveFlip::random_seed()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
            Executive::validate_transaction(tx)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time, for safely
            // resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            sp_consensus_babe::BabeConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: PRIMARY_PROBABILITY,
                genesis_authorities: Babe::authorities(),
                randomness: Babe::randomness(),
                secondary_slots: true,
            }
        }
    }

    impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
        fn authorities() -> Vec<AuthorityDiscoveryId> {
            AuthorityDiscovery::authorities()
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
        UncheckedExtrinsic,
    > for Runtime {
        fn query_info(uxt: UncheckedExtrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl pallet_robonomics_agent_runtime_api::RobonomicsAgentApi<Block, Runtime> for Runtime {
        fn account() -> AccountId {
            MultiSigner::from(RobonomicsAgent::account()).into_account()
        }
    }

    impl pallet_robonomics_agent_runtime_api::RobonomicsLiabilityApi<Block, Runtime> for Runtime {
        fn pull() -> Option<Vec<u8>> {
            None
        }

        fn recv() -> Vec<pallet_robonomics_provider::RobonomicsMessage<Runtime>> {
            vec![]
        }

        fn send_demand(
            technics: pallet_robonomics_liability::TechnicalParam<Runtime>,
            economics: pallet_robonomics_liability::EconomicalParam<Runtime>,
        ) -> Result<(), ()> {
            RobonomicsAgent::send_demand(technics, economics)
        }

        fn send_offer(
            technics: pallet_robonomics_liability::TechnicalParam<Runtime>,
            economics: pallet_robonomics_liability::EconomicalParam<Runtime>,
        ) -> Result<(), ()> {
            RobonomicsAgent::send_offer(technics, economics)
        }

        fn send_report(
            index: pallet_robonomics_liability::LiabilityIndex<Runtime>,
            report: pallet_robonomics_liability::TechnicalReport<Runtime>,
        ) -> Result<(), ()> {
            RobonomicsAgent::send_report(index, report)
        }
    }

    impl pallet_robonomics_agent_runtime_api::RobonomicsBlockchainApi<Block, Runtime> for Runtime {
        fn send_record(
            record: Vec<u8> 
        ) -> Result<(), ()> {
            RobonomicsAgent::send_record(record)
        }
    }
}
