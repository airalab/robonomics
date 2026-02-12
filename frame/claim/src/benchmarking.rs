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
//! Benchmarking for the Robonomics Claim pallet.

use super::Call;
#[cfg(feature = "runtime-benchmarks")]
use super::*;
use frame_benchmarking::v2::*;
use frame_support::{
    dispatch::{DispatchInfo, GetDispatchInfo},
    traits::{IsSubType, UnfilteredDispatchable},
};
use frame_system::RawOrigin;
use secp_utils::*;
use sp_runtime::{
    traits::{Dispatchable, ValidateUnsigned},
    DispatchResult,
};

const SEED: u32 = 0;

const MAX_CLAIMS: u32 = 10_000;
const VALUE: u32 = 1_000_000;

fn create_claim<T: Config>(input: u32) -> DispatchResult {
    let secret_key = libsecp256k1::SecretKey::parse(&keccak_256(&input.encode())).unwrap();
    let eth_address = eth(&secret_key);
    super::Pallet::<T>::add_claim(RawOrigin::Root.into(), eth_address, VALUE.into())?;
    let pallet_account = T::PalletId::get().into_account_truncating();
    T::Currency::resolve_creating(&pallet_account, T::Currency::issue(VALUE.into()));
    Ok(())
}

#[benchmarks(
		where
			<T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>> + From<Call<T>>,
			<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo> + GetDispatchInfo,
			<<T as frame_system::Config>::RuntimeCall as Dispatchable>::PostInfo: Default,
	)]
mod benchmarks {
    use super::*;

    // Benchmark `claim` including `validate_unsigned` logic.
    #[benchmark]
    fn claim() -> Result<(), BenchmarkError> {
        let c = MAX_CLAIMS;
        for _ in 0..c / 2 {
            create_claim::<T>(c)?;
        }
        let secret_key = libsecp256k1::SecretKey::parse(&keccak_256(&c.encode())).unwrap();
        let eth_address = eth(&secret_key);
        let account: T::AccountId = account("user", c, SEED);
        let signature = sig::<T>(&secret_key, &account.encode());
        super::Pallet::<T>::add_claim(RawOrigin::Root.into(), eth_address, VALUE.into())?;
        assert_eq!(Claims::<T>::get(eth_address), Some(VALUE.into()));
        let source = sp_runtime::transaction_validity::TransactionSource::External;
        let call_enc = Call::<T>::claim {
            dest: account.clone(),
            ethereum_signature: signature.clone(),
        }
        .encode();

        #[block]
        {
            let call = <Call<T> as Decode>::decode(&mut &*call_enc)
                .expect("call is encoded above, encoding must be correct");
            super::Pallet::<T>::validate_unsigned(source, &call)
                .map_err(|e| -> &'static str { e.into() })?;
            call.dispatch_bypass_filter(RawOrigin::None.into())?;
        }

        assert_eq!(Claims::<T>::get(eth_address), None);
        Ok(())
    }

    // Benchmark `add_claim` when there already exists `c` claims in storage.
    #[benchmark]
    fn add_claim() -> Result<(), BenchmarkError> {
        let c = MAX_CLAIMS;
        for _ in 0..c / 2 {
            create_claim::<T>(c)?;
        }
        let eth_address = account("eth_address", 0, SEED);

        #[extrinsic_call]
        _(RawOrigin::Root, eth_address, VALUE.into());

        assert_eq!(Claims::<T>::get(eth_address), Some(VALUE.into()));
        Ok(())
    }

    // Benchmark the time it takes to do `repeat` number of keccak256 hashes
    #[benchmark(extra)]
    fn keccak256(i: Linear<0, 10_000>) {
        let bytes = (i).encode();

        #[block]
        {
            for _ in 0..i {
                let _hash = keccak_256(&bytes);
            }
        }
    }

    // Benchmark the time it takes to do `repeat` number of `eth_recover`
    #[benchmark(extra)]
    fn eth_recover(i: Linear<0, 1_000>) {
        // Crate signature
        let secret_key = libsecp256k1::SecretKey::parse(&keccak_256(&i.encode())).unwrap();
        let account: T::AccountId = account("user", i, SEED);
        let signature = sig::<T>(&secret_key, &account.encode());
        let data = account.using_encoded(to_ascii_hex);

        #[block]
        {
            for _ in 0..i {
                assert!(super::Pallet::<T>::eth_recover(&signature, &data).is_some());
            }
        }
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test,);
}
