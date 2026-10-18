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
//! Weight functions of `pallet-robonomics-subscription-purchase`.

use core::marker::PhantomData;
use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};

/// Weights of the pallet's extrinsics.
///
/// Deposit purchases are charged by the XCM weigher of the runtime, as part of
/// the instruction that deposits the asset.
pub trait WeightInfo {
    /// `create_product` with `resources` resources.
    fn create_product(resources: u32) -> Weight;
    /// `update_product` with `resources` resources.
    fn update_product(resources: u32) -> Weight;
    /// `disable_product`.
    fn disable_product() -> Weight;
}

/// Conservative weights derived from storage access counts. They should be
/// replaced by benchmarked weights.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn create_product(resources: u32) -> Weight {
        Weight::from_parts(15_000_000, 2_000)
            .saturating_add(Weight::from_parts(1_000_000, 100).saturating_mul(resources.into()))
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn update_product(resources: u32) -> Weight {
        Weight::from_parts(15_000_000, 2_000)
            .saturating_add(Weight::from_parts(1_000_000, 100).saturating_mul(resources.into()))
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn disable_product() -> Weight {
        Weight::from_parts(10_000_000, 2_000)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }
}

impl WeightInfo for () {
    fn create_product(resources: u32) -> Weight {
        Weight::from_parts(15_000_000, 2_000)
            .saturating_add(Weight::from_parts(1_000_000, 100).saturating_mul(resources.into()))
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(2))
    }

    fn update_product(resources: u32) -> Weight {
        Weight::from_parts(15_000_000, 2_000)
            .saturating_add(Weight::from_parts(1_000_000, 100).saturating_mul(resources.into()))
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(1))
    }

    fn disable_product() -> Weight {
        Weight::from_parts(10_000_000, 2_000)
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(1))
    }
}
