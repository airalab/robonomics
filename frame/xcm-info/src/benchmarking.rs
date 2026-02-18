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
//! Benchmarks for XcmInfo Pallet

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_std::prelude::*;

#[benchmarks]
mod benchmarks {
    use super::*;
    use xcm::latest::prelude::*;

    #[benchmark]
    fn set_relay_network() {
        #[extrinsic_call]
        _(RawOrigin::Root, NetworkId::Kusama);
    }

    #[benchmark]
    fn set_asset_link() {
        let location = Location::here();
        let asset_id: T::AssetId = Default::default();

        #[extrinsic_call]
        _(RawOrigin::Root, asset_id, location);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Runtime,);
}
