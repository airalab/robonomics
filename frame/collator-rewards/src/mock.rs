///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
//! Minimal mock runtime for testing `AuthorRewards`.

use frame_support::{construct_runtime, derive_impl, parameter_types};

pub type AccountId = u64;
pub type Balance = u128;

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type Balance = Balance;
    type AccountStore = System;
}

parameter_types! {
    /// Per-block reward used by tests: 4_200_000 (matches the Robonomics
    /// runtime's 0.0042 XRT at 9 decimals).
    pub const TestReward: Balance = 4_200_000;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    use sp_runtime::BuildStorage;
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}
