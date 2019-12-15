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
//! Robonomics runtime test utilities.

use super::*;
use crate::technics::PureIPFS;
use crate::economics::Communism;
use crate::signed::SignedLiability;
use sp_runtime::{Perbill, traits::Verify, testing::Header};
use primitives::H256;
use runtime_io;
use support::{impl_outer_origin, parameter_types};
use support::weights::Weight;
use node_primitives::{AccountIndex, AccountId, Signature};

impl_outer_origin!{
	pub enum Origin for Runtime {}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl system::Trait for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = ();
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
    type Lookup = Indices;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}
impl indices::Trait for Runtime {
    type AccountIndex = AccountIndex;
    type ResolveHint = indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
    type IsDeadAccount = ();
    type Event = ();
}
impl Trait for Runtime {
    type Technics = PureIPFS;
    type Economics = Communism;
    type Liability = SignedLiability<Self::Technics, Self::Economics, <Signature as Verify>::Signer, Signature>;
    type Event = ();
}

pub fn new_test_ext() -> runtime_io::TestExternalities {
	let storage = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
    storage.into()
}

pub type System = system::Module<Runtime>;
pub type Indices = indices::Module<Runtime>;
pub type Liability = Module<Runtime>;
