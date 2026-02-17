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
// Benchmarks for Liability Pallet

#![cfg(feature = "runtime-benchmarks")]

use super::{Pallet as Liability, *};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use hex_literal::hex;
use sp_std::prelude::*;

// IPFS raw hash (sha256)
const IPFS_HASH: [u8; 32] =
    hex!["30f3d649b3d140a6601e11a2cfbe3560e60dc5434f62d702ac8ceff4e1890015"];

// Define a test runtime for benchmarking
#[cfg(feature = "runtime-benchmarks")]
mod benchmark_runtime {
    use super::*;
    use crate::{self as liability};
    use frame_support::parameter_types;
    use sp_core::H256;
    use sp_runtime::{generic, traits::IdentityLookup, AccountId32};

    pub type Block = generic::Block<
        generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
        sp_runtime::generic::UncheckedExtrinsic<
            sp_runtime::MultiAddress<AccountId32, ()>,
            RuntimeCall,
            sp_runtime::MultiSignature,
            (),
        >,
    >;

    type Balance = u128;

    frame_support::construct_runtime!(
        pub enum BenchmarkRuntime {
            System: frame_system,
            Balances: pallet_balances,
            Liability: liability,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u32 = 250;
    }

    impl frame_system::Config for BenchmarkRuntime {
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Nonce = u32;
        type Block = Block;
        type Hash = H256;
        type Hashing = ::sp_runtime::traits::BlakeTwo256;
        type AccountId = AccountId32;
        type Lookup = IdentityLookup<Self::AccountId>;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<Balance>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BaseCallFilter = frame_support::traits::Everything;
        type SystemWeightInfo = ();
        type BlockWeights = ();
        type BlockLength = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = frame_support::traits::ConstU32<16>;
        type RuntimeTask = RuntimeTask;
        type ExtensionsWeightInfo = ();
        type SingleBlockMigrations = ();
        type MultiBlockMigrator = ();
        type PreInherents = ();
        type PostInherents = ();
        type PostTransactions = ();
    }

    parameter_types! {
        pub const MaxLocks: u32 = 50;
        pub const MaxReserves: u32 = 50;
        pub const ExistentialDeposit: Balance = 10;
    }

    impl pallet_balances::Config for BenchmarkRuntime {
        type MaxLocks = MaxLocks;
        type MaxReserves = MaxReserves;
        type ReserveIdentifier = [u8; 8];
        type Balance = Balance;
        type RuntimeEvent = RuntimeEvent;
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
        type FreezeIdentifier = ();
        type MaxFreezes = ();
        type RuntimeHoldReason = ();
        type RuntimeFreezeReason = RuntimeFreezeReason;
        type DoneSlashHandler = ();
    }

    impl crate::Config for BenchmarkRuntime {
        type RuntimeEvent = RuntimeEvent;
        type Agreement = crate::signed::SignedAgreement<
            crate::technics::IPFS,
            crate::economics::SimpleMarket<Self::AccountId, Balances>,
            Self::AccountId,
            sp_runtime::MultiSignature,
        >;
        type Report = crate::signed::SignedReport<
            Self::Nonce,
            Self::AccountId,
            sp_runtime::MultiSignature,
            crate::technics::IPFS,
        >;
        type WeightInfo = weights::TestWeightInfo;
    }
}

#[benchmarks]
mod benchmarks {
    use super::*;
    use crate::economics::SimpleMarket;
    use crate::signed::{SignedAgreement, SignedReport};
    use crate::technics::IPFS;
    use frame_support::traits::Currency;
    use parity_scale_codec::{Decode, Encode};
    use sp_core::sr25519;
    use sp_runtime::{AccountId32, MultiSignature};

    fn dummy_signature() -> MultiSignature {
        MultiSignature::Sr25519(sr25519::Signature::from_raw([0u8; 64]))
    }

    fn create_test_agreement() -> SignedAgreement<
        IPFS,
        SimpleMarket<AccountId32, benchmark_runtime::Balances>,
        AccountId32,
        MultiSignature,
    > {
        let technics = IPFS {
            hash: IPFS_HASH.into(),
        };
        let economics = SimpleMarket::<AccountId32, benchmark_runtime::Balances> { price: 10 };
        let promisee: AccountId32 = account("alice", 0, 0);
        let promisor: AccountId32 = account("bob", 0, 0);
        let _ = benchmark_runtime::Balances::make_free_balance_be(&promisee, 10000u128);
        let _ = benchmark_runtime::Balances::make_free_balance_be(&promisor, 10000u128);
        let signature = dummy_signature();

        SignedAgreement {
            technics,
            economics,
            promisee,
            promisor,
            promisee_signature: signature.clone(),
            promisor_signature: signature,
        }
    }

    fn create_test_report() -> SignedReport<u32, AccountId32, MultiSignature, IPFS> {
        SignedReport {
            index: 0,
            sender: account("bob", 0, 0),
            payload: IPFS {
                hash: IPFS_HASH.into(),
            },
            signature: dummy_signature(),
        }
    }

    #[benchmark]
    fn create() {
        let caller: T::AccountId = whitelisted_caller();
        let signed_agreement = create_test_agreement();

        let agreement = T::Agreement::decode(&mut &signed_agreement.encode()[..])
            .expect("Failed to decode agreement for benchmarking");

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), agreement);
    }

    #[benchmark]
    fn finalize() {
        let caller: T::AccountId = whitelisted_caller();

        // Create a liability first
        let agreement = create_test_agreement();
        let agreement_typed =
            T::Agreement::decode(&mut &agreement.encode()[..]).expect("Failed to decode agreement");
        let _ = Liability::<T>::create(RawOrigin::Signed(caller.clone()).into(), agreement_typed);

        // Create report to finalize the liability
        let signed_report = create_test_report();
        let report = ReportFor::<T>::decode(&mut &signed_report.encode()[..])
            .expect("Failed to decode report");

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), report);
    }

    impl_benchmark_test_suite!(
        Liability,
        crate::tests::new_test_ext(),
        crate::tests::Runtime,
    );
}
