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

pub mod robonomics;
pub mod constants;
pub mod types;
pub mod impls;

use rstd::prelude::*;
use support::{construct_runtime, parameter_types};
use primitives::OpaqueMetadata;
use sr_primitives::{
    ApplyResult, generic, create_runtime_str, key_types
};
use sr_primitives::weights::Weight;
use sr_primitives::transaction_validity::TransactionValidity;
use sr_primitives::traits::{
    self, BlakeTwo256, Block as BlockT,
    DigestFor, NumberFor, StaticLookup, SaturatedConversion,
};
use babe::{AuthorityId as BabeId};
use authority_discovery_primitives::{AuthorityId as EncodedAuthorityId, Signature as EncodedSignature};
use im_online::sr25519::{AuthorityId as ImOnlineId, AuthoritySignature as ImOnlineSignature};
use system::offchain::TransactionSubmitter;
use grandpa::fg_primitives::{self, ScheduledChange};
use grandpa::{AuthorityWeight as GrandpaWeight, AuthorityId as GrandpaId};
use client::{
    block_builder::api::{CheckInherentsResult, InherentData, self as block_builder_api},
    runtime_api, impl_runtime_apis
};
use crate::impls::{CurrencyToVoteHandler, WeightMultiplierUpdateHandler, WeightToFee};
use crate::constants::{time::*, currency::*};
use crate::types::{
    Balance, BlockNumber, Index, Hash, AccountId, AccountIndex, Moment, Signature,
};
use codec::{Encode, Decode};
use version::RuntimeVersion;
#[cfg(feature = "std")]
use version::NativeVersion;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use balances::Call as BalancesCall;
pub use timestamp::Call as TimestampCall;
pub use robonomics::Call as RobonomicsCall;
pub use sr_primitives::{Permill, Perbill, impl_opaque_keys};
pub use support::StorageValue;
pub use staking::StakerStatus;
pub use system::EventRecord;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("robonomics"),
    impl_name: create_runtime_str!("robonomics-airalab"),
    authoring_version: 1,
    spec_version: 30,
    impl_version: 0,
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

impl system::Trait for Runtime {
    type Call = Call;
    type Version = Version;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = Indices;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// TODO: doc
    type WeightMultiplierUpdate = WeightMultiplierUpdateHandler;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// TODO: doc
    type BlockHashCount = BlockHashCount;
    /// TODO: doc
    type MaximumBlockWeight = MaximumBlockWeight;
    /// TODO: doc
    type MaximumBlockLength = MaximumBlockLength;
    /// TODO: doc
    type AvailableBlockRatio = AvailableBlockRatio;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl timestamp::Trait for Runtime {
    /// A timestamp: seconds since the unix epoch.
    type Moment = Moment;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
    pub const UncleGenerations: u64 = 5;
}

parameter_types! {
    pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl babe::Trait for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
}

impl authorship::Trait for Runtime {
    type FindAuthor = session::FindAccountFromAuthorIndex<Self, Babe>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = Staking;
}

impl indices::Trait for Runtime {
    /// The type for recording indexing into the account enumeration. If this ever overflows,
    /// there will be problems!
    type AccountIndex = AccountIndex;
    /// Use the standard means of resolving an index hint from an id.
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    /// Determine whether an account is dead.
    type IsDeadAccount = Balances;
    /// The uniquitous event type.
    type Event = Event;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1 * COASE;
    pub const TransferFee: Balance = 1 * GLUSHKOV;
    pub const CreationFee: Balance = 1 * GLUSHKOV;
    pub const TransactionBaseFee: Balance = 1 * GLUSHKOV;
    pub const TransactionByteFee: Balance = 50 * COASE;
}

impl balances::Trait for Runtime {
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// What to do if an account's free balance gets zeroed.
    type OnFreeBalanceZero = (Staking, Session);
    /// What to do if a new account is created.
    type OnNewAccount = Indices;
    /// The uniquitous event type.
    type Event = Event;
    type TransactionPayment = ();
    type DustRemoval = ();
    type TransferPayment = ();
    type ExistentialDeposit = ExistentialDeposit;
    type TransferFee = TransferFee;
    type CreationFee = CreationFee;
    type TransactionBaseFee = TransactionBaseFee;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = WeightToFee;
}

type SessionHandlers = (Grandpa, Babe, ImOnline, AuthorityDiscovery);

impl_opaque_keys! {
    pub struct SessionKeys {
        #[id(key_types::GRANDPA)]
        pub grandpa: GrandpaId,
        #[id(key_types::BABE)]
        pub babe: BabeId,
        #[id(key_types::IM_ONLINE)]
        pub im_online: ImOnlineId,
    }
}

// NOTE: `SessionHandler` and `SessionKeys` are co-dependent: One key will be used for each
// handler. The number and order of items in `SessionHandler` *MUST* be the same number and order of keys
// in `SessionKeys`.

impl session::Trait for Runtime {
    type OnSessionEnding = Staking;
    type SessionHandler = SessionHandlers;
    type ShouldEndSession = Babe;
    type Event = Event;
    type Keys = SessionKeys;
    type ValidatorId = AccountId;
    type ValidatorIdOf = staking::StashOf<Self>;
    type SelectInitialValidators = Staking;
}

impl session::historical::Trait for Runtime {
    type FullIdentification = staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = staking::ExposureOf<Runtime>;
}

parameter_types! {
    pub const SessionsPerEra: sr_staking_primitives::SessionIndex = 6;
    pub const BondingDuration: staking::EraIndex = 24 * 28;
}

impl staking::Trait for Runtime {
    type Currency = Balances;
    type Time = Timestamp;
    type CurrencyToVote = CurrencyToVoteHandler;
    type OnRewardMinted = ();
    type Event = Event;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SessionInterface = Self;
}

impl grandpa::Trait for Runtime {
    type Event = Event;
}

parameter_types! {
    pub const WindowSize: BlockNumber = 101;
    pub const ReportLatency: BlockNumber = 1000;
}

impl finality_tracker::Trait for Runtime {
    type OnFinalizationStalled = Grandpa;
    type WindowSize = WindowSize;
    type ReportLatency = ReportLatency;
}

impl sudo::Trait for Runtime {
    type Proposal = Call;
    type Event = Event;
}

type SubmitTransaction = TransactionSubmitter<ImOnlineId, Runtime, UncheckedExtrinsic>;

impl im_online::Trait for Runtime {
    type Call = Call;
    type Event = Event;
    type AuthorityId = ImOnlineId;
    type SubmitTransaction = SubmitTransaction;
    type ReportUnresponsiveness = Offences;
    type CurrentElectedSet = staking::CurrentElectedStashAccounts<Runtime>;
}

impl offences::Trait for Runtime {
    type Event = Event;
    type IdentificationTuple = session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
}

impl authority_discovery::Trait for Runtime {}

impl robonomics::Trait for Runtime {
    /// Native token as processing currency.
    type Currency = Balances;
    /// The uniquitous event type.
    type Event = Event;
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = types::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // Basic stuff.
        System: system::{Module, Call, Storage, Config, Event},
        Timestamp: timestamp::{Module, Call, Storage, Inherent},

        // Native currency and accounts.
        Balances: balances,
        Indices: indices,

        // PoS consensus support.
        Session: session::{Module, Call, Storage, Event, Config<T>},
        Authorship: authorship::{Module, Call, Storage, Inherent},
        Staking: staking::{default, OfflineWorker},
        Offences: offences::{Module, Call, Storage, Event},
        Babe: babe::{Module, Call, Storage, Config, Inherent(Timestamp)},
        FinalityTracker: finality_tracker::{Module, Call, Inherent},
        Grandpa: grandpa::{Module, Call, Storage, Config, Event},
        ImOnline: im_online::{Module, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
        AuthorityDiscovery: authority_discovery::{Module, Call, Config<T>},

        // Robonomics Network support.
        Robonomics: robonomics::{Module, Call, Storage, Event<T>},

        // Sudo. Usable initially.
        Sudo: sudo,
    }
);

impl system::offchain::CreateTransaction<Runtime, UncheckedExtrinsic> for Runtime {
    type Signature = Signature;

    fn create_transaction<F: system::offchain::Signer<AccountId, Self::Signature>>(
        call: Call,
        account: AccountId,
        index: Index,
    ) -> Option<(Call, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
        let period = 1 << 8;
        let current_block = System::block_number().saturated_into::<u64>();
        let tip = 0;
        let extra: SignedExtra = (
            system::CheckVersion::<Runtime>::new(),
            system::CheckGenesis::<Runtime>::new(),
            system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
            system::CheckNonce::<Runtime>::from(index),
            system::CheckWeight::<Runtime>::new(),
            balances::TakeFees::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra).ok()?;
        let signature = F::sign(account.clone(), &raw_payload)?;
        let address = Indices::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature, extra)))
    }
}

/// The type used as a helper for interpreting the sender of transactions.
pub type Context = system::ChainContext<Runtime>;

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
    system::CheckVersion<Runtime>,
    system::CheckGenesis<Runtime>,
    system::CheckEra<Runtime>,
    system::CheckNonce<Runtime>,
    system::CheckWeight<Runtime>,
    balances::TakeFees<Runtime>
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = executive::Executive<Runtime, Block, Context, Runtime, AllModules>;

// Implement our runtime API endpoints. This is just a bunch of proxying.
impl_runtime_apis! {
    impl runtime_api::Core<Block> for Runtime {
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

    impl runtime_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
        }
    }

    impl block_builder_api::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyResult {
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
            System::random_seed()
        }
    }

    impl runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
            Executive::validate_transaction(tx)
        }
    }

    impl offchain_primitives::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(number: NumberFor<Block>) {
            Executive::offchain_worker(number)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_pending_change(digest: &DigestFor<Block>)
            -> Option<ScheduledChange<NumberFor<Block>>>
        {
            Grandpa::pending_change(digest)
        }

        fn grandpa_forced_change(digest: &DigestFor<Block>)
            -> Option<(NumberFor<Block>, ScheduledChange<NumberFor<Block>>)>
        {
            Grandpa::forced_change(digest)
        }

        fn grandpa_authorities() -> Vec<(GrandpaId, GrandpaWeight)> {
            Grandpa::grandpa_authorities()
        }
    }

    impl babe_primitives::BabeApi<Block> for Runtime {
        fn startup_data() -> babe_primitives::BabeConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time,
            // for safely resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            babe_primitives::BabeConfiguration {
                median_required_blocks: 1000,
                slot_duration: Babe::slot_duration(),
                c: (278, 1000),
            }
        }

        fn epoch() -> babe_primitives::Epoch {
            babe_primitives::Epoch {
                start_slot: Babe::epoch_start_slot(),
                authorities: Babe::authorities(),
                epoch_index: Babe::epoch_index(),
                randomness: Babe::randomness(),
                duration: EpochDuration::get(),
                secondary_slots: Babe::secondary_slots().0,
            }
        }
    }

    impl authority_discovery_primitives::AuthorityDiscoveryApi<Block> for Runtime {
        fn authorities() -> Vec<EncodedAuthorityId> {
            AuthorityDiscovery::authorities().into_iter()
                .map(|id| id.encode())
                .map(EncodedAuthorityId)
                .collect()
        }

        fn sign(payload: &Vec<u8>) -> Option<(EncodedSignature, EncodedAuthorityId)> {
            AuthorityDiscovery::sign(payload).map(|(sig, id)| {
                (EncodedSignature(sig.encode()), EncodedAuthorityId(id.encode()))
            })
        }

        fn verify(payload: &Vec<u8>, signature: &EncodedSignature, authority_id: &EncodedAuthorityId) -> bool {
            let signature = match ImOnlineSignature::decode(&mut &signature.0[..]) {
                Ok(s) => s,
                _ => return false,
            };

            let authority_id = match ImOnlineId::decode(&mut &authority_id.0[..]) {
                Ok(id) => id,
                _ => return false,
            };

            AuthorityDiscovery::verify(payload, signature, authority_id)
        }
    }

    impl types::AccountNonceApi<Block> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl substrate_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            let seed = seed.as_ref().map(|s| rstd::str::from_utf8(&s).expect("Seed is an utf8 string"));
            SessionKeys::generate(seed)
        }
    }
}
