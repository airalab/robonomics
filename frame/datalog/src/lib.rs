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
//! Simple Robonomics datalog runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]
#![allow(clippy::unused_unit)]
#![allow(clippy::from_over_into)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use frame_support::{pallet_prelude::*, traits::Time};
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::{Decode, Encode};
    use sp_std::prelude::*;

    use super::*;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::config]
    pub trait Config: frame_system::Config + TypeInfo {
        /// Current time source.
        type Time: Time;
        /// Datalog record data type.
        type Record: Parameter + Default + MaxEncodedLen;
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Data log window size
        #[pallet::constant]
        type WindowSize: Get<u64>;
        /// Extrinsic weights
        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Data exceeds size limit
        RecordTooBig,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New data added.
        NewRecord(T::AccountId, <T::Time as Time>::Moment, T::Record),
        /// Account datalog erased.
        Erased(T::AccountId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    /// Ringbuffer start/end pointers
    #[pallet::storage]
    #[pallet::getter(fn datalog_index)]
    pub type DatalogIndex<T> = StorageMap<
        _,
        Twox64Concat,
        <T as frame_system::Config>::AccountId,
        RingBufferIndex,
        ValueQuery,
    >;

    /// Ringbuffer items
    #[pallet::storage]
    #[pallet::getter(fn datalog_item)]
    pub type DatalogItem<T> = StorageMap<
        _,
        Twox64Concat,
        (<T as frame_system::Config>::AccountId, u64),
        RingBufferItem<T>,
        ValueQuery,
    >;

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Store new data into blockchain.
        #[pallet::weight(T::WeightInfo::record())]
        #[pallet::call_index(0)]
        pub fn record(origin: OriginFor<T>, record: T::Record) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            // remove previous version from storage
            let now = T::Time::now();
            let item = RingBufferItem(now, record);

            DatalogIndex::<T>::mutate(&sender, |idx| {
                let window_size = T::WindowSize::get();
                let end = idx.add(window_size);

                DatalogItem::<T>::insert((&sender, end), &item)
            });

            let (now, record) = item.split();

            Self::deposit_event(Event::NewRecord(sender, now, record));
            Ok(().into())
        }

        /// Clear account datalog.
        #[pallet::weight(T::WeightInfo::erase())]
        #[pallet::call_index(1)]
        pub fn erase(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            let mut idx = DatalogIndex::<T>::take(&sender);

            let window_size = T::WindowSize::get();
            // get the number of items in the ring buffer
            let _count = idx.count(window_size);

            for start in idx.iter(window_size) {
                DatalogItem::<T>::remove((&sender, start))
            }

            Self::deposit_event(Event::Erased(sender));
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get account datalog as an ordered array
        pub fn data(account: &<T as frame_system::Config>::AccountId) -> Vec<RingBufferItem<T>> {
            let mut idx = DatalogIndex::<T>::get(&account);
            let window_size = T::WindowSize::get();

            idx.iter(window_size)
                .map(|i| DatalogItem::<T>::get((&account, i)))
                .collect()
        }
    }

    #[cfg_attr(feature = "std", derive(Debug, PartialEq))]
    #[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
    pub struct RingBufferItem<T: Config>(
        #[codec(compact)] <<T as Config>::Time as Time>::Moment,
        <T as Config>::Record,
    );

    impl<T: Config> Default for RingBufferItem<T> {
        fn default() -> Self {
            Self(Default::default(), Default::default())
        }
    }

    #[cfg(test)]
    impl<T: Config> RingBufferItem<T> {
        pub(crate) fn new(
            now: <<T as Config>::Time as Time>::Moment,
            record: <T as Config>::Record,
        ) -> Self {
            Self(now, record)
        }
    }

    impl<T: Config> RingBufferItem<T> {
        #[inline]
        fn split(self) -> (<<T as Config>::Time as Time>::Moment, <T as Config>::Record) {
            (self.0, self.1)
        }
    }

    #[cfg_attr(feature = "std", derive(Debug, PartialEq))]
    #[derive(Encode, Decode, Default, TypeInfo, MaxEncodedLen)]
    pub struct RingBufferIndex {
        #[codec(compact)]
        pub(crate) start: u64,
        #[codec(compact)]
        pub(crate) end: u64,
    }

    impl RingBufferIndex {
        #[inline]
        pub(crate) fn count(&self, max: u64) -> u64 {
            if self.start <= self.end {
                self.end - self.start
            } else {
                max + self.end - self.start
            }
        }

        #[inline]
        fn next(val: &mut u64, max: u64) {
            *val += 1;
            if *val == max {
                *val = 0
            }
        }
        /// Add value to ring buffer, returning an index for insert slot
        pub fn add(&mut self, max: u64) -> u64 {
            let v = self.end;
            Self::next(&mut self.end, max);
            if self.start == self.end {
                Self::next(&mut self.start, max);
            }
            v
        }
        /// Returns the ring buffer item iterator
        fn iter(&mut self, max: u64) -> RingBufferIterator<'_> {
            RingBufferIterator { inner: self, max }
        }
    }

    struct RingBufferIterator<'a> {
        inner: &'a mut RingBufferIndex,
        max: u64,
    }

    impl Iterator for RingBufferIterator<'_> {
        type Item = u64;
        fn next(&mut self) -> Option<Self::Item> {
            if self.inner.end == self.inner.start {
                None
            } else {
                let u = self.inner.start;
                RingBufferIndex::next(&mut self.inner.start, self.max);
                Some(u)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use frame_support::{assert_err, assert_ok, derive_impl, parameter_types, BoundedVec};

    use sp_runtime::{BuildStorage, DispatchError};

    use crate::{self as datalog, *};

    type Block = frame_system::mocking::MockBlock<Runtime>;
    type Item = RingBufferItem<Runtime>;

    frame_support::construct_runtime!(
        pub enum Runtime {
            System: frame_system,
            Timestamp: pallet_timestamp,
            Datalog: datalog,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Runtime {
        type Block = Block;
    }

    impl pallet_timestamp::Config for Runtime {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = ();
        type WeightInfo = ();
    }

    const WINDOW: u64 = 20;
    parameter_types! {
        pub const WindowSize: u64 = WINDOW;
        pub const MaximumMessageSize: u32 = 512;
    }

    impl Config for Runtime {
        type Time = Timestamp;
        type Record = BoundedVec<u8, MaximumMessageSize>;
        type RuntimeEvent = RuntimeEvent;
        type WindowSize = WindowSize;
        type WeightInfo = weights::TestWeightInfo;
    }

    pub fn new_test_ext() -> sp_io::TestExternalities {
        let storage = RuntimeGenesisConfig {
            system: Default::default(),
        }
        .build_storage()
        .unwrap();
        storage.into()
    }

    #[test]
    fn test_ringbuffer_index() {
        let mut idx: RingBufferIndex = Default::default();
        assert!(idx.start == idx.end);
        assert!(idx.start == 0);

        let i = idx.add(WINDOW);
        assert_eq!(i, 0);
        assert_eq!(idx.end, 1);

        assert_eq!(idx.count(WINDOW), 1);

        for _ in 0..WINDOW {
            let _ = idx.add(WINDOW);
        }
        assert_eq!(idx.count(WINDOW), WINDOW - 1);
    }

    #[test]
    fn test_store_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = BoundedVec::try_from(b"datalog".to_vec()).unwrap();
            assert_ok!(Datalog::record(
                RuntimeOrigin::signed(sender),
                record.clone()
            ));
            assert_eq!(Datalog::data(&sender), vec![Item::new(0, record)]);
        })
    }

    #[test]
    fn test_recycle_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;

            for i in 0..(WINDOW + 10) {
                assert_ok!(Datalog::record(
                    RuntimeOrigin::signed(sender),
                    BoundedVec::try_from(i.to_be_bytes().to_vec()).unwrap()
                ));
            }

            let data: Vec<_> = (11..(WINDOW + 10))
                .map(|i| Item::new(0, BoundedVec::try_from(i.to_be_bytes().to_vec()).unwrap()))
                .collect();

            assert_eq!(Datalog::data(&sender), data);
            let idx = Datalog::datalog_index(&sender);
            assert_eq!(idx, RingBufferIndex { start: 11, end: 10 });
            assert_eq!(idx.count(WINDOW), WINDOW - 1);
        })
    }

    #[test]
    fn test_erase_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = BoundedVec::try_from(b"datalog".to_vec()).unwrap();
            assert_ok!(Datalog::record(
                RuntimeOrigin::signed(sender),
                record.clone()
            ));
            // old log should be empty
            assert_eq!(Datalog::data(&sender), vec![Item::new(0, record)]);
            assert_eq!(
                Datalog::datalog_index(&sender),
                RingBufferIndex { start: 0, end: 1 }
            );

            assert_ok!(Datalog::erase(RuntimeOrigin::signed(sender)));
            assert_eq!(Datalog::data(&sender), vec![]);

            assert_eq!(
                Datalog::datalog_index(&sender),
                RingBufferIndex { start: 0, end: 0 }
            );
        })
    }

    #[test]
    fn test_bad_origin() {
        new_test_ext().execute_with(|| {
            assert_err!(
                Datalog::record(RuntimeOrigin::none(), Default::default()),
                DispatchError::BadOrigin
            );
        })
    }

    pub fn hash2vec(ss58hash: &str) -> BoundedVec<u8, MaximumMessageSize> {
        let ss58vec = bs58::decode(ss58hash)
            .into_vec()
            .expect("Couldn't decode from Base58");
        BoundedVec::try_from(ss58vec).expect("Couldn't bound decoded Base58")
    }

    #[test]
    fn test_store_ipfs_hashes() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let record = hash2vec("QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz4");

            assert_ok!(Datalog::record(
                RuntimeOrigin::signed(sender),
                record.clone()
            ));
            assert_eq!(Datalog::data(&sender), vec![Item::new(0, record.clone())]);

            let record2 = hash2vec("zdj7WWYAEceQ6ncfPZeRFjozov4dC7FaxU7SuMwzW4VuYBDta");

            Timestamp::set_timestamp(100);
            assert_ok!(Datalog::record(
                RuntimeOrigin::signed(sender),
                record2.clone()
            ));
            assert_eq!(
                Datalog::data(&sender),
                vec![
                    Item::new(0, record.clone()),
                    Item::new(100, record2.clone()),
                ]
            );
            let record3 = hash2vec("QmWboFP8XeBtFMbNYK3Ne8Z3gKFBSR5iQzkKgeNgQz3dz2");

            Timestamp::set_timestamp(200);
            assert_ok!(Datalog::record(
                RuntimeOrigin::signed(sender),
                record3.clone()
            ));
            assert_eq!(
                Datalog::data(&sender),
                vec![
                    Item::new(0, record),
                    Item::new(100, record2),
                    Item::new(200, record3),
                ]
            );
        })
    }
}
