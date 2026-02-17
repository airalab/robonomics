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
//! Mock runtime for the Robonomics Claim pallet tests.

#[cfg(test)]
use super::*;
use secp_utils::*;

// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate as claims;
use frame_support::{derive_impl, parameter_types, PalletId};
use pallet_balances;
use sp_runtime::{traits::AccountIdConversion, BuildStorage};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Claims: claims,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type AccountData = pallet_balances::AccountData<u64>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type AccountStore = System;
}

parameter_types! {
    pub Prefix: &'static [u8] = b"Pay RUSTs to the TEST account:";
    pub ClaimPalletId: PalletId = PalletId(*b"claimerc");
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletId = ClaimPalletId;
    type Prefix = Prefix;
    type WeightInfo = weights::TestWeightInfo;
}

pub fn alice() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap()
}
pub fn bob() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Bob")).unwrap()
}
pub fn dave() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Dave")).unwrap()
}
pub fn eve() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Eve")).unwrap()
}
pub fn frank() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Frank")).unwrap()
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let claims_account_id = ClaimPalletId::get().into_account_truncating();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(claims_account_id, 1_000)],
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();
    claims::GenesisConfig::<Test> {
        claims: vec![
            (eth(&alice()), 100),
            (eth(&dave()), 200),
            (eth(&eve()), 300),
            (eth(&frank()), 400),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
