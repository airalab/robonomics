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
//! The Robonomics runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod economics;
pub mod signed;
pub mod technics;
pub mod traits;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::traits::*;
    use frame_support::{dispatch, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// How to make and process agreement between two parties.
        type Agreement: dispatch::Parameter + Processing + Agreement<Self::AccountId>;

        /// How to report of agreement execution.
        type Report: dispatch::Parameter + Report<Self::Index, Self::AccountId>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    pub type TechnicsFor<T> =
        <<T as Config>::Agreement as Agreement<<T as frame_system::Config>::AccountId>>::Technical;
    pub type EconomicsFor<T> =
        <<T as Config>::Agreement as Agreement<<T as frame_system::Config>::AccountId>>::Economical;
    pub type ReportFor<T> = <T as Config>::Report;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(
        T::Index = "LiabilityIndex",
        T::AccountId = "AccountId",
        TechnicsFor<T> = "Technics",
        EconomicsFor<T> = "Economics",
        ReportFor<T> = "Report",
    )]
    pub enum Event<T: Config> {
        /// Yay! New liability created.
        NewLiability(
            T::Index,
            TechnicsFor<T>,
            EconomicsFor<T>,
            T::AccountId,
            T::AccountId,
        ),

        /// Liability report published.
        NewReport(T::Index, ReportFor<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Agreement proof verification failed.
        BadAgreementProof,
        /// Report proof verification failed.
        BadReportProof,
        /// Wrong report sender account.
        BadReportSender,
        /// Liability already finalized.
        AlreadyFinalized,
        /// Real world oracle is not ready for this report.
        OracleIsNotReady,
        /// Unable to load agreement from storage.
        AgreementNotFound,
    }

    #[pallet::storage]
    #[pallet::getter(fn latest_index)]
    /// Latest liability index.
    pub(super) type LatestIndex<T: Config> = StorageValue<_, T::Index>;

    #[pallet::storage]
    #[pallet::getter(fn agreement_of)]
    /// Technical and economical parameters of liability.
    pub(super) type AgreementOf<T: Config> = StorageMap<_, Twox64Concat, T::Index, T::Agreement>;

    #[pallet::storage]
    #[pallet::getter(fn report_of)]
    /// Result of liability execution.
    pub(super) type ReportOf<T: Config> = StorageMap<_, Twox64Concat, T::Index, ReportFor<T>>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create agreement between two parties.
        #[pallet::weight(200_000)]
        pub fn create(origin: OriginFor<T>, agreement: T::Agreement) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            ensure!(agreement.verify(), Error::<T>::BadAgreementProof);

            // Start agreement processing
            agreement.on_start()?;

            // Store agreement on storage
            let latest_index = <LatestIndex<T>>::get().unwrap_or(Default::default());
            <AgreementOf<T>>::insert(latest_index, agreement.clone());
            <LatestIndex<T>>::put(latest_index + 1u32.into());

            // Emit event
            Self::deposit_event(Event::NewLiability(
                latest_index,
                agreement.technical(),
                agreement.economical(),
                agreement.promisee(),
                agreement.promisor(),
            ));

            Ok(().into())
        }

        /// Publish technical report of complite works.
        #[pallet::weight(200_000)]
        pub fn finalize(origin: OriginFor<T>, report: ReportFor<T>) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            // Check report proof
            ensure!(report.verify(), Error::<T>::BadReportProof);

            let index = report.index();
            // Is liability already finalized?
            ensure!(
                <ReportOf<T>>::get(index) == None,
                Error::<T>::AlreadyFinalized
            );

            // Decode agreement from storage
            if let Some(agreement) = <AgreementOf<T>>::get(index) {
                // Check report sender
                ensure!(
                    report.sender() == agreement.promisor(),
                    Error::<T>::BadReportSender
                );

                // Run agreement final processing
                match report.is_confirmed() {
                    None => Err(Error::<T>::OracleIsNotReady)?,
                    Some(x) => agreement.on_finish(x)?,
                }

                // Store report on storage
                <ReportOf<T>>::insert(index, report.clone());

                // Emit event
                Self::deposit_event(Event::NewReport(index, report));
                Ok(().into())
            } else {
                Err(Error::<T>::AgreementNotFound.into())
            }
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;
        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::create(agreement) => ValidTransaction::with_tag_prefix("liability")
                    .priority(TransactionPriority::max_value())
                    .and_provides(vec![(
                        agreement.technical(),
                        agreement.economical(),
                        agreement.promisee(),
                        agreement.promisor(),
                    )
                        .encode()])
                    .longevity(64u64)
                    .propagate(true)
                    .build(),
                Call::finalize(report) => ValidTransaction::with_tag_prefix("liability")
                    .priority(TransactionPriority::max_value())
                    .and_provides(vec![(report.index(), report.sender()).encode()])
                    .longevity(64u64)
                    .propagate(true)
                    .build(),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::signed::*;
    use crate::technics::IPFS;
    use crate::traits::*;
    use crate::{self as liability, *};
    use base58::FromBase58;
    use frame_support::{assert_err, assert_ok, parameter_types};
    use sp_core::{crypto::Pair, sr25519, H256};
    use sp_runtime::{
        testing::Header,
        traits::{IdentifyAccount, IdentityLookup, Verify},
        AccountId32, MultiSignature,
    };

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
    type Block = frame_system::mocking::MockBlock<Runtime>;

    frame_support::construct_runtime!(
        pub enum Runtime where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Module, Call, Config, Storage, Event<T>},
            Liability: liability::{Module, Call, Storage, Event<T>, ValidateUnsigned},
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    impl frame_system::Config for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Call = Call;
        type Hash = H256;
        type Hashing = ::sp_runtime::traits::BlakeTwo256;
        type AccountId = AccountId32;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = Event;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BaseCallFilter = ();
        type SystemWeightInfo = ();
        type BlockWeights = ();
        type BlockLength = ();
        type SS58Prefix = ();
    }

    impl Config for Runtime {
        type Event = Event;
        type Agreement = SignedAgreement<
            // Provide task in IPFS
            IPFS,
            // No payments
            (),
            // Use standard accounts
            Self::AccountId,
            // Use standard signatures
            MultiSignature,
        >;
        type Report = SignedReport<
            // Indexing liabilities using system index
            Self::Index,
            // Use standard accounts
            Self::AccountId,
            // Use standard signatures
            MultiSignature,
            // Provide report in IPFS
            IPFS,
        >;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    #[test]
    fn test_initial_setup() {
        new_test_ext().execute_with(|| {
            assert_eq!(Liability::latest_index(), None);
        });
    }

    fn get_params_proof(
        uri: &str,
        technics: &TechnicsFor<Runtime>,
        economics: &EconomicsFor<Runtime>,
    ) -> (AccountId32, MultiSignature) {
        let pair = sr25519::Pair::from_string(uri, None).unwrap();
        let sender = <MultiSignature as Verify>::Signer::from(pair.public()).into_account();
        let signature = <ProofSigner<_> as AgreementProofBuilder<_, _, _, _>>::proof(
            technics, economics, &pair,
        )
        .into();
        (sender, signature)
    }

    fn get_report_proof(uri: &str, index: &u64, message: &IPFS) -> (AccountId32, MultiSignature) {
        let pair = sr25519::Pair::from_string(uri, None).unwrap();
        let sender = <MultiSignature as Verify>::Signer::from(pair.public()).into_account();
        let signature =
            <ProofSigner<_> as ReportProofBuilder<_, _, _, _>>::proof(index, message, &pair).into();
        (sender, signature)
    }

    #[test]
    fn test_liability_proofs() {
        let technics = IPFS {
            hash: "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
                .from_base58()
                .unwrap(),
        };
        let economics = ();
        let (sender, signature) = get_params_proof("//Alice", &technics, &economics);
        let agreement: <Runtime as Config>::Agreement = SignedAgreement {
            technics,
            economics,
            promisee: sender.clone(),
            promisor: sender.clone(),
            promisee_signature: signature.clone(),
            promisor_signature: signature.clone(),
        };
        assert_eq!(agreement.verify(), true);

        let index = 1;
        let payload = IPFS {
            hash: "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
                .from_base58()
                .unwrap(),
        };
        let (sender, signature) = get_report_proof("//Alice", &index, &payload);
        let report = SignedReport {
            index,
            sender,
            payload,
            signature,
        };
        assert_eq!(report.verify(), true);
    }

    #[test]
    fn test_liability_lifecycle() {
        new_test_ext().execute_with(|| {
            assert_eq!(Liability::latest_index(), None);

            let technics = IPFS {
                hash: "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
                    .from_base58()
                    .unwrap(),
            };
            let economics = ();

            let (promisee, promisee_signature) = get_params_proof("//Alice", &technics, &economics);
            let (promisor, promisor_signature) = get_params_proof("//Bob", &technics, &economics);
            let agreement = SignedAgreement {
                technics,
                economics,
                promisee,
                promisor,
                promisee_signature: Default::default(),
                promisor_signature,
            };

            assert_err!(
                Liability::create(Origin::none(), agreement.clone()),
                Error::<Runtime>::BadAgreementProof,
            );
            assert_eq!(Liability::latest_index(), None);

            let agreement = SignedAgreement {
                promisee_signature,
                ..agreement
            };
            assert_ok!(Liability::create(Origin::none(), agreement.clone()),);
            assert_eq!(Liability::latest_index(), Some(1));
            assert_eq!(Liability::report_of(0), None);
            assert_eq!(Liability::agreement_of(0), Some(agreement));

            let index = 0;
            let payload = IPFS {
                hash: "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
                    .from_base58()
                    .unwrap(),
            };
            let (_, bad_signature) = get_report_proof("//Alice", &index, &payload);
            let (sender, signature) = get_report_proof("//Bob", &index, &payload);

            let report = SignedReport {
                index,
                sender,
                payload,
                signature: bad_signature,
            };
            assert_err!(
                Liability::finalize(Origin::none(), report.clone()),
                Error::<Runtime>::BadReportProof,
            );
            assert_eq!(Liability::report_of(0), None);

            let report = SignedReport {
                signature,
                ..report.clone()
            };
            assert_ok!(Liability::finalize(Origin::none(), report.clone()));
            assert_eq!(Liability::report_of(0), Some(report));
        })
    }
}
