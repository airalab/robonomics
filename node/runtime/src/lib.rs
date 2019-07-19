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

#[cfg(feature = "std")]
use serde_derive::{Serialize, Deserialize};
#[cfg(feature = "std")]
use primitives::bytes;

use rstd::prelude::*;
use support::{construct_runtime, parameter_types};
use parity_codec::{Encode, Decode};
use primitives::OpaqueMetadata;
use runtime_primitives::{
    ApplyResult, AnySignature, Fixed64, generic, create_runtime_str, key_types
};
use runtime_primitives::transaction_validity::TransactionValidity;
use runtime_primitives::weights::{Weight, WeightMultiplier, MAX_TRANSACTIONS_WEIGHT, IDEAL_TRANSACTIONS_WEIGHT};
use runtime_primitives::traits::{
    self, Verify, BlakeTwo256, Block as BlockT, Convert,
    DigestFor, NumberFor, StaticLookup, Saturating
};
use grandpa::fg_primitives::{self, ScheduledChange};
use grandpa::{AuthorityWeight as GrandpaWeight};
use finality_tracker::{DEFAULT_REPORT_LATENCY, DEFAULT_WINDOW_SIZE};
use client::{
    block_builder::api::{CheckInherentsResult, InherentData, self as block_builder_api},
    runtime_api, impl_runtime_apis
};
use version::RuntimeVersion;
#[cfg(feature = "std")]
use version::NativeVersion;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use balances::Call as BalancesCall;
pub use timestamp::Call as TimestampCall;
pub use robonomics::Call as RobonomicsCall;
pub use runtime_primitives::{Permill, Perbill, impl_opaque_keys};
pub use support::StorageValue;
pub use staking::StakerStatus;
pub use system::EventRecord;
pub use grandpa::{AuthorityId as GrandpaId};

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Alias to 512-bit hash when used in the context of a signature on the chain.
pub type Signature = AnySignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <Signature as Verify>::Signer;

/// The aura crypto scheme defined via the keypair type.
#[cfg(feature = "std")]
pub type AuraPair = primitives::ed25519::Pair;

/// Alias to the signature scheme used for Aura authority signatures.
pub type AuraSignature = primitives::ed25519::Signature;

/// The Ed25519 pub key of an session that belongs to an Aura authority of the chain.
pub type AuraId = primitives::ed25519::Public;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Balance of an account.
pub type Balance = u128;

/// A hash of some data used by the chain.
pub type Hash = primitives::H256;

/// Index of a block number in the chain.
pub type BlockNumber = u64;

/// Index of an account's extrinsic in the chain.
pub type Nonce = u64;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core datastructures.
pub mod opaque {
    use super::*;

    /// Opaque, encoded, unchecked extrinsic.
    #[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
    pub struct UncheckedExtrinsic(#[cfg_attr(feature = "std", serde(with="bytes"))] pub Vec<u8>);
    impl traits::Extrinsic for UncheckedExtrinsic {
        fn is_signed(&self) -> Option<bool> {
            None
        }
    }
    /// Digest item type.
    pub type DigestItem = generic::DigestItem<Hash>;
    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("robonomics"),
    impl_name: create_runtime_str!("robonomics-node"),
    authoring_version: 1,
    spec_version: 30,
    impl_version: 30,
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
}

/// A struct that updates the weight multiplier based on the saturation level of the previous block.
/// This should typically be called once per-block.
///
/// This assumes that weight is a numeric value in the u32 range.
///
/// Formula:
///   diff = (ideal_weight - current_block_weight)
///   v = 0.00004
///   next_weight = weight * (1 + (v . diff) + (v . diff)^2 / 2)
///
/// https://research.web3.foundation/en/latest/polkadot/Token%20Economics/#relay-chain-transaction-fees
pub struct WeightMultiplierUpdateHandler;
impl Convert<(Weight, WeightMultiplier), WeightMultiplier> for WeightMultiplierUpdateHandler {
	fn convert(previous_state: (Weight, WeightMultiplier)) -> WeightMultiplier {
		let (block_weight, multiplier) = previous_state;
		let ideal = IDEAL_TRANSACTIONS_WEIGHT as u128;
		let block_weight = block_weight as u128;

		// determines if the first_term is positive
		let positive = block_weight >= ideal;
		let diff_abs = block_weight.max(ideal) - block_weight.min(ideal);
		// diff is within u32, safe.
		let diff = Fixed64::from_rational(diff_abs as i64, MAX_TRANSACTIONS_WEIGHT as u64);
		let diff_squared = diff.saturating_mul(diff);

		// 0.00004 = 4/100_000 = 40_000/10^9
		let v = Fixed64::from_rational(4, 100_000);
		// 0.00004^2 = 16/10^10 ~= 2/10^9. Taking the future /2 into account, then it is just 1 parts
		// from a billionth.
		let v_squared_2 = Fixed64::from_rational(1, 1_000_000_000);

		let first_term = v.saturating_mul(diff);
		// It is very unlikely that this will exist (in our poor perbill estimate) but we are giving
		// it a shot.
		let second_term = v_squared_2.saturating_mul(diff_squared);

		if positive {
			let excess = first_term.saturating_add(second_term);
			multiplier.saturating_add(WeightMultiplier::from_fixed(excess))
		} else {
			// first_term > second_term
			let negative = first_term - second_term;
			multiplier.saturating_sub(WeightMultiplier::from_fixed(negative))
				// despite the fact that apply_to saturates weight (final fee cannot go below 0)
				// it is crucially important to stop here and don't further reduce the weight fee
				// multiplier. While at -1, it means that the network is so un-congested that all
				// transactions are practically free. We stop here and only increase if the network
				// became more busy.
				.max(WeightMultiplier::from_rational(-1, 1))
		}
	}
}

impl system::Trait for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = Indices;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Nonce;
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
}

parameter_types! {
    pub const MinimumPeriod: u64 = SECS_PER_BLOCK / 2;
}

impl timestamp::Trait for Runtime {
    /// A timestamp: seconds since the unix epoch.
    type Moment = Moment;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
        pub const UncleGenerations: u64 = 0;
}

// TODO: #2986 implement this properly
impl authorship::Trait for Runtime {
    type FindAuthor = ();
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = ();
}

impl aura::Trait for Runtime {
    type HandleReport = aura::StakingSlasher<Runtime>;
    type AuthorityId = AuraId;
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
}

pub const MINUTES: Moment = 60 / SECS_PER_BLOCK;
pub const HOURS: Moment = MINUTES * 60;
pub const DAYS: Moment = HOURS * 24;

parameter_types! {
    pub const Period: BlockNumber = 10 * MINUTES;
    pub const Offset: BlockNumber = 0;
}

type SessionHandlers = (Grandpa, Aura);

impl_opaque_keys! {
    pub struct SessionKeys {
        #[id(key_types::ED25519)]
        pub ed25519: GrandpaId,
    }
}

// NOTE: `SessionHandler` and `SessionKeys` are co-dependent: One key will be used for each
// handler. The number and order of items in `SessionHandler` *MUST* be the same number and order of keys
// in `SessionKeys`.

impl session::Trait for Runtime {
    type OnSessionEnding = Staking;
    type SessionHandler = SessionHandlers;
    type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
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
    pub const SessionsPerEra: session::SessionIndex = 6;
    pub const BondingDuration: staking::EraIndex = 24 * 28;
}

pub const COASE: Balance = 1_000;
pub const GLUSHKOV: Balance = 1_000 * COASE;
pub const XRT: Balance = 1_000 * GLUSHKOV;

pub struct CurrencyToVoteHandler;

impl CurrencyToVoteHandler {
    fn factor() -> u128 { (Balances::total_issuance() / u64::max_value() as u128).max(1) }
}

impl Convert<u128, u64> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u64 { (x / Self::factor()) as u64 }
}

impl Convert<u128, u128> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u128 { x * Self::factor() }
}

pub const SECS_PER_BLOCK: Moment = 4;

impl staking::Trait for Runtime {
    type Currency = Balances;
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
    pub const WindowSize: BlockNumber = DEFAULT_WINDOW_SIZE.into();
    pub const ReportLatency: BlockNumber = DEFAULT_REPORT_LATENCY.into();
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

impl robonomics::Trait for Runtime {
    /// Native token as processing currency.
    type Currency = Balances;
    /// The uniquitous event type.
    type Event = Event;
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: system::{Module, Call, Storage, Config, Event},
        Aura: aura::{Module, Config<T>, Inherent(Timestamp)},
        Timestamp: timestamp::{Module, Call, Storage, Inherent},
        Authorship: authorship::{Module, Call, Storage},
        Indices: indices,
        Balances: balances,
        Session: session::{Module, Call, Storage, Event, Config<T>},
        Staking: staking::{default, OfflineWorker},
        FinalityTracker: finality_tracker::{Module, Call, Inherent},
        Grandpa: grandpa::{Module, Call, Storage, Config, Event},
        Sudo: sudo,
        Robonomics: robonomics::{Module, Call, Storage, Event<T>},
    }
);

/// The type used as a helper for interpreting the sender of transactions.
type Context = system::ChainContext<Runtime>;
/// The address format for describing accounts.
type Address = <Indices as StaticLookup>::Source;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedMortalCompactExtrinsic<Address, Nonce, Call, Signature>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Nonce, Call>;
/// Executive: handles dispatch to the various modules.
pub type Executive = executive::Executive<Runtime, Block, Context, Balances, Runtime, AllModules>;

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

    impl consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> u64 {
            Aura::slot_duration()
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities()
        }
    }
}
