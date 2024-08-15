///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2023 Robonomics Network <research@robonomics.network>
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
//! Digital twin runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::collections::{btree_map::BTreeMap, vec_deque::VecDeque};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The maximum string length for digital twin storage map.
        #[pallet::constant]
        type MaxLength: Get<u32>;

        /// The maximum map length for digital twin.
        #[pallet::constant]
        type MaxCount: Get<u32>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New digital twin record was added: [sender, count, key, value].
        DigitalTwinRecordInserted(
            T::AccountId,
            u32,
            BoundedVec<u8, T::MaxLength>,
            BoundedVec<u8, T::MaxLength>,
        ),
        /// Digital twin record was updated: [sender, count, key, value].
        DigitalTwinRecordUpdated(
            T::AccountId,
            u32,
            BoundedVec<u8, T::MaxLength>,
            BoundedVec<u8, T::MaxLength>,
        ),
        /// Digital twin record was removed: [sender, count, key].
        DigitalTwinRecordRemoved(T::AccountId, u32, BoundedVec<u8, T::MaxLength>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_idle(_block: BlockNumberFor<T>, weight: Weight) -> Weight {
            log::warn!("⚠️  on idle..");
            weight
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn callback_queue)]
    /// ???
    pub(super) type CallbackQueue<T: Config> =
        StorageValue<_, VecDeque<BoundedVec<u8, T::MaxLength>>>;

    #[pallet::storage]
    #[pallet::getter(fn digital_twin)]
    /// Digital twin storage format:
    /// address -> digital twin (map string -> string).
    pub(super) type DigitalTwins<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        BTreeMap<BoundedVec<u8, T::MaxLength>, BoundedVec<u8, T::MaxLength>>,
    >;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Store data in digital twin.
        #[pallet::weight(50_000)]
        pub fn insert(
            origin: OriginFor<T>,
            key: BoundedVec<u8, T::MaxLength>,
            value: BoundedVec<u8, T::MaxLength>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            <DigitalTwins<T>>::mutate(sender.clone(), |m| match m {
                None => {
                    // Create record
                    let mut map = BTreeMap::new();
                    map.insert(key.clone(), value.clone());
                    *m = Some(map);
                    Self::deposit_event(Event::DigitalTwinRecordInserted(sender, 1, key, value));

                    // TODO: callback

                    Ok(().into())
                }
                Some(map) => {
                    if map.contains_key(&key.clone()) {
                        // Update record
                        map.insert(key.clone(), value.clone());
                        Self::deposit_event(Event::DigitalTwinRecordUpdated(
                            sender,
                            map.len() as u32,
                            key,
                            value,
                        ));

                        // TODO: callback

                        Ok(().into())
                    } else {
                        // Insert record
                        if (map.len() as u32) < T::MaxCount::get() {
                            map.insert(key.clone(), value.clone());
                            Self::deposit_event(Event::DigitalTwinRecordInserted(
                                sender,
                                map.len() as u32,
                                key,
                                value,
                            ));

                            // TODO: callback

                            Ok(().into())
                        } else {
                            Err("Max count!".into())
                        }
                    }
                }
            })
        }

        /// Remove data from digital twin.
        #[pallet::weight(50_000)]
        pub fn remove(
            origin: OriginFor<T>,
            key: BoundedVec<u8, T::MaxLength>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            <DigitalTwins<T>>::mutate(sender.clone(), |m| match m {
                Some(map) => match map.remove(&key) {
                    None => Err("Unknown key!".into()),
                    _ => {
                        Self::deposit_event(Event::DigitalTwinRecordRemoved(
                            sender,
                            map.len() as u32,
                            key,
                        ));
                        Ok(().into())
                    }
                },
                None => Err("Digital twin doesn't exist!".into()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as digital_twin, *};

    use frame_support::{assert_err, assert_ok, parameter_types};
    use sp_core::ConstU32;
    use sp_runtime::{traits::IdentityLookup, BoundedVec, BuildStorage, DispatchError};

    type Block = frame_system::mocking::MockBlock<Runtime>;

    frame_support::construct_runtime!(
        pub enum Runtime {
            System: frame_system,
            DigitalTwin: digital_twin,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    impl frame_system::Config for Runtime {
        type RuntimeOrigin = RuntimeOrigin;
        type Nonce = u64;
        type Block = Block;
        type RuntimeCall = RuntimeCall;
        type Hash = sp_core::H256;
        type Hashing = sp_runtime::traits::BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = ();
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

    impl Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type MaxLength = ConstU32<512>;
        type MaxCount = ConstU32<256>;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap();
        storage.into()
    }

    #[test]
    fn test_create() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            assert_eq!(DigitalTwin::digital_twin(sender), None);
            let key = BoundedVec::new();
            let value = BoundedVec::new();
            assert_ok!(DigitalTwin::insert(
                RuntimeOrigin::signed(sender),
                key,
                value
            ));
            assert_eq!(DigitalTwin::digital_twin(sender).expect("").len(), 1);
        })
    }

    #[test]
    fn test_destroy() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            assert_eq!(DigitalTwin::digital_twin(sender), None);
            let key = BoundedVec::new();
            let value = BoundedVec::new();
            assert_ok!(DigitalTwin::insert(
                RuntimeOrigin::signed(sender),
                key.clone(),
                value
            ));
            assert_eq!(DigitalTwin::digital_twin(sender).expect("").len(), 1);
            assert_ok!(DigitalTwin::remove(RuntimeOrigin::signed(sender), key,));
            assert_eq!(DigitalTwin::digital_twin(sender).expect("").len(), 0);
        })
    }

    #[test]
    fn test_bad_origin() {
        new_test_ext().execute_with(|| {
            let key = BoundedVec::new();
            let value = BoundedVec::new();
            assert_err!(
                DigitalTwin::insert(RuntimeOrigin::none(), key, value),
                DispatchError::BadOrigin
            );
        })
    }
}
