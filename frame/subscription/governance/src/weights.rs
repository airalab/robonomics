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
//! Weight functions of `pallet-robonomics-subscription-governance`.

use core::marker::PhantomData;
use polkadot_sdk::{
    frame_support::{
        traits::Get,
        weights::{constants::RocksDbWeight, Weight},
    },
    frame_system,
};

/// Weights of the pallet's extrinsics.
///
/// The figures include the work of the subscription allocator behind the
/// extrinsic, since the pallet only forwards to it.
pub trait WeightInfo {
    /// `allocate` with `resources` resources.
    fn allocate(resources: u32) -> Weight;
    /// `revoke` of a subscription with up to `resources` resources.
    fn revoke(resources: u32) -> Weight;
}

/// Conservative weights derived from storage access counts. They should be
/// replaced by benchmarked weights.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn allocate(resources: u32) -> Weight {
        Weight::from_parts(45_000_000, 4_000)
            .saturating_add(Weight::from_parts(10_000_000, 1_000).saturating_mul(resources.into()))
            .saturating_add(T::DbWeight::get().reads(6_u64.saturating_add(2 * resources as u64)))
            .saturating_add(T::DbWeight::get().writes(6_u64.saturating_add(3 * resources as u64)))
    }

    fn revoke(resources: u32) -> Weight {
        Weight::from_parts(25_000_000, 3_000)
            .saturating_add(Weight::from_parts(5_000_000, 500).saturating_mul(resources.into()))
            .saturating_add(T::DbWeight::get().reads(3_u64.saturating_add(resources as u64)))
            .saturating_add(T::DbWeight::get().writes(3_u64.saturating_add(2 * resources as u64)))
    }
}

impl WeightInfo for () {
    fn allocate(resources: u32) -> Weight {
        Weight::from_parts(45_000_000, 4_000)
            .saturating_add(Weight::from_parts(10_000_000, 1_000).saturating_mul(resources.into()))
            .saturating_add(RocksDbWeight::get().reads(6_u64.saturating_add(2 * resources as u64)))
            .saturating_add(RocksDbWeight::get().writes(6_u64.saturating_add(3 * resources as u64)))
    }

    fn revoke(resources: u32) -> Weight {
        Weight::from_parts(25_000_000, 3_000)
            .saturating_add(Weight::from_parts(5_000_000, 500).saturating_mul(resources.into()))
            .saturating_add(RocksDbWeight::get().reads(3_u64.saturating_add(resources as u64)))
            .saturating_add(RocksDbWeight::get().writes(3_u64.saturating_add(2 * resources as u64)))
    }
}
