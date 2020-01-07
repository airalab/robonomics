///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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

use sp_std::prelude::*;
use codec::{Encode, Decode};
use frame_system::{self as system, ensure_none};
use frame_support::{
    ensure, decl_module, decl_storage, decl_event, decl_error, StorageValue,
};
use sp_runtime::{
    transaction_validity::{
        TransactionValidity, ValidTransaction, InvalidTransaction, TransactionPriority,
    },
};

/// Import module traits.
pub mod traits;
use traits::*;

pub mod signed;
pub mod technics;
pub mod economics;

/// Type synonym for technical trait parameter.
pub type TechnicalParam<T> = <<T as Trait>::Technics as Technical>::Parameter;

/// Type synonym for technical report trait parameter.
pub type TechnicalReport<T> = <<T as Trait>::Technics as Technical>::Report;

/// Type synonym for economical trait parameter.
pub type EconomicalParam<T> = <<T as Trait>::Economics as Economical>::Parameter;

/// Type synonym for liability proof parameter.
pub type ProofParam<T> =
    <<T as Trait>::Liability as Agreement<
        <T as Trait>::Technics,
        <T as Trait>::Economics,
        AccountId<T>
    >>::Proof;

/// Current runtime account identificator.
pub type AccountId<T> = <T as frame_system::Trait>::AccountId;

/// Indexing up to 2^64 liabilities
pub type LiabilityIndex = u64;

/// Liability module main trait.
pub trait Trait: frame_system::Trait {
    /// Technical aspects of agreement.
    type Technics: Technical;

    /// Economical aspects of agreement.
    type Economics: Economical;

    /// How to make and process agreement between two parties. 
    type Liability: Processing + Agreement<Self::Technics, Self::Economics, AccountId<Self>>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event! {
    pub enum Event<T>
    where AccountId = AccountId<T>,
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
        LatestIndex get(fn latest_index): LiabilityIndex;
        /// SCALE-encoded liability parameters.
        LiabilityOf get(fn liability_of): map LiabilityIndex => Vec<u8>;
        /// Set `true` when liability report already send.
        IsFinalized get(fn is_finalized): map LiabilityIndex => bool;
        /// SCALE-encoded liability report.
        ReportOf    get(fn report_of): map LiabilityIndex => Vec<u8>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Create agreement between two parties.
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
            if !liability.verify(ProofTarget::Promisee, &promisee_proof) {
                Err(Error::<T>::BadPromiseeProof)?
            }

            // Check promisor proof
            if !liability.verify(ProofTarget::Promisor, &promisor_proof) {
                Err(Error::<T>::BadPromisorProof)?
            }

            // Run economical processing
            liability.on_start()?;

            // Store liability params as bytestring 
            let latest_index = <LatestIndex>::get();
            liability.using_encoded(|encoded|
                <LiabilityOf>::insert(latest_index, Vec::from(encoded))
            );
            <LatestIndex>::put(latest_index + 1);

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
        fn finalize(
            origin,
            index: u64,
            report: TechnicalReport<T>,
            proof: ProofParam<T>,
        ) {
            ensure_none(origin)?;

            // Is liability already finalized? 
            ensure!(!<IsFinalized>::get(index), "already finalized");

            // Decode liability from storage
            if let Ok(liability) = T::Liability::decode(&mut &<LiabilityOf>::get(index)[..]) {
                // Check report proof
                if !liability.verify(ProofTarget::Report(report.clone()), &proof) {
                    Err(Error::<T>::BadReportProof)?
                }

                // Run economical processing
                // TODO: get parameter from oracle
                liability.on_finish(true)?;

                // Store report as bytestring
                report.using_encoded(|bytes| <ReportOf>::insert(index, Vec::from(bytes)));

                // Set finalized flag
                <IsFinalized>::insert(index, true);

                // Emit event
                Self::deposit_event(RawEvent::NewReport(index, report));
            } else {
                Err(Error::<T>::LiabilityDecodeFailure)?
            }
        }
    }
}

#[allow(deprecated)]
impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
        match call {
            Call::create(technics, economics, promisee, promisor, promisee_proof, promisor_proof) => {
                let liability = T::Liability::new(
                    technics.clone(),
                    economics.clone(),
                    promisee.clone(),
                    promisor.clone(),
                );

                if !liability.verify(ProofTarget::Promisee, promisee_proof) {
                    return InvalidTransaction::BadProof.into();
                }

                if !liability.verify(ProofTarget::Promisor, promisor_proof) {
                    return InvalidTransaction::BadProof.into();
                }

                Ok(ValidTransaction {
                    priority: TransactionPriority::max_value(),
                    requires: vec![],
                    provides: vec![(technics, economics, promisee, promisor).encode()],
                    longevity: 64_u64,
                    propagate: true,
                })
            },

            Call::finalize(index, report, proof) =>
                match T::Liability::decode(&mut &<LiabilityOf>::get(*index)[..]) {
                    Ok(liability) => {
                        if !liability.verify(ProofTarget::Report(report.clone()), proof) {
                            return InvalidTransaction::BadProof.into();
                        }

                        Ok(ValidTransaction {
                            priority: TransactionPriority::max_value(),
                            requires: vec![],
                            provides: vec![(index, report).encode()],
                            longevity: 64_u64,
                            propagate: true,
                        })
                    },
                    _ => InvalidTransaction::Call.into()
                },

            _ => InvalidTransaction::Call.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as liability;
    use super::technics::PureIPFS;
    use super::economics::Communism;
    use super::signed::SignedLiability;
    use sp_runtime::{Perbill, traits::{Verify, IdentifyAccount, IdentityLookup}, testing::Header};
    use node_primitives::{AccountId, Signature};
    use frame_system::{self as system};
    use frame_support::{
        assert_ok, assert_err, impl_outer_event, impl_outer_origin, parameter_types,
        weights::Weight,
    };
    use sp_core::{H256, sr25519, crypto::Pair};
    use base58::FromBase58;

    impl_outer_event! {
        pub enum MetaEvent for Runtime {
            liability<T>,
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
        let pair: sr25519::Pair = Pair::from_string(uri, None).unwrap();
        let order = (technics.clone(), economics.clone());
        let sender = <Signature as Verify>::Signer::from(pair.public()).into_account();
        let signature = order.using_encoded(|params| pair.sign(params)).into();
        (sender, signature)
    }

    fn get_report_proof(uri: &str, report: &TechnicalReport<Runtime>) -> ProofParam<Runtime> {
        let pair: sr25519::Pair = Pair::from_string(uri, None).unwrap();
        report.using_encoded(|params| pair.sign(params)).into()
    }

    #[test]
    fn test_liability_proofs() {
        let technics = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4".from_base58().unwrap();
        let economics = ();
        let (sender, params_proof) = get_params_proof("//Alice", &technics, &economics);
        let liability = <Runtime as Trait>::Liability::new(
            technics,
            economics,
            sender.clone(),
            sender,
        );
        assert_eq!(liability.verify(ProofTarget::Promisor, &params_proof), true);
        assert_eq!(liability.verify(ProofTarget::Promisee, &params_proof), true);

        let report = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4".from_base58().unwrap();
        let report_proof = get_report_proof("//Alice", &report);
        assert_eq!(liability.verify(ProofTarget::Report(report), &report_proof), true);
    }

    #[test]
    fn test_liability_lifecycle() {
        new_test_ext().execute_with(|| {
            assert_eq!(Liability::latest_index(), 0);

            let technics = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4".from_base58().unwrap();
            let economics = ();

            let (promisee, promisee_proof) = get_params_proof("//Alice", &technics, &economics);
            let (promisor, promisor_proof) = get_params_proof("//Bob", &technics, &economics);

            assert_err!(Liability::create(
                Origin::NONE,
                technics.clone(),
                economics.clone(),
                promisee.clone(),
                promisor.clone(),
                promisor_proof.clone(),
                promisor_proof.clone(),
            ), Error::<Runtime>::BadPromiseeProof);
            assert_eq!(Liability::latest_index(), 0);

            assert_err!(Liability::create(
                Origin::NONE,
                technics.clone(),
                economics.clone(),
                promisee.clone(),
                promisor.clone(),
                promisee_proof.clone(),
                promisee_proof.clone(),
            ), Error::<Runtime>::BadPromisorProof);
            assert_eq!(Liability::latest_index(), 0);


            assert_ok!(Liability::create(
                Origin::NONE,
                technics,
                economics,
                promisee,
                promisor,
                promisee_proof,
                promisor_proof
            ));
            assert_eq!(Liability::latest_index(), 1);
            assert_eq!(Liability::is_finalized(0), false);

            let report = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4".from_base58().unwrap();
            let bad_proof = get_report_proof("//Alice", &report);
            let good_proof = get_report_proof("//Bob", &report);

            assert_err!(
                Liability::finalize(Origin::NONE, 0, report.clone(), bad_proof),
                Error::<Runtime>::BadReportProof
            );
            assert_eq!(Liability::is_finalized(0), false);

            assert_ok!(Liability::finalize(Origin::NONE, 0, report, good_proof));
            assert_eq!(Liability::is_finalized(0), true);
        })
    }
}
