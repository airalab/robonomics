//! The Robonomics runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

extern crate sr_std as rstd;
#[macro_use]
extern crate substrate_client as client;
#[macro_use]
extern crate srml_support;
extern crate sr_primitives as runtime_primitives;
#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;
extern crate substrate_primitives as primitives;
extern crate parity_codec;
#[macro_use]
extern crate parity_codec_derive;
#[macro_use]
extern crate sr_version as version;
extern crate srml_fees as fees;
extern crate srml_sudo as sudo;
extern crate srml_aura as aura;
extern crate srml_system as system;
extern crate srml_session as session;
extern crate srml_staking as staking;
extern crate srml_grandpa as grandpa;
extern crate srml_indices as indices;
extern crate srml_balances as balances;
extern crate srml_executive as executive;
extern crate srml_consensus as consensus;
extern crate srml_timestamp as timestamp;
extern crate substrate_consensus_aura_primitives as consensus_aura;


pub mod robonomics;

use rstd::prelude::*;
#[cfg(feature = "std")]
use primitives::bytes;
use primitives::{Ed25519AuthorityId, OpaqueMetadata};
use runtime_primitives::{
    ApplyResult, transaction_validity::TransactionValidity, Ed25519Signature, generic,
    traits::{self, Verify, Convert, BlakeTwo256, Block as BlockT, DigestFor, NumberFor, StaticLookup},
};
use grandpa::fg_primitives::{self, ScheduledChange};
use client::{
    block_builder::api::{CheckInherentsResult, InherentData, self as block_builder_api},
    runtime_api
};
use version::RuntimeVersion;
#[cfg(feature = "std")]
use version::NativeVersion;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use balances::Call as BalancesCall;
pub use consensus::Call as ConsensusCall;
pub use timestamp::Call as TimestampCall;
pub use robonomics::Call as RobonomicsCall;
pub use runtime_primitives::{Permill, Perbill};
pub use srml_support::StorageValue;
pub use timestamp::BlockPeriod;
pub use system::EventRecord;

/// Alias to 512-bit hash when used in the context of a signature on the chain.
pub type Signature = Ed25519Signature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <Signature as Verify>::Signer;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// The Ed25519 pub key of an session that belongs to an authority of the chain. This is
/// exactly equivalent to what the substrate calls an "authority".
pub type SessionKey = primitives::Ed25519AuthorityId;

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
    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256, generic::DigestItem<Hash, SessionKey>>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;
    /// Opaque session key type.
    pub type SessionKey = Ed25519AuthorityId;
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("robonomics"),
    impl_name: create_runtime_str!("robonomics-node"),
    authoring_version: 1,
    spec_version: 20,
    impl_version: 1,
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
    /// The header digest type.
    type Digest = generic::Digest<Log>;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous log type.
    type Log = Log;
    /// The ubiquitous origin type.
    type Origin = Origin;
}

impl timestamp::Trait for Runtime {
    /// A timestamp: seconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
}

impl aura::Trait for Runtime {
    type HandleReport = aura::StakingSlasher<Runtime>;
}

impl consensus::Trait for Runtime {
    /// The identifier we use to refer to authorities.
    type SessionKey = SessionKey;
    // The aura module handles offline-reports internally
    // rather than using an explicit report system.
    type InherentOfflineReport = ();
    /// The ubiquitous log type.
    type Log = Log;
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

impl balances::Trait for Runtime {
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// What to do if an account's free balance gets zeroed.
    type OnFreeBalanceZero = (Staking, Session);
    /// What to do if a new account is created.
    type OnNewAccount = Indices;
    /// The uniquitous event type.
    type Event = Event;
}

impl fees::Trait for Runtime {
	type TransferAsset = Balances;
	type Event = Event;
}

/// Session key conversion.
pub struct SessionKeyConversion;
impl Convert<AccountId, SessionKey> for SessionKeyConversion {
    fn convert(a: AccountId) -> SessionKey {
        a.to_fixed_bytes().into()
    }
}

impl session::Trait for Runtime {
    type ConvertAccountIdToSessionKey = SessionKeyConversion;
    type OnSessionChange = (Staking, grandpa::SyncedAuthorities<Runtime>);
    type Event = Event;
}

impl staking::Trait for Runtime {
    type Currency = balances::Module<Self>;
    type OnRewardMinted = ();
    type Event = Event;
}

impl grandpa::Trait for Runtime {
    type SessionKey = SessionKey;
    type Log = Log;
    type Event = Event;
}

impl sudo::Trait for Runtime {
    type Event = Event;
    type Proposal = Call;
}

impl robonomics::Trait for Runtime {
    type Event = Event;
}

construct_runtime!(
    pub enum Runtime with Log(InternalLog: DigestItem<Hash, SessionKey>) where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: system::{default, Log(ChangesTrieRoot)},
        Timestamp: timestamp::{Module, Call, Storage, Config<T>, Inherent},
        Consensus: consensus::{Module, Call, Storage, Config<T>, Log(AuthoritiesChange), Inherent},
        Aura: aura::{Module},
        Session: session,
        Staking: staking::{default, OfflineWorker},
        Grandpa: grandpa::{Module, Call, Storage, Config<T>, Log(), Event<T>},
        Indices: indices,
        Balances: balances,
        Fees: fees::{Module, Storage, Config<T>, Event<T>},
        Sudo: sudo,
        Robonomics: robonomics::{Module, Call, Storage, Event<T>},
    }
);

/// The type used as a helper for interpreting the sender of transactions.
type Context = system::ChainContext<Runtime>;
/// The address format for describing accounts.
type Address = <Indices as StaticLookup>::Source;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedMortalCompactExtrinsic<Address, Nonce, Call, Ed25519Signature>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Nonce, Call>;
/// Executive: handles dispatch to the various modules.
pub type Executive = executive::Executive<Runtime, Block, Context, Fees, AllModules>;

// Implement our runtime API endpoints. This is just a bunch of proxying.
impl_runtime_apis! {
    impl runtime_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn authorities() -> Vec<SessionKey> {
            Consensus::authorities()
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialise_block(header: &<Block as BlockT>::Header) {
            Executive::initialise_block(header)
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

        fn finalise_block() -> <Block as BlockT>::Header {
            Executive::finalise_block()
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

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_pending_change(digest: &DigestFor<Block>)
            -> Option<ScheduledChange<NumberFor<Block>>>
        {
            for log in digest.logs.iter().filter_map(|l| match l {
                Log(InternalLog::grandpa(grandpa_signal)) => Some(grandpa_signal),
                _=> None
            }) {
                if let Some(change) = Grandpa::scrape_digest_change(log) {
                    return Some(change);
                }
            }
            None
        }
        fn grandpa_forced_change(digest: &DigestFor<Block>)
            -> Option<(NumberFor<Block>, ScheduledChange<NumberFor<Block>>)>
        {
            for log in digest.logs.iter().filter_map(|l| match l {
                Log(InternalLog::grandpa(grandpa_signal)) => Some(grandpa_signal),
                _ => None
            }) {
                if let Some(change) = Grandpa::scrape_digest_forced_change(log) {
                    return Some(change);
                }
            }
            None
        }

        fn grandpa_authorities() -> Vec<(SessionKey, u64)> {
            Grandpa::grandpa_authorities()
        }
    }

    impl consensus_aura::AuraApi<Block> for Runtime {
        fn slot_duration() -> u64 {
            Aura::slot_duration()
        }
    }
}
