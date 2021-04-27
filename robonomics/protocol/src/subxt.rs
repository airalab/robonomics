///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! SubXt compatible Robonomics Network types.

use sp_runtime::{
    generic::Header,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    MultiSignature, OpaqueExtrinsic,
};
use substrate_subxt::{
    balances::{self, BalancesEventTypeRegistry},
    extrinsic::DefaultExtra,
    register_default_type_sizes,
    system::{self, SystemEventTypeRegistry},
    EventTypeRegistry, Runtime,
};

pub mod datalog;
pub mod launch;
pub mod pallet_datalog;
pub mod pallet_launch;

use pallet_datalog::DatalogEventTypeRegistry;
use pallet_launch::LaunchEventTypeRegistry;

/// Robonomics Network family substrate runtimes.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Robonomics;

/// Robonomics account is MultiSignature account.
pub type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Datalog window size.
pub const WindowSize: u64 = 128;

impl Runtime for Robonomics {
    type Signature = MultiSignature;
    type Extra = DefaultExtra<Self>;

    fn register_type_sizes(event_type_registry: &mut EventTypeRegistry<Self>) {
        event_type_registry.with_system();
        event_type_registry.with_balances();
        event_type_registry.with_datalog();
        event_type_registry.with_launch();
        register_default_type_sizes(event_type_registry);
    }
}

impl system::System for Robonomics {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Address = Self::AccountId;
    type Header = Header<Self::BlockNumber, BlakeTwo256>;
    type Extrinsic = OpaqueExtrinsic;
    type AccountData = balances::AccountData<<Self as balances::Balances>::Balance>;
}

impl balances::Balances for Robonomics {
    type Balance = u128;
}

impl pallet_datalog::Datalog for Robonomics {
    type Record = Vec<u8>;
}

impl pallet_launch::Launch for Robonomics {
    type Parameter = bool;
}
