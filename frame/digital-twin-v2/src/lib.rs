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
    // use sp_core::H256;
    use sp_std::collections::btree_map::BTreeMap;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The maximum string length for digital twin storage map.
        #[pallet::constant]
        type MaxLength: Get<u32>;

        /// The maximum data map size for digital twin.
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
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[derive(Encode, Decode, Default, TypeInfo, MaxEncodedLen, PartialEqNoBound, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct DigitalTwin<T: Config> {
        pub data: BTreeMap<BoundedVec<u8, T::MaxLength>, BoundedVec<u8, T::MaxLength>>,
        pub count: u32,
    }

    #[pallet::storage]
    /// Digital twin storage format: address -> digital twin.
    pub(super) type DigitalTwins<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, DigitalTwin<T>>;

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
            <DigitalTwins<T>>::mutate(sender.clone(), |dt| match dt {
                None => {
                    let mut data = BTreeMap::new();
                    data.insert(key.clone(), value.clone());
                    *dt = Some(DigitalTwin { data, count: 1 });
                    Self::deposit_event(Event::DigitalTwinRecordInserted(sender, 1, key, value));

                    // TODO: callback

                    Ok(().into())
                }
                Some(digital_twin) => {
                    if digital_twin.data.contains_key(&key.clone()) {
                        digital_twin.data.insert(key.clone(), value.clone());
                        Self::deposit_event(Event::DigitalTwinRecordUpdated(
                            sender,
                            digital_twin.count,
                            key,
                            value,
                        ));

                        // TODO: callback

                        Ok(().into())
                    } else {
                        if digital_twin.count < T::MaxCount::get() {
                            digital_twin.data.insert(key.clone(), value.clone());
                            digital_twin.count += 1;
                            Self::deposit_event(Event::DigitalTwinRecordInserted(
                                sender,
                                digital_twin.count,
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
            <DigitalTwins<T>>::mutate(sender.clone(), |dt| match dt {
                Some(digital_twin) => match digital_twin.data.remove(&key) {
                    None => Err("Unknown key!".into()),
                    _ => {
                        digital_twin.count -= 1;
                        Self::deposit_event(Event::DigitalTwinRecordRemoved(
                            sender,
                            digital_twin.count,
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
    use sp_core::H256;
    use sp_runtime::{traits::IdentityLookup, BuildStorage, DispatchError};

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
            assert_eq!(DigitalTwin::total(), None);
            let sender = 1;
            let topic = "0x0000000000000000000000000000000000000000000000000000000000000000"
                .parse::<H256>()
                .unwrap();
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender), topic));
            assert_eq!(DigitalTwin::total(), Some(1));
            assert_eq!(DigitalTwin::owner(0), Some((sender, topic)));
        })
    }

    #[test]
    fn test_destroy() {
        new_test_ext().execute_with(|| {
            assert_eq!(DigitalTwin::total(), None);
            let sender = 1;
            let topic = "0x0000000000000000000000000000000000000000000000000000000000000000"
                .parse::<H256>()
                .unwrap();
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender), topic));
            assert_eq!(DigitalTwin::total(), Some(1));
            assert_eq!(DigitalTwin::owner(0), Some((sender, topic)));
            assert_ok!(DigitalTwin::destroy(RuntimeOrigin::signed(sender), 0));
            assert_eq!(DigitalTwin::total(), Some(0));
        })
    }

    #[test]
    fn test_bad_origin() {
        new_test_ext().execute_with(|| {
            let topic = "0x0000000000000000000000000000000000000000000000000000000000000000"
                .parse::<H256>()
                .unwrap();
            assert_err!(
                DigitalTwin::create(RuntimeOrigin::none(), topic),
                DispatchError::BadOrigin
            );
        })
    }
}
