///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
// Benchmarks for Datalog Pallet

use super::{Pallet as Datalog, *};
use codec::{Decode, Encode};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::pallet_prelude::Get;
use frame_system::RawOrigin;
use sp_std::prelude::*;

fn setup_record<T: Config>() -> T::Record {
    let s = T::MaximumMessageSize::get();
    let mut v = Vec::with_capacity(s - 4);
    v.resize(s - 4, 0x1F);

    v.using_encoded(|mut slice| T::Record::decode(&mut slice).unwrap_or_default())
}

fn setup_datalog<T: Config>(caller: T::AccountId, data: T::Record) -> Result<(), &'static str>
where
    T::Record: Clone,
{
    for _ in 0..T::WindowSize::get() {
        Datalog::<T>::record(RawOrigin::Signed(caller.clone()).into(), data.clone())?;
    }
    Ok(())
}

const SEED: u32 = 0;

benchmarks! {

    record {
        let caller: T::AccountId =  account("caller", 1, SEED );
        let data = setup_record::<T>();
        setup_datalog::<T>( caller.clone(), data.clone() )?;
    }: _( RawOrigin::Signed(caller), data )

    erase {
        let caller : T::AccountId=  account("caller", 1, SEED);
        let data = setup_record::<T>();
        setup_datalog::<T>( caller.clone(), data )?;
    }: _( RawOrigin::Signed(caller) )
}

impl_benchmark_test_suite!(Datalog, crate::tests::new_test_ext(), crate::tests::Runtime,);
