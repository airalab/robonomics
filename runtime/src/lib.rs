// Copyright 2018 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

extern crate sr_io as runtime_io;

#[macro_use]
extern crate sr_api as runtime_api;

#[macro_use]
extern crate srml_support;

#[macro_use]
extern crate sr_primitives as runtime_primitives;

#[cfg(feature = "std")]
#[macro_use]
extern crate serde_derive;

extern crate substrate_primitives;

#[macro_use]
extern crate parity_codec_derive;

#[cfg_attr(not(feature = "std"), macro_use)]
extern crate sr_std as rstd;
extern crate srml_balances as balances;
extern crate srml_consensus as consensus;
extern crate srml_contract as contract;
extern crate srml_council as council;
extern crate srml_democracy as democracy;
extern crate srml_executive as executive;
extern crate srml_session as session;
extern crate srml_staking as staking;
extern crate srml_system as system;
extern crate srml_timestamp as timestamp;
extern crate srml_treasury as treasury;
#[macro_use]
extern crate sr_version as version;
extern crate node_primitives;

use rstd::prelude::*;
use node_primitives::{AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, SessionKey, Signature, InherentData, Timestamp as TimestampType};
use runtime_api::runtime::*;
use runtime_primitives::ApplyResult;
use runtime_primitives::transaction_validity::TransactionValidity;
use runtime_primitives::generic;
use runtime_primitives::traits::{BlakeTwo256, DigestItem, Block as BlockT};
use version::{RuntimeVersion, ApiId};
#[cfg(feature = "std")]
use version::NativeVersion;

#[cfg(any(feature = "std", test))]
pub use runtime_primitives::BuildStorage;
pub use consensus::Call as ConsensusCall;
pub use timestamp::Call as TimestampCall;
pub use balances::Call as BalancesCall;
pub use runtime_primitives::{Permill, Perbill};
pub use timestamp::BlockPeriod;
pub use srml_support::StorageValue;
#[cfg(any(feature = "std", test))]

const TIMESTAMP_SET_POSITION: u32 = 0;
const NOTE_OFFLINE_POSITION: u32 = 1;

const BLOCK_BUILDER: ApiId = *b"blkbuild";
const TAGGED_TRANSACTION_QUEUE: ApiId = *b"validatx";
const METADATA: ApiId = *b"metadata";

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: ver_str!("template-node"),
	impl_name: ver_str!("substrate-template-node"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 0,
	apis: apis_vec!([
		(BLOCK_BUILDER, 1),
		(TAGGED_TRANSACTION_QUEUE, 1),
		(METADATA, 1)
	]),
};

/// Native version.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

impl system::Trait for Runtime {
	type Origin = Origin;
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type Digest = generic::Digest<Log>;
	type AccountId = AccountId;
	type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
	type Event = Event;
	type Log = Log;
}

impl consensus::Trait for Runtime {
	const NOTE_OFFLINE_POSITION: u32 = NOTE_OFFLINE_POSITION;
	type Log = Log;
	type SessionKey = SessionKey;
	type OnOfflineValidator = ();
}

impl timestamp::Trait for Runtime {
	const TIMESTAMP_SET_POSITION: u32 = TIMESTAMP_SET_POSITION;
	type Moment = u64;
}

impl balances::Trait for Runtime {
	type Balance = Balance;
	type AccountIndex = AccountIndex;
	type OnFreeBalanceZero = ();
	type EnsureAccountLiquid = ();
	type Event = Event;
}

// TODO: v1: should be macro
impl DigestItem for Log {
	type Hash = Hash;
	type AuthorityId = SessionKey;

	fn as_authorities_change(&self) -> Option<&[Self::AuthorityId]> {
		match self.0 {
			InternalLog::consensus(ref item) => item.as_authorities_change(),
			_ => None,
		}
	}

	fn as_changes_trie_root(&self) -> Option<&Self::Hash> {
		match self.0 {
			InternalLog::system(ref item) => item.as_changes_trie_root(),
			_ => None,
		}
	}
}

construct_runtime!(
	pub enum Runtime with Log(InternalLog: DigestItem<Hash, SessionKey>) {
		System: system::{default, Log(ChangesTrieRoot)},
		Consensus: consensus::{Module, Call, Storage, Config, Log(AuthoritiesChange)},
		Balances: balances,
		Timestamp: timestamp::{Module, Call, Storage, Config},
	}
);

type Context = system::ChainContext<Runtime>;
type Address = AccountId;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256, Log>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedMortalExtrinsic<Address, Index, Call, Signature>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Index, Call>;
/// Executive: handles dispatch to the various modules.
pub type Executive = executive::Executive<Runtime, Block, Context, Balances, AllModules>;


//TODO: Move upstream into `BlockBuilder`.
pub mod block_builder_ext {
	#[cfg(feature = "std")]
	extern crate substrate_client as client;
	#[cfg(feature = "std")]
	extern crate parity_codec;

	use super::BlockT;
	use rstd::borrow::Cow;
	#[cfg(feature = "std")]
	use substrate_primitives::Blake2Hasher;
	#[cfg(feature = "std")]
	use runtime_primitives::generic::BlockId;

	#[derive(Encode, Decode, Debug)]
	pub enum Error {
		Generic(Cow<'static, str>),
		TimestampInFuture(u64),
	}

	decl_apis! {
		pub trait BlockBuilderExt<Block: BlockT> {
			fn check_inherents<InherentData>(block: Block, data: InherentData) -> Result<(), Error>;
		}
	}

	#[cfg(feature = "std")]
	impl<B, E, Block> BlockBuilderExt<Block> for client::Client<B, E, Block> where
		B: client::backend::Backend<Block, Blake2Hasher>,
		E: client::CallExecutor<Block, Blake2Hasher>,
		Block: BlockT,
	{
		type Error = client::error::Error;

		fn check_inherents<InherentData: parity_codec::Encode + parity_codec::Decode>(
			&self,
			at: &BlockId<Block>,
			block: &Block,
			data: &InherentData
		) -> Result<Result<(), Error>, Self::Error> {
			self.call_api_at(at, "check_inherents", &(block, data))
		}
	}

}
use block_builder_ext::runtime::BlockBuilderExt;
use block_builder_ext::Error as BBEError;

impl_apis! {
	impl Core<Block, SessionKey> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn authorities() -> Vec<SessionKey> {
			Consensus::authorities()
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}
	}

	impl Metadata for Runtime {
		fn metadata() -> Vec<u8> {
			Runtime::metadata()
		}
	}

	impl BlockBuilder<Block, InherentData, UncheckedExtrinsic> for Runtime {
		fn initialise_block(header: <Block as BlockT>::Header) {
			Executive::initialise_block(&header)
		}

		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalise_block() -> <Block as BlockT>::Header {
			Executive::finalise_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<UncheckedExtrinsic> {
			// TODO: v1: should be automatically gathered.
			let mut inherent = vec![generic::UncheckedMortalExtrinsic::new_unsigned(
				Call::Timestamp(TimestampCall::set(data.timestamp))
			)];

			if !data.offline_indices.is_empty() {
				inherent.push(generic::UncheckedMortalExtrinsic::new_unsigned(
					Call::Consensus(ConsensusCall::note_offline(data.offline_indices))
				));
			}

			inherent
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			System::random_seed()
		}
	}

	impl BlockBuilderExt<Block, InherentData> for Runtime {
		fn check_inherents(block: Block, data: InherentData) -> Result<(), BBEError> {
			// TODO: v1: should be automatically gathered

			// Timestamp module...
			const MAX_TIMESTAMP_DRIFT: TimestampType = 60;
			let xt = block.extrinsics.get(TIMESTAMP_SET_POSITION as usize)
				.ok_or_else(|| BBEError::Generic("No valid timestamp inherent in block".into()))?;
			let t = match (xt.is_signed(), &xt.function) {
				(false, Call::Timestamp(TimestampCall::set(t))) => t,
				_ => return Err(BBEError::Generic("No valid timestamp inherent in block".into())),
			};

			if *t > data.timestamp + MAX_TIMESTAMP_DRIFT {
				return Err(BBEError::TimestampInFuture(*t))
			}

			// Offline indices
			let noted_offline =
				block.extrinsics.get(NOTE_OFFLINE_POSITION as usize).and_then(|xt| match xt.function {
					Call::Consensus(ConsensusCall::note_offline(ref x)) => Some(&x[..]),
					_ => None,
				}).unwrap_or(&[]);

			noted_offline.iter().try_for_each(|n|
				if !data.offline_indices.contains(n) {
					Err(BBEError::Generic("Online node marked offline".into()))
				} else {
					Ok(())
				}
			)
		}
	}

	impl TaggedTransactionQueue<Block, TransactionValidity> for Runtime {
		fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
			Executive::validate_transaction(tx)
		}
	}
}
