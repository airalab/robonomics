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
//! Simple Robonomics datalog runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_support::traits::Time;
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Current time source.
        type Time: Time;
        /// Datalog record data type.
        type Record: Parameter + From<Vec<u8>>;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", T::Record = "Record", <T::Time as Time>::Moment = "Moment")]
    pub enum Event<T: Config> {
        /// New data added.
        NewRecord(T::AccountId, <T::Time as Time>::Moment, T::Record),
        /// Account datalog erased.
        Erased(T::AccountId),
        /// Record sended to another location.
        RecordSent(T::AccountId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn datalog)]
    /// Time tagged data of given account.
    pub(super) type Datalog<T> = StorageMap<
        _,
        Twox64Concat,
        <T as frame_system::Config>::AccountId,
        Vec<(<<T as Config>::Time as Time>::Moment, <T as Config>::Record)>,
    >;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Store new data into blockchain.
        #[pallet::weight(500_000)]
        pub fn record(origin: OriginFor<T>, record: T::Record) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let now = T::Time::now();
            <Datalog<T>>::mutate(sender.clone(), |m| match m {
                None => *m = Some(vec![(now.clone(), record.clone())]),
                Some(v) => v.push((now.clone(), record.clone())),
            });
            Self::deposit_event(Event::NewRecord(sender, now, record));
            Ok(().into())
        }

        /// Clear account datalog.
        #[pallet::weight(100_000)]
        pub fn erase(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            <Datalog<T>>::remove(sender.clone());
            Self::deposit_event(Event::Erased(sender));
            Ok(().into())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as datalog, *};

    use base58::FromBase58;
    use frame_support::{assert_err, assert_ok, parameter_types};
    use sp_core::H256;
    use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchError};

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
    type Block = frame_system::mocking::MockBlock<Runtime>;

    frame_support::construct_runtime!(
        pub enum Runtime where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Module, Call, Config, Storage, Event<T>},
            Timestamp: pallet_timestamp::{Module, Storage},
            Datalog: datalog::{Module, Call, Storage, Event<T>},
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
        type Hashing = sp_runtime::traits::BlakeTwo256;
        type AccountId = u64;
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

    impl pallet_timestamp::Config for Runtime {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = ();
        type WeightInfo = ();
    }

    impl Config for Runtime {
        type Time = Timestamp;
        type Record = Vec<u8>;
        type Event = Event;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    #[test]
    fn test_store_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = vec![42];
            assert_ok!(Datalog::record(Origin::signed(sender), record.clone()));
            assert_eq!(Datalog::datalog(sender), Some(vec![(0, record)]));
        })
    }

    #[test]
    fn test_erase_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = vec![1, 2, 3];
            assert_ok!(Datalog::record(Origin::signed(sender), record.clone()));
            assert_eq!(Datalog::datalog(sender), Some(vec![(0, record)]));
            assert_ok!(Datalog::erase(Origin::signed(sender)));
            assert_eq!(Datalog::datalog(sender), None);
        })
    }

    #[test]
    fn test_bad_origin() {
        new_test_ext().execute_with(|| {
            assert_err!(
                Datalog::record(Origin::none(), vec![]),
                DispatchError::BadOrigin
            );
        })
    }

    #[test]
    fn test_store_ipfs_hashes() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4"
                .from_base58()
                .unwrap();
            assert_ok!(Datalog::record(Origin::signed(sender), record.clone()));
            assert_eq!(Datalog::datalog(sender), Some(vec![(0, record.clone())]));
            let record2 = "zdj7WWYAEceQ6ncfPZeRFjozov4dC7FaxU7SuMwzW4VuYBDta"
                .from_base58()
                .unwrap();
            assert_ok!(Datalog::record(Origin::signed(sender), record2.clone()));
            assert_eq!(
                Datalog::datalog(sender),
                Some(vec![(0, record.clone()), (0, record2.clone())])
            );
            let record3 = "QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz2"
                .from_base58()
                .unwrap();
            assert_ok!(Datalog::record(Origin::signed(sender), record3.clone()));
            assert_eq!(
                Datalog::datalog(sender),
                Some(vec![(0, record), (0, record2), (0, record3)])
            );
        })
    }
}

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking {
    use crate::*;
    use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
    use frame_system::RawOrigin;
    use sp_std::boxed::Box;
    use sp_std::vec;
    use sp_std::vec::Vec;

    benchmarks! {
        datalog_record {
            let test_string =
                // IPFS hash of word "test"
                "QmeomffUNfmQy76CQGy9NdmqEnnHU9soCexBnGU3ezPHVH"
                .to_string()
                .as_bytes()
                .to_vec();
            let caller = account("caller", 0, 0);
            }: record(RawOrigin::Signed(caller), test_string.into())
    }
}
