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
//! Weight functions of `pallet-robonomics-transaction`.

use core::marker::PhantomData;
use polkadot_sdk::{
    frame_support::{traits::Get, weights::Weight},
    frame_system,
};

/// Weights of the transaction extension.
///
/// The pallet has no extrinsics; the weight is the overhead of the
/// `ChargeTransaction` extension itself.
pub trait WeightInfo {
    /// Worst-case overhead of a Subscription-funded transaction: node-path
    /// authorization and local accounting, each performed in validation and
    /// in pre-dispatch, plus the post-dispatch correction.
    fn subscription_charge() -> Weight;
}

/// Conservative weights derived from storage access counts. They should be
/// replaced by benchmarked weights.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn subscription_charge() -> Weight {
        // Two authorizations walk at most a full Scope path each (node and
        // access reads), then local and global accounting is read three times.
        Weight::from_parts(150_000_000, 250_000)
            .saturating_add(T::DbWeight::get().reads(150))
            .saturating_add(T::DbWeight::get().writes(4))
    }
}

impl WeightInfo for () {
    fn subscription_charge() -> Weight {
        Weight::from_parts(150_000_000, 250_000)
    }
}
