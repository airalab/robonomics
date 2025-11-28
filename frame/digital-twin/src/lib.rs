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
//! Digital Twin v2 runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
//!
//! This unified pallet replaces DigitalTwin v1, Datalog, and Launch pallets with a
//! single generic API supporting multiple topic types through a type-safe enum design.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod migrations;
pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, traits::Time};
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
    use sp_core::H256;
    use sp_std::prelude::*;

    use super::*;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

    /// Unified topic data enum supporting multiple topic types.
    #[derive(
        Encode,
        Decode,
        Clone,
        PartialEq,
        Eq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        DecodeWithMemTracking,
    )]
    #[scale_info(skip_type_params(T))]
    #[codec(encode_bound())]
    #[codec(decode_bound())]
    pub enum TopicData<T: Config> {
        /// Data source mapping (from DigitalTwin v1)
        Source(T::AccountId),
        /// Timestamped data record (from Datalog)
        Data {
            #[codec(compact)]
            timestamp: <T::Time as Time>::Moment,
            data: BoundedVec<u8, T::MaxDataSize>,
        },
        /// Robot launch command (from Launch)
        Command {
            target: T::AccountId,
            params: BoundedVec<u8, T::MaxDataSize>,
        },
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + TypeInfo {
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Current time source.
        type Time: Time;
        
        /// Maximum size of data/params in bytes.
        #[pallet::constant]
        type MaxDataSize: Get<u32>;
        
        /// Maximum number of topics per digital twin.
        #[pallet::constant]
        type MaxTopicsPerTwin: Get<u32>;
        
        /// Extrinsic weights
        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Sender is not the owner of the digital twin
        NotOwner,
        /// Digital twin does not exist
        TwinNotFound,
        /// Topic not found
        TopicNotFound,
        /// Too many topics for this digital twin
        TooManyTopics,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New digital twin was registered: [sender, id]
        NewDigitalTwin(T::AccountId, u32),
        /// Digital twin topic was set: [sender, id, topic]
        TopicSet(T::AccountId, u32, H256),
        /// Digital twin topic was removed: [sender, id, topic]
        TopicRemoved(T::AccountId, u32, H256),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            migrations::migrate::<T>()
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn twin_count)]
    /// Total count of registered digital twins.
    pub type TwinCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn owner)]
    /// Get owner of digital twin with given id.
    pub type Owner<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn topics)]
    /// Get topic data for a digital twin.
    pub type Topics<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, H256, TopicData<T>>;

    #[pallet::storage]
    #[pallet::getter(fn topic_list)]
    /// List of all topics for a digital twin (for enumeration).
    pub type TopicList<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, BoundedVec<H256, T::MaxTopicsPerTwin>, ValueQuery>;

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create new digital twin.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create())]
        pub fn create(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let id = TwinCount::<T>::get();
            TwinCount::<T>::put(id.saturating_add(1));
            Owner::<T>::insert(id, sender.clone());
            Self::deposit_event(Event::NewDigitalTwin(sender, id));
            Ok(().into())
        }

        /// Set source topic for digital twin (compatibility with v1).
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_topic())]
        pub fn set_source(
            origin: OriginFor<T>,
            id: u32,
            topic: H256,
            source: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let owner = Owner::<T>::get(id).ok_or(Error::<T>::TwinNotFound)?;
            ensure!(owner == sender, Error::<T>::NotOwner);

            let data = TopicData::Source(source);
            
            // Check if topic is new and add to list if necessary
            let topic_exists = Topics::<T>::contains_key(id, topic);
            if !topic_exists {
                TopicList::<T>::try_mutate(id, |list| {
                    list.try_push(topic)
                        .map_err(|_| Error::<T>::TooManyTopics)
                })?;
            }

            Topics::<T>::insert(id, topic, data);
            Self::deposit_event(Event::TopicSet(sender, id, topic));
            Ok(().into())
        }

        /// Set data topic for digital twin.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_topic())]
        pub fn set_data(
            origin: OriginFor<T>,
            id: u32,
            topic: H256,
            data: BoundedVec<u8, T::MaxDataSize>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let owner = Owner::<T>::get(id).ok_or(Error::<T>::TwinNotFound)?;
            ensure!(owner == sender, Error::<T>::NotOwner);

            let timestamp = T::Time::now();
            let topic_data = TopicData::Data { timestamp, data };
            
            // Check if topic is new and add to list if necessary
            let topic_exists = Topics::<T>::contains_key(id, topic);
            if !topic_exists {
                TopicList::<T>::try_mutate(id, |list| {
                    list.try_push(topic)
                        .map_err(|_| Error::<T>::TooManyTopics)
                })?;
            }

            Topics::<T>::insert(id, topic, topic_data);
            Self::deposit_event(Event::TopicSet(sender, id, topic));
            Ok(().into())
        }

        /// Set command topic for digital twin.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::set_topic())]
        pub fn set_command(
            origin: OriginFor<T>,
            id: u32,
            topic: H256,
            target: T::AccountId,
            params: BoundedVec<u8, T::MaxDataSize>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let owner = Owner::<T>::get(id).ok_or(Error::<T>::TwinNotFound)?;
            ensure!(owner == sender, Error::<T>::NotOwner);

            let topic_data = TopicData::Command { target, params };
            
            // Check if topic is new and add to list if necessary
            let topic_exists = Topics::<T>::contains_key(id, topic);
            if !topic_exists {
                TopicList::<T>::try_mutate(id, |list| {
                    list.try_push(topic)
                        .map_err(|_| Error::<T>::TooManyTopics)
                })?;
            }

            Topics::<T>::insert(id, topic, topic_data);
            Self::deposit_event(Event::TopicSet(sender, id, topic));
            Ok(().into())
        }

        /// Remove topic from digital twin.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::remove_topic())]
        pub fn remove_topic(
            origin: OriginFor<T>,
            id: u32,
            topic: H256,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let owner = Owner::<T>::get(id).ok_or(Error::<T>::TwinNotFound)?;
            ensure!(owner == sender, Error::<T>::NotOwner);
            ensure!(Topics::<T>::contains_key(id, topic), Error::<T>::TopicNotFound);

            Topics::<T>::remove(id, topic);
            TopicList::<T>::mutate(id, |list| {
                list.retain(|&t| t != topic);
            });

            Self::deposit_event(Event::TopicRemoved(sender, id, topic));
            Ok(().into())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as digital_twin, *};

    use frame_support::{assert_err, assert_ok, derive_impl, parameter_types, traits::ConstU32, BoundedVec};
    use sp_core::H256;
    use sp_runtime::BuildStorage;

    type Block = frame_system::mocking::MockBlock<Runtime>;

    frame_support::construct_runtime!(
        pub enum Runtime {
            System: frame_system,
            Timestamp: pallet_timestamp,
            DigitalTwin: digital_twin,
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

    impl Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type Time = Timestamp;
        type MaxDataSize = ConstU32<512>;
        type MaxTopicsPerTwin = ConstU32<100>;
        type WeightInfo = ();
    }

    pub fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap();
        storage.into()
    }

    #[test]
    fn test_create() {
        new_test_ext().execute_with(|| {
            assert_eq!(DigitalTwin::twin_count(), 0);
            let sender = 1;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            assert_eq!(DigitalTwin::twin_count(), 1);
            assert_eq!(DigitalTwin::owner(0), Some(sender));
        })
    }

    #[test]
    fn test_set_topic_source() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let source = 2;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            
            let topic = H256::default();
            assert_ok!(DigitalTwin::set_source(
                RuntimeOrigin::signed(sender),
                0,
                topic,
                source
            ));
            
            // Verify the data was stored
            let stored = DigitalTwin::topics(0, topic);
            assert!(stored.is_some());
            if let Some(TopicData::Source(s)) = stored {
                assert_eq!(s, source);
            } else {
                panic!("Expected Source variant");
            }
            assert_eq!(DigitalTwin::topic_list(0), vec![topic]);
        })
    }

    #[test]
    fn test_set_topic_data() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            
            let topic = H256::default();
            let payload = BoundedVec::try_from(b"test data".to_vec()).unwrap();
            
            assert_ok!(DigitalTwin::set_data(
                RuntimeOrigin::signed(sender),
                0,
                topic,
                payload.clone()
            ));
            
            // Verify the data was stored
            let stored = DigitalTwin::topics(0, topic);
            assert!(stored.is_some());
            if let Some(TopicData::Data { data: stored_data, .. }) = stored {
                assert_eq!(stored_data, payload);
            } else {
                panic!("Expected Data variant");
            }
        })
    }

    #[test]
    fn test_set_topic_command() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let robot = 2;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            
            let topic = H256::default();
            let params = BoundedVec::try_from(b"launch params".to_vec()).unwrap();
            
            assert_ok!(DigitalTwin::set_command(
                RuntimeOrigin::signed(sender),
                0,
                topic,
                robot,
                params.clone()
            ));
            
            // Verify the data was stored
            let stored = DigitalTwin::topics(0, topic);
            assert!(stored.is_some());
            if let Some(TopicData::Command { target, params: stored_params }) = stored {
                assert_eq!(target, robot);
                assert_eq!(stored_params, params);
            } else {
                panic!("Expected Command variant");
            }
        })
    }

    #[test]
    fn test_set_topic_not_owner() {
        new_test_ext().execute_with(|| {
            let owner = 1;
            let bad_sender = 2;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(owner)));
            
            let topic = H256::default();
            assert_err!(
                DigitalTwin::set_source(
                    RuntimeOrigin::signed(bad_sender),
                    0,
                    topic,
                    3
                ),
                Error::<Runtime>::NotOwner
            );
        })
    }

    #[test]
    fn test_set_topic_twin_not_found() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let topic = H256::default();
            assert_err!(
                DigitalTwin::set_source(
                    RuntimeOrigin::signed(sender),
                    999,
                    topic,
                    2
                ),
                Error::<Runtime>::TwinNotFound
            );
        })
    }

    #[test]
    fn test_remove_topic() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            
            let topic = H256::default();
            assert_ok!(DigitalTwin::set_source(
                RuntimeOrigin::signed(sender),
                0,
                topic,
                2
            ));
            
            assert!(DigitalTwin::topics(0, topic).is_some());
            
            assert_ok!(DigitalTwin::remove_topic(
                RuntimeOrigin::signed(sender),
                0,
                topic
            ));
            
            assert!(DigitalTwin::topics(0, topic).is_none());
            assert_eq!(DigitalTwin::topic_list(0), vec![]);
        })
    }

    #[test]
    fn test_remove_topic_not_owner() {
        new_test_ext().execute_with(|| {
            let owner = 1;
            let bad_sender = 2;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(owner)));
            
            let topic = H256::default();
            assert_ok!(DigitalTwin::set_source(
                RuntimeOrigin::signed(owner),
                0,
                topic,
                3
            ));
            
            assert_err!(
                DigitalTwin::remove_topic(
                    RuntimeOrigin::signed(bad_sender),
                    0,
                    topic
                ),
                Error::<Runtime>::NotOwner
            );
        })
    }

    #[test]
    fn test_remove_topic_not_found() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            
            let topic = H256::default();
            assert_err!(
                DigitalTwin::remove_topic(
                    RuntimeOrigin::signed(sender),
                    0,
                    topic
                ),
                Error::<Runtime>::TopicNotFound
            );
        })
    }

    #[test]
    fn test_multiple_topics() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            
            let topic1 = H256::from_low_u64_be(1);
            let topic2 = H256::from_low_u64_be(2);
            let topic3 = H256::from_low_u64_be(3);
            
            assert_ok!(DigitalTwin::set_source(
                RuntimeOrigin::signed(sender),
                0,
                topic1,
                2
            ));
            
            assert_ok!(DigitalTwin::set_data(
                RuntimeOrigin::signed(sender),
                0,
                topic2,
                BoundedVec::try_from(b"data".to_vec()).unwrap()
            ));
            
            assert_ok!(DigitalTwin::set_command(
                RuntimeOrigin::signed(sender),
                0,
                topic3,
                3,
                BoundedVec::try_from(b"params".to_vec()).unwrap()
            ));
            
            let list = DigitalTwin::topic_list(0);
            assert_eq!(list.len(), 3);
            assert!(list.contains(&topic1));
            assert!(list.contains(&topic2));
            assert!(list.contains(&topic3));
        })
    }

    #[test]
    fn test_bad_origin() {
        new_test_ext().execute_with(|| {
            assert_err!(
                DigitalTwin::create(RuntimeOrigin::none()),
                sp_runtime::DispatchError::BadOrigin
            );
        })
    }
}
