///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2023 Robonomics Network <research@robonomics.network>
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

use super::{Pallet as Launch, *};
use frame_benchmarking::Vec;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
pub use pallet::*;
use parity_scale_codec::{Decode, Encode};

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

benchmarks! {

    launch {
        let caller: T::AccountId =  account("caller", 1, SEED );
        let data: T::AccountId =  account("caller", 1, SEED );
        let param = setup_param::<T>();
        setup_launch::<T>( caller.clone() )?;
    }: _( RawOrigin::Signed(caller), data, param)

    verify {
    }
}

impl_benchmark_test_suite!(Launch, crate::tests::new_test_ext(), crate::tests::Runtime,);
