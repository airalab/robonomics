///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life>
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
//! The Robonomics runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, StorageValue};
use frame_system::ensure_none;
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
    ValidTransaction,
};
use sp_std::prelude::*;

/// Import module traits.
pub mod traits;
use traits::*;

pub mod economics;
pub mod signed;
pub mod technics;

/// Type synonym for technical trait parameter.
pub type TechnicalParam<T> = <<T as Trait>::Technics as Technical>::Parameter;

/// Type synonym for technical report trait parameter.
pub type TechnicalReport<T> = <<T as Trait>::Technics as Technical>::Report;

/// Type synonym for economical trait parameter.
pub type EconomicalParam<T> = <<T as Trait>::Economics as Economical>::Parameter;

/// Type synonym for liability proof parameter.
pub type ProofParam<T> =
    <<T as Trait>::Liability as Agreement<<T as Trait>::Technics, <T as Trait>::Economics>>::Proof;

pub type LiabilityIndex<T> =
    <<T as Trait>::Liability as Agreement<<T as Trait>::Technics, <T as Trait>::Economics>>::Index;

/// Current runtime account identificator.
pub type AccountId<T> = <T as frame_system::Trait>::AccountId;

/// Liability module main trait.
pub trait Trait: frame_system::Trait {
    /// Technical aspects of agreement.
    type Technics: Technical;

    /// Economical aspects of agreement.
    type Economics: Economical;

    /// How to make and process agreement between two parties.
    type Liability: Codec
        + Processing
        + Agreement<Self::Technics, Self::Economics, AccountId = AccountId<Self>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event! {
    pub enum Event<T>
    where AccountId = AccountId<T>,
          LiabilityIndex = LiabilityIndex<T>,
          TechnicalParam = TechnicalParam<T>,
          EconomicalParam = EconomicalParam<T>,
          TechnicalReport = TechnicalReport<T>,
    {
        /// Yay! New liability created.
        NewLiability(LiabilityIndex, TechnicalParam, EconomicalParam, AccountId, AccountId),

        /// Liability report published.
        NewReport(LiabilityIndex, TechnicalReport),
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Promisor agreement proof verification failed
        BadPromisorProof,
        /// Promisee agreement proof verification failed
        BadPromiseeProof,
        /// Promisor report proof verification failed
        BadReportProof,
        /// Unable to decode liability at given index
        LiabilityDecodeFailure,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Liability {
        /// Latest liability index.
        LatestIndex get(fn latest_index): LiabilityIndex<T>;
        /// SCALE-encoded liability parameters.
        LiabilityOf get(fn liability_of): map hasher(blake2_128_concat)
                                          LiabilityIndex<T> => Vec<u8>;
        /// Set `true` when liability report already send.
        IsFinalized get(fn is_finalized): map hasher(blake2_128_concat)
                                          LiabilityIndex<T> => bool;
        /// SCALE-encoded liability report.
        ReportOf    get(fn report_of): map hasher(blake2_128_concat)
                                       LiabilityIndex<T> => Vec<u8>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Create agreement between two parties.
        #[weight = 200_000_000]
        fn create(
            origin,
            technics: TechnicalParam<T>,
            economics: EconomicalParam<T>,
            promisee: AccountId<T>,
            promisor: AccountId<T>,
            promisee_proof: ProofParam<T>,
            promisor_proof: ProofParam<T>,
        ) {
            ensure_none(origin)?;

            // Create liability
            let liability = T::Liability::new(
                technics.clone(),
                economics.clone(),
                promisee.clone(),
                promisor.clone(),
            );

            // Check promisee proof
            if !liability.check_params(&promisee_proof, &promisee) {
                Err(Error::<T>::BadPromiseeProof)?
            }

            // Check promisor proof
            if !liability.check_params(&promisor_proof, &promisor) {
                Err(Error::<T>::BadPromisorProof)?
            }

            // Run economical processing
            liability.on_start()?;

            // Store liability params as bytestring
            let latest_index = <LatestIndex<T>>::get();
            liability.using_encoded(|encoded|
                <LiabilityOf<T>>::insert(latest_index, Vec::from(encoded))
            );
            <LatestIndex<T>>::put(latest_index + 1.into());

            // Emit event
            Self::deposit_event(RawEvent::NewLiability(
                latest_index,
                technics,
                economics,
                promisee,
                promisor,
            ));
        }

        /// Publish technical report of complite works.
        #[weight = 200_000_000]
        fn finalize(
            origin,
            index: LiabilityIndex<T>,
            report: TechnicalReport<T>,
            proof: ProofParam<T>,
        ) {
            ensure_none(origin)?;

            // Is liability already finalized?
            ensure!(!<IsFinalized<T>>::get(index), "already finalized");

            // Decode liability from storage
            if let Ok(liability) = T::Liability::decode(&mut &<LiabilityOf<T>>::get(index)[..]) {
                // Check report proof
                if !liability.check_report(&index, &report, &proof) {
                    Err(Error::<T>::BadReportProof)?
                }

                // Run economical processing
                // TODO: get parameter from oracle
                liability.on_finish(true)?;

                // Store report as bytestring
                report.using_encoded(|bytes| <ReportOf<T>>::insert(index, Vec::from(bytes)));

                // Set finalized flag
                <IsFinalized<T>>::insert(index, true);

                // Emit event
                Self::deposit_event(RawEvent::NewReport(index, report));
            } else {
                Err(Error::<T>::LiabilityDecodeFailure)?
            }
        }
    }
}

impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        match call {
            Call::create(
                technics,
                economics,
                promisee,
                promisor,
                promisee_proof,
                promisor_proof,
            ) => {
                let liability = T::Liability::new(
                    technics.clone(),
                    economics.clone(),
                    promisee.clone(),
                    promisor.clone(),
                );

                if !liability.check_params(promisee_proof, promisee) {
                    return InvalidTransaction::BadProof.into();
                }

                if !liability.check_params(promisor_proof, promisor) {
                    return InvalidTransaction::BadProof.into();
                }

                Ok(ValidTransaction {
                    priority: TransactionPriority::max_value(),
                    requires: Default::default(),
                    provides: vec![(technics, economics, promisee, promisor).encode()],
                    longevity: 64_u64,
                    propagate: true,
                })
            }

            Call::finalize(index, report, proof) => {
                match T::Liability::decode(&mut &<LiabilityOf<T>>::get(*index)[..]) {
                    Ok(liability) => {
                        if !liability.check_report(index, report, proof) {
                            return InvalidTransaction::BadProof.into();
                        }

                        Ok(ValidTransaction {
                            priority: TransactionPriority::max_value(),
                            requires: Default::default(),
                            provides: vec![(index, report).encode()],
                            longevity: 64_u64,
                            propagate: true,
                        })
                    }
                    _ => InvalidTransaction::Call.into(),
                }
            }

            _ => InvalidTransaction::Call.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::economics::Communism;
    use super::signed::{ProofSigner, SignedLiability};
    use super::technics::PureIPFS;
    use super::*;
    use crate as liability;
    use base58::FromBase58;
    use frame_support::{
        assert_err, assert_ok, impl_outer_event, impl_outer_origin, parameter_types,
        weights::Weight,
    };
    use frame_system::{self as system};
    use node_primitives::{AccountId, Signature};
    use sp_core::{crypto::Pair, sr25519, H256};
    use sp_runtime::{
        testing::Header,
        traits::{IdentifyAccount, IdentityLookup, Verify},
        Perbill,
    };

    impl_outer_event! {
        pub enum MetaEvent for Runtime {
            system<T>, liability<T>,
        }
    }

    impl_outer_origin! {
        pub enum Origin for Runtime {}
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Runtime;

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: Weight = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::one();
    }

    impl frame_system::Trait for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Call = ();
        type Hash = H256;
        type Hashing = ::sp_runtime::traits::BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = MetaEvent;
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type ModuleToIndex = ();
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BlockExecutionWeight = ();
        type ExtrinsicBaseWeight = ();
        type MaximumExtrinsicWeight = ();
        type BaseCallFilter = ();
        type SystemWeightInfo = ();
    }

    impl Trait for Runtime {
        type Event = MetaEvent;
        type Technics = PureIPFS;
        type Economics = Communism;
        type Liability = SignedLiability<
            Self::Technics,
            Self::Economics,
            Signature,
            <Signature as Verify>::Signer,
            AccountId,
        >;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    type Liability = Module<Runtime>;

    #[test]
    fn test_initial_setup() {
        new_test_ext().execute_with(|| {
            assert_eq!(Liability::latest_index(), 0);
        });
    }

    fn get_params_proof(
        uri: &str,
        technics: &TechnicalParam<Runtime>,
        economics: &EconomicalParam<Runtime>,
    ) -> (AccountId, ProofParam<Runtime>) {
        let pair = sr25519::Pair::from_string(uri, None).unwrap();
        let sender = <Signature as Verify>::Signer::from(pair.public()).into_account();
        let signature = <ProofSigner<sr25519::Pair> as ProofBuilder<
            <Runtime as Trait>::Technics,
            <Runtime as Trait>::Economics,
            LiabilityIndex<Runtime>,
            _,
            _,
        >>::proof_params(technics, economics, pair)
        .into();
        (sender, signature)
    }

    fn get_report_proof(
        uri: &str,
        index: &LiabilityIndex<Runtime>,
        report: &TechnicalReport<Runtime>,
    ) -> ProofParam<Runtime> {
        let pair = sr25519::Pair::from_string(uri, None).unwrap();
        <ProofSigner<sr25519::Pair> as ProofBuilder<
            <Runtime as Trait>::Technics,
            <Runtime as Trait>::Economics,
            LiabilityIndex<Runtime>,
            _,
            _,
        >>::proof_report(index, report, pair)
        .into()
    }

    #[test]
    fn test_liability_proofs() {
        let technics = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
            .from_base58()
            .unwrap();
        let economics = ();
        let (sender, params_proof) = get_params_proof("//Alice", &technics, &economics);
        let liability =
            <Runtime as Trait>::Liability::new(technics, economics, sender.clone(), sender.clone());
        assert_eq!(liability.check_params(&params_proof, &sender), true);

        let index = 1;
        let report = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
            .from_base58()
            .unwrap();
        let report_proof = get_report_proof("//Alice", &index, &report);
        assert_eq!(liability.check_report(&index, &report, &report_proof), true);
    }

    #[test]
    fn test_liability_lifecycle() {
        new_test_ext().execute_with(|| {
            assert_eq!(Liability::latest_index(), 0);

            let technics = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
                .from_base58()
                .unwrap();
            let economics = ();

            let (promisee, promisee_proof) = get_params_proof("//Alice", &technics, &economics);
            let (promisor, promisor_proof) = get_params_proof("//Bob", &technics, &economics);

            assert_err!(
                Liability::create(
                    Origin::none(),
                    technics.clone(),
                    economics.clone(),
                    promisee.clone(),
                    promisor.clone(),
                    promisor_proof.clone(),
                    promisor_proof.clone(),
                ),
                Error::<Runtime>::BadPromiseeProof
            );
            assert_eq!(Liability::latest_index(), 0);

            assert_err!(
                Liability::create(
                    Origin::none(),
                    technics.clone(),
                    economics.clone(),
                    promisee.clone(),
                    promisor.clone(),
                    promisee_proof.clone(),
                    promisee_proof.clone(),
                ),
                Error::<Runtime>::BadPromisorProof
            );
            assert_eq!(Liability::latest_index(), 0);

            assert_ok!(Liability::create(
                Origin::none(),
                technics,
                economics,
                promisee,
                promisor,
                promisee_proof,
                promisor_proof
            ));
            assert_eq!(Liability::latest_index(), 1);
            assert_eq!(Liability::is_finalized(0), false);

            let index = 0;
            let report = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
                .from_base58()
                .unwrap();
            let bad_proof = get_report_proof("//Alice", &index, &report);
            let good_proof = get_report_proof("//Bob", &index, &report);

            assert_err!(
                Liability::finalize(Origin::none(), 0, report.clone(), bad_proof),
                Error::<Runtime>::BadReportProof
            );
            assert_eq!(Liability::is_finalized(0), false);

            assert_ok!(Liability::finalize(Origin::none(), 0, report, good_proof));
            assert_eq!(Liability::is_finalized(0), true);
        })
    }
}
