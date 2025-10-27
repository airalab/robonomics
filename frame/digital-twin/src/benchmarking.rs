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
// Benchmarks for Digital Twin Pallet

#![cfg(feature = "runtime-benchmarks")]

use super::{Pallet as DigitalTwin, *};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_core::H256;
use sp_std::prelude::*;

const SEED: u32 = 0;

#[benchmarks]
mod benchmarks {
    use super::*;
    #[cfg(test)]
    use frame_system::RawOrigin;

    #[benchmark]
    fn create() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        DigitalTwin::<T>::create(RawOrigin::Signed(caller.clone()).into())?;

        #[extrinsic_call]
        create(RawOrigin::Signed(caller));

        Ok(())
    }

    #[benchmark]
    fn set_source() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let id: u32 = 0;
        let topic: H256 = Default::default();
        let source: T::AccountId = account("source", 2, SEED);
        DigitalTwin::<T>::create(RawOrigin::Signed(caller.clone()).into())?;
        DigitalTwin::<T>::set_source(
            RawOrigin::Signed(caller.clone()).into(),
            id,
            topic,
            source.clone(),
        )?;

        #[extrinsic_call]
        set_source(RawOrigin::Signed(caller), id, topic, source);

        Ok(())
    }

    // #[benchmark]
    // fn remove_source() -> Result<(), BenchmarkError> {
    //     let caller: T::AccountId = account("caller", 1, SEED);
    //     let id: u32 = 0;
    //     let topic: H256 = Default::default();
    //     let source: T::AccountId = account("source", 2, SEED);
    //     DigitalTwin::<T>::create(RawOrigin::Signed(caller.clone()).into())?;
    //     DigitalTwin::<T>::set_source(
    //         RawOrigin::Signed(caller.clone()).into(),
    //         id,
    //         topic,
    //         source.clone(),
    //     )?;
    //     DigitalTwin::<T>::remove_source(
    //         RawOrigin::Signed(caller.clone()).into(),
    //         id,
    //         topic,
    //         source.clone(),
    //     )?;
    //
    //     #[extrinsic_call]
    //     remove_source(RawOrigin::Signed(caller), id, topic, source);
    //
    //     Ok(())
    // }

    impl_benchmark_test_suite!(
        DigitalTwin,
        crate::tests::new_test_ext(),
        crate::tests::Runtime,
    );
}
