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
//! The Robonomics runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod economics;
pub mod signed;
pub mod technics;
pub mod traits;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

pub use pallet::*;
pub use signed::*;
pub use traits::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::traits::*;
    use super::*;
    use frame_support::{dispatch, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;

    /// Agreement indexing parameter.
    pub type Index = u32;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// How to make and process agreement between two parties.
        type Agreement: dispatch::Parameter
            + Processing
            + Agreement<Self::AccountId>
            + MaxEncodedLen;

        /// How to report of agreement execution.
        type Report: dispatch::Parameter + Report<Index, Self::AccountId> + MaxEncodedLen;

        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Extrinsic weights
        type WeightInfo: WeightInfo;
    }

    pub type TechnicsFor<T> =
        <<T as Config>::Agreement as Agreement<<T as frame_system::Config>::AccountId>>::Technical;
    pub type EconomicsFor<T> =
        <<T as Config>::Agreement as Agreement<<T as frame_system::Config>::AccountId>>::Economical;
    pub type ReportFor<T> = <T as Config>::Report;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Yay! New liability created.
        NewLiability(
            Index,
            TechnicsFor<T>,
            EconomicsFor<T>,
            T::AccountId,
            T::AccountId,
        ),

        /// Liability report published.
        NewReport(Index, ReportFor<T>),
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
    #[pallet::getter(fn next_index)]
    /// Next liability index.
    pub(super) type NextIndex<T: Config> = StorageValue<_, Index, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn agreement_of)]
    /// Technical and economical parameters of liability.
    pub(super) type AgreementOf<T: Config> = StorageMap<_, Twox64Concat, Index, T::Agreement>;

    #[pallet::storage]
    #[pallet::getter(fn report_of)]
    /// Result of liability execution.
    pub(super) type ReportOf<T: Config> = StorageMap<_, Twox64Concat, Index, ReportFor<T>>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create agreement between two parties.
        #[pallet::weight(T::WeightInfo::create())]
        #[pallet::call_index(0)]
        pub fn create(origin: OriginFor<T>, agreement: T::Agreement) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            ensure!(agreement.verify(), Error::<T>::BadAgreementProof);

            // Start agreement processing
            agreement.on_start()?;

            // Store agreement on storage
            let next_index = <NextIndex<T>>::get();
            <AgreementOf<T>>::insert(next_index, agreement.clone());
            <NextIndex<T>>::put(next_index + 1u32);

            // Emit event
            Self::deposit_event(Event::NewLiability(
                next_index,
                agreement.technical(),
                agreement.economical(),
                agreement.promisee(),
                agreement.promisor(),
            ));

            Ok(().into())
        }

        /// Publish technical report of complite works.
        #[pallet::weight(T::WeightInfo::finalize())]
        #[pallet::call_index(1)]
        pub fn finalize(origin: OriginFor<T>, report: ReportFor<T>) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

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
}

#[cfg(test)]
mod tests {
    use crate::economics::SimpleMarket;
    use crate::signed::*;
    use crate::technics::IPFS;
    use crate::traits::*;
    use crate::{self as liability, *};
    use frame_support::{assert_err, assert_ok, parameter_types};
    use hex_literal::hex;
    use sp_core::{crypto::Pair, sr25519, H256};
    use sp_keyring::AccountKeyring;
    use sp_runtime::{
        traits::{IdentifyAccount, IdentityLookup, Verify},
        AccountId32, BuildStorage, MultiSignature,
    };

    type Block = frame_system::mocking::MockBlock<Runtime>;
    type Balance = u128;

    const XRT: Balance = 1_000_000_000;

    frame_support::construct_runtime!(
        pub enum Runtime {
            System: frame_system,
            Balances: pallet_balances,
            Liability: liability,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    impl frame_system::Config for Runtime {
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
    }

    parameter_types! {
        pub const MaxLocks: u32 = 50;
        pub const MaxReserves: u32 = 50;
        pub const ExistentialDeposit: Balance = 10;
    }

    impl pallet_balances::Config for Runtime {
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
        type MaxHolds = ();
    }

    impl Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type Agreement = SignedAgreement<
            // Provide task in IPFS
            IPFS,
            // Liability has a price
            SimpleMarket<Self::AccountId, Balances>,
            // Use standard accounts
            Self::AccountId,
            // Use standard signatures
            MultiSignature,
        >;
        type Report = SignedReport<
            // Indexing liabilities
            Index,
            // Use standard accounts
            Self::AccountId,
            // Use standard signatures
            MultiSignature,
            // Provide report in IPFS
            IPFS,
        >;
    }

    // IPFS raw hash (sha256)
    const IPFS_HASH: [u8; 32] =
        hex!["30f3d649b3d140a6601e11a2cfbe3560e60dc5434f62d702ac8ceff4e1890015"];

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = RuntimeGenesisConfig {
            system: Default::default(),
            balances: pallet_balances::GenesisConfig::<Runtime> {
                balances: vec![
                    (AccountKeyring::Alice.into(), 100 * XRT),
                    (AccountKeyring::Bob.into(), 100 * XRT),
                ],
            },
        }
        .build_storage()
        .unwrap();
        storage.into()
    }

    #[test]
    fn test_initial_setup() {
        new_test_ext().execute_with(|| {
            assert_eq!(Liability::next_index(), 0);
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

    fn get_report_proof(uri: &str, index: &u32, message: &IPFS) -> (AccountId32, MultiSignature) {
        let pair = sr25519::Pair::from_string(uri, None).unwrap();
        let sender = <MultiSignature as Verify>::Signer::from(pair.public()).into_account();
        let signature =
            <ProofSigner<_> as ReportProofBuilder<_, _, _, _>>::proof(index, message, &pair).into();
        (sender, signature)
    }

    #[test]
    fn test_liability_proofs() {
        let technics = IPFS {
            hash: IPFS_HASH.into(),
        };
        let economics = SimpleMarket { price: 10 };
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
            hash: IPFS_HASH.into(),
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
            assert_eq!(Liability::next_index(), 0);

            let technics = IPFS {
                hash: IPFS_HASH.into(),
            };
            let economics = SimpleMarket { price: 10 * XRT };

            let (alice, promisee_signature) = get_params_proof("//Alice", &technics, &economics);
            let (bob, promisor_signature) = get_params_proof("//Bob", &technics, &economics);

            assert_eq!(System::account(&alice).data.free, 100 * XRT);
            assert_eq!(System::account(&bob).data.free, 100 * XRT);

            let agreement = SignedAgreement {
                technics,
                economics,
                promisee: alice.clone(),
                promisor: bob.clone(),
                promisee_signature: promisor_signature.clone(),
                promisor_signature,
            };

            assert_err!(
                Liability::create(
                    RuntimeOrigin::signed(agreement.promisor.clone()),
                    agreement.clone()
                ),
                Error::<Runtime>::BadAgreementProof,
            );
            assert_eq!(Liability::next_index(), 0);
            assert_eq!(System::account(&alice).data.free, 100 * XRT);
            assert_eq!(System::account(&bob).data.free, 100 * XRT);

            let agreement = SignedAgreement {
                promisee_signature,
                ..agreement
            };
            assert_ok!(Liability::create(
                RuntimeOrigin::signed(agreement.promisor.clone()),
                agreement.clone()
            ),);
            assert_eq!(Liability::next_index(), 1);
            assert_eq!(Liability::report_of(0), None);
            assert_eq!(Liability::agreement_of(0), Some(agreement));
            assert_eq!(System::account(&alice).data.free, 90 * XRT);
            assert_eq!(System::account(&bob).data.free, 100 * XRT);

            let index = 0u32;
            let payload = IPFS {
                hash: IPFS_HASH.into(),
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
                Liability::finalize(RuntimeOrigin::signed(report.sender.clone()), report.clone()),
                Error::<Runtime>::BadReportProof,
            );
            assert_eq!(Liability::report_of(0), None);
            assert_eq!(System::account(&alice).data.free, 90 * XRT);
            assert_eq!(System::account(&bob).data.free, 100 * XRT);

            let report = SignedReport {
                signature,
                ..report.clone()
            };
            assert_ok!(Liability::finalize(
                RuntimeOrigin::signed(report.sender.clone()),
                report.clone()
            ));
            assert_eq!(Liability::report_of(0), Some(report));
            assert_eq!(System::account(&alice).data.free, 90 * XRT);
            assert_eq!(System::account(&bob).data.free, 110 * XRT);
        })
    }
}
