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
//! Weight functions of `pallet-robonomics-subscription`.

use core::marker::PhantomData;
use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};

/// Weights of the pallet's operations.
///
/// The pallet has no extrinsics: `allocate` and `revoke` are called by
/// allocators, `consume_*` and `correct` by resource consumers, and the
/// expiration weights are charged by `on_initialize`.
pub trait WeightInfo {
    /// `allocate` replacing a subscription with `resources` resources.
    fn allocate(resources: u32) -> Weight;
    /// `revoke` of a subscription with `resources` resources.
    fn revoke(resources: u32) -> Weight;
    /// `consume_global`.
    fn consume_global() -> Weight;
    /// `consume_local`.
    fn consume_local() -> Weight;
    /// `correct` of a global or local charge.
    fn correct() -> Weight;
    /// Scanning one expiration bucket during `on_initialize`.
    fn scan_bucket() -> Weight;
    /// Processing one agenda entry during `on_initialize`, releasing the
    /// subscription if the entry is not stale.
    fn expire_entry() -> Weight;
}

/// Conservative weights derived from storage access counts. They should be
/// replaced by benchmarked weights.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn allocate(resources: u32) -> Weight {
        Weight::from_parts(40_000_000, 4_000)
            .saturating_add(Weight::from_parts(10_000_000, 1_000).saturating_mul(resources.into()))
            .saturating_add(T::DbWeight::get().reads(6_u64.saturating_add(2 * resources as u64)))
            .saturating_add(T::DbWeight::get().writes(6_u64.saturating_add(3 * resources as u64)))
    }

    fn revoke(resources: u32) -> Weight {
        Weight::from_parts(20_000_000, 3_000)
            .saturating_add(Weight::from_parts(5_000_000, 500).saturating_mul(resources.into()))
            .saturating_add(T::DbWeight::get().reads(3_u64.saturating_add(resources as u64)))
            .saturating_add(T::DbWeight::get().writes(3_u64.saturating_add(2 * resources as u64)))
    }

    fn consume_global() -> Weight {
        Weight::from_parts(20_000_000, 3_000)
            .saturating_add(T::DbWeight::get().reads(3))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn consume_local() -> Weight {
        Weight::from_parts(30_000_000, 4_000)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn correct() -> Weight {
        Weight::from_parts(30_000_000, 4_000)
            .saturating_add(T::DbWeight::get().reads(4))
            .saturating_add(T::DbWeight::get().writes(2))
    }

    fn scan_bucket() -> Weight {
        Weight::from_parts(5_000_000, 1_000)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn expire_entry() -> Weight {
        Weight::from_parts(30_000_000, 4_000)
            .saturating_add(T::DbWeight::get().reads(5))
            .saturating_add(T::DbWeight::get().writes(5))
    }
}

impl WeightInfo for () {
    fn allocate(resources: u32) -> Weight {
        Weight::from_parts(40_000_000, 4_000)
            .saturating_add(Weight::from_parts(10_000_000, 1_000).saturating_mul(resources.into()))
            .saturating_add(RocksDbWeight::get().reads(6_u64.saturating_add(2 * resources as u64)))
            .saturating_add(RocksDbWeight::get().writes(6_u64.saturating_add(3 * resources as u64)))
    }

    fn revoke(resources: u32) -> Weight {
        Weight::from_parts(20_000_000, 3_000)
            .saturating_add(Weight::from_parts(5_000_000, 500).saturating_mul(resources.into()))
            .saturating_add(RocksDbWeight::get().reads(3_u64.saturating_add(resources as u64)))
            .saturating_add(RocksDbWeight::get().writes(3_u64.saturating_add(2 * resources as u64)))
    }

    fn consume_global() -> Weight {
        Weight::from_parts(20_000_000, 3_000)
            .saturating_add(RocksDbWeight::get().reads(3))
            .saturating_add(RocksDbWeight::get().writes(1))
    }

    fn consume_local() -> Weight {
        Weight::from_parts(30_000_000, 4_000)
            .saturating_add(RocksDbWeight::get().reads(4))
            .saturating_add(RocksDbWeight::get().writes(2))
    }

    fn correct() -> Weight {
        Weight::from_parts(30_000_000, 4_000)
            .saturating_add(RocksDbWeight::get().reads(4))
            .saturating_add(RocksDbWeight::get().writes(2))
    }

    fn scan_bucket() -> Weight {
        Weight::from_parts(5_000_000, 1_000)
            .saturating_add(RocksDbWeight::get().reads(1))
            .saturating_add(RocksDbWeight::get().writes(1))
    }

    fn expire_entry() -> Weight {
        Weight::from_parts(30_000_000, 4_000)
            .saturating_add(RocksDbWeight::get().reads(5))
            .saturating_add(RocksDbWeight::get().writes(5))
    }
}
