///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_std::collections::btree_map::BTreeMap;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New digital twin was registered: [sender, id].
        NewDigitalTwin(T::AccountId, u32),
        /// Digital twin topic was changed: [sender, id, topic, source]
        TopicChanged(T::AccountId, u32, H256, T::AccountId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn total)]
    /// Total count of registered digital twins.
    pub(super) type Total<T> = StorageValue<_, u32>;

    #[pallet::storage]
    #[pallet::getter(fn owner)]
    /// Get owner of digital twin with given id.
    pub(super) type Owner<T> =
        StorageMap<_, Twox64Concat, u32, <T as frame_system::Config>::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn digital_twin)]
    /// Get internal structure of difital twin in format: topic hash -> source account.
    pub(super) type DigitalTwin<T> =
        StorageMap<_, Twox64Concat, u32, BTreeMap<H256, <T as frame_system::Config>::AccountId>>;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create new digital twin.
        #[pallet::weight(50_000)]
        pub fn create(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let id = <Total<T>>::get().unwrap_or(0);
            <Total<T>>::put(id + 1);
            <Owner<T>>::insert(id, sender.clone());
            Self::deposit_event(Event::NewDigitalTwin(sender, id));
            Ok(().into())
        }

        /// Set data source account for difital twin.
        #[pallet::weight(50_000)]
        pub fn set_source(
            origin: OriginFor<T>,
            id: u32,
            topic: H256,
            source: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(
                <Owner<T>>::get(id) == Some(sender.clone()),
                "sender should be a twin owner"
            );
            Self::deposit_event(Event::TopicChanged(sender, id, topic, source.clone()));
            <DigitalTwin<T>>::mutate(id, |m| match m {
                None => {
                    let mut map = BTreeMap::new();
                    map.insert(topic, source);
                    *m = Some(map);
                }
                Some(map) => {
                    map.insert(topic, source);
                }
            });
            Ok(().into())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as digital_twin, *};

    use frame_support::{assert_err, assert_ok, parameter_types};
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
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            assert_eq!(DigitalTwin::total(), Some(1));
            assert_eq!(DigitalTwin::owner(0), Some(sender));
        })
    }

    #[test]
    fn test_set_source() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let bad_sender = 2;
            assert_ok!(DigitalTwin::create(RuntimeOrigin::signed(sender)));
            assert_err!(
                DigitalTwin::set_source(
                    RuntimeOrigin::signed(bad_sender),
                    0,
                    Default::default(),
                    bad_sender
                ),
                DispatchError::Other("sender should be a twin owner")
            );
            assert_ok!(DigitalTwin::set_source(
                RuntimeOrigin::signed(sender),
                0,
                Default::default(),
                bad_sender
            ));
        })
    }

    #[test]
    fn test_bad_origin() {
        new_test_ext().execute_with(|| {
            assert_err!(
                DigitalTwin::create(RuntimeOrigin::none()),
                DispatchError::BadOrigin
            );
        })
    }
}
