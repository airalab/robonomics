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

pub mod economics;
pub mod signed;
pub mod technics;
pub mod traits;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::traits::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// Type synonym for technical trait parameter.
    pub type TechnicalParam<T> = <<T as Config>::Technics as Technical>::Parameter;

    /// Type synonym for technical report trait parameter.
    pub type TechnicalReport<T> = <<T as Config>::Technics as Technical>::Report;

    /// Type synonym for economical trait parameter.
    pub type EconomicalParam<T> = <<T as Config>::Economics as Economical>::Parameter;

    /// Type synonym for liability proof parameter.
    pub type ProofParam<T> = <<T as Config>::Liability as Agreement<
        <T as Config>::Technics,
        <T as Config>::Economics,
    >>::Proof;

    pub type LiabilityIndex<T> = <<T as Config>::Liability as Agreement<
        <T as Config>::Technics,
        <T as Config>::Economics,
    >>::Index;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Technical aspects of agreement.
        type Technics: Technical;

        /// Economical aspects of agreement.
        type Economics: Economical;

        /// How to make and process agreement between two parties.
        type Liability: codec::Codec
            + Processing
            + Agreement<
                Self::Technics,
                Self::Economics,
                AccountId = Self::AccountId,
                Index = Self::Index,
            >;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(
        T::AccountId = "AccountId",
        LiabilityIndex<T> = "LiabilityIndex",
        TechnicalParam<T> = "TechnicalParam",
        EconomicalParam<T> = "EconomicalParam",
        TechnicalReport<T> = "TechnicalReport",
    )]
    pub enum Event<T: Config> {
        /// Yay! New liability created.
        NewLiability(
            LiabilityIndex<T>,
            TechnicalParam<T>,
            EconomicalParam<T>,
            T::AccountId,
            T::AccountId,
        ),

        /// Liability report published.
        NewReport(LiabilityIndex<T>, TechnicalReport<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Promisor agreement proof verification failed
        BadPromisorProof,
        /// Promisee agreement proof verification failed
        BadPromiseeProof,
        /// Promisor report proof verification failed
        BadReportProof,
        /// Unable to decode liability at given index
        LiabilityDecodeFailure,
    }

    #[pallet::storage]
    #[pallet::getter(fn latest_index)]
    /// Latest liability index.
    pub(super) type LatestIndex<T> = StorageValue<_, LiabilityIndex<T>>;

    #[pallet::storage]
    #[pallet::getter(fn liability_of)]
    /// SCALE-encoded liability parameters.
    pub(super) type LiabilityOf<T> = StorageMap<_, Twox64Concat, LiabilityIndex<T>, Vec<u8>>;

    #[pallet::storage]
    #[pallet::getter(fn report_of)]
    /// SCALE-encoded liability report.
    pub(super) type ReportOf<T> = StorageMap<_, Twox64Concat, LiabilityIndex<T>, Vec<u8>>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create agreement between two parties.
        #[pallet::weight(200_000)]
        pub fn create(
            origin: OriginFor<T>,
            technics: TechnicalParam<T>,
            economics: EconomicalParam<T>,
            promisee: T::AccountId,
            promisor: T::AccountId,
            promisee_proof: ProofParam<T>,
            promisor_proof: ProofParam<T>,
        ) -> DispatchResultWithPostInfo {
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
            let latest_index = <LatestIndex<T>>::get().unwrap_or(Default::default());
            liability.using_encoded(|encoded| {
                <LiabilityOf<T>>::insert(latest_index, Vec::from(encoded))
            });
            <LatestIndex<T>>::put(latest_index + 1u32.into());

            // Emit event
            Self::deposit_event(Event::NewLiability(
                latest_index,
                technics,
                economics,
                promisee,
                promisor,
            ));

            Ok(().into())
        }

        /// Publish technical report of complite works.
        #[pallet::weight(200_000)]
        pub fn finalize(
            origin: OriginFor<T>,
            index: LiabilityIndex<T>,
            report: TechnicalReport<T>,
            proof: ProofParam<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            // Is liability already finalized?
            ensure!(<ReportOf<T>>::get(index) == None, "already finalized");

            // Decode liability from storage
            if let Some(encoded) = <LiabilityOf<T>>::get(index) {
                if let Ok(liability) = T::Liability::decode(&mut encoded.as_ref()) {
                    // Check report proof
                    if !liability.check_report(&index, &report, &proof) {
                        return Err(Error::<T>::BadReportProof.into());
                    }

                    // Run economical processing
                    // TODO: get parameter from oracle
                    liability.on_finish(true)?;

                    // Store report as bytestring
                    report.using_encoded(|bytes| <ReportOf<T>>::insert(index, Vec::from(bytes)));

                    // Emit event
                    Self::deposit_event(Event::NewReport(index, report));
                    return Ok(().into());
                }
            }

            Err(Error::<T>::LiabilityDecodeFailure.into())
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
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

                    ValidTransaction::with_tag_prefix("liability")
                        .priority(TransactionPriority::max_value())
                        .and_provides(vec![(technics, economics, promisee, promisor).encode()])
                        .longevity(64u64)
                        .propagate(true)
                        .build()
                }

                Call::finalize(index, report, proof) => {
                    if let Some(encoded) = <LiabilityOf<T>>::get(index) {
                        if let Ok(liability) = T::Liability::decode(&mut encoded.as_ref()) {
                            if !liability.check_report(index, report, proof) {
                                return InvalidTransaction::BadProof.into();
                            } else {
                                return ValidTransaction::with_tag_prefix("liability")
                                    .priority(TransactionPriority::max_value())
                                    .and_provides(vec![(index, report).encode()])
                                    .longevity(64u64)
                                    .propagate(true)
                                    .build();
                            }
                        }
                    }
                    InvalidTransaction::Call.into()
                }

                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::economics::Communism;
    use crate::signed::{ProofSigner, SignedLiability};
    use crate::technics::PureIPFS;
    use crate::traits::*;
    use crate::{self as liability, *};
    use base58::FromBase58;
    use codec::Encode;
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
        type Technics = PureIPFS;
        type Economics = Communism;
        type Liability = SignedLiability<
            Self::Technics,
            Self::Economics,
            MultiSignature,
            <MultiSignature as Verify>::Signer,
            AccountId32,
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
        technics: &TechnicalParam<Runtime>,
        economics: &EconomicalParam<Runtime>,
    ) -> (AccountId32, ProofParam<Runtime>) {
        let pair = sr25519::Pair::from_string(uri, None).unwrap();
        let sender = <MultiSignature as Verify>::Signer::from(pair.public()).into_account();
        let signature = <ProofSigner<sr25519::Pair> as ProofBuilder<
            <Runtime as Config>::Technics,
            <Runtime as Config>::Economics,
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
            <Runtime as Config>::Technics,
            <Runtime as Config>::Economics,
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
        let liability = <Runtime as Config>::Liability::new(
            technics,
            economics,
            sender.clone(),
            sender.clone(),
        );
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
            assert_eq!(Liability::latest_index(), None);

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
            assert_eq!(Liability::latest_index(), None);

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
            assert_eq!(Liability::latest_index(), None);

            assert_ok!(Liability::create(
                Origin::none(),
                technics,
                economics,
                promisee,
                promisor,
                promisee_proof,
                promisor_proof
            ));
            assert_eq!(Liability::latest_index(), Some(1));
            assert_eq!(Liability::report_of(0), None);

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
            assert_eq!(Liability::report_of(0), None);

            assert_ok!(Liability::finalize(
                Origin::none(),
                0,
                report.clone(),
                good_proof
            ));
            assert_eq!(Liability::report_of(0), Some(report.encode()));
        })
    }
}
