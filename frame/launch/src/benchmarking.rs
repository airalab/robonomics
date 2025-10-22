///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
// Benchmarks for Launch Pallet

#![cfg(feature = "runtime-benchmarks")]

use super::{Pallet as Launch, *};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::Vec;

const SEED: u32 = 0;

fn setup_param<T: Config>() -> T::Parameter {
    let s = 1024;
    let mut v = Vec::with_capacity(s - 4);
    v.resize(s - 4, 0x1F);
    v.using_encoded(|mut slice| T::Parameter::decode(&mut slice).unwrap_or_default())
}

fn setup_launch<T: Config>(caller: T::AccountId) -> Result<(), &'static str> {
    let data: T::AccountId = account("caller", 1, SEED);
    let param = setup_param::<T>();
    for _ in 0..1000 {
        Launch::<T>::launch(
            RawOrigin::Signed(caller.clone()).into(),
            data.clone(),
            param.clone(),
        )?;
    }
    Ok(())
}

#[benchmarks]
mod benchmarks {
    use super::*;
    #[cfg(test)]
    use frame_system::RawOrigin;

    #[benchmark]
    fn launch() -> Result<(), BenchmarkError> {
        let caller: T::AccountId = account("caller", 1, SEED);
        let data: T::AccountId = account("caller", 1, SEED);
        let param = setup_param::<T>();
        setup_launch::<T>(caller.clone())?;

        #[extrinsic_call]
        launch(RawOrigin::Signed(caller), data, param);

        Ok(())
    }

    impl_benchmark_test_suite!(Launch, crate::tests::new_test_ext(), crate::tests::Runtime,);
}
