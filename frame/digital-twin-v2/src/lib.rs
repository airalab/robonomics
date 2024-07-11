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
    use sp_core::H256;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New digital twin was registered: [id, sender, topic].
        NewDigitalTwin(u32, T::AccountId, H256),
        /// Digital twin was removed: [id]
        DigitalTwinDestroyed(u32),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn total)]
    /// Total count of registered digital twins.
    pub(super) type Total<T> = StorageValue<_, u32>;

    #[pallet::storage]
    #[pallet::getter(fn owner)]
    /// Digital twin with given id, owner address and topic hash.
    pub(super) type DigitalTwin<T> =
        StorageMap<_, Twox64Concat, u32, (<T as frame_system::Config>::AccountId, H256)>;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create new digital twin.
        #[pallet::weight(50_000)]
        pub fn create(origin: OriginFor<T>, topic: H256) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let id = <Total<T>>::get().unwrap_or(0);
            <Total<T>>::put(id + 1);
            <DigitalTwin<T>>::insert(id, (sender.clone(), topic));
            Self::deposit_event(Event::NewDigitalTwin(id, sender, topic));
            Ok(().into())
        }

        /// Destroy digital twin.
        #[pallet::weight(50_000)]
        pub fn destroy(origin: OriginFor<T>, id: u32) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            if let Some((owner, _)) = <DigitalTwin<T>>::get(id) {
                ensure!(
                    Some(owner) == Some(sender.clone()),
                    "sender should be a twin owner"
                );
                Self::deposit_event(Event::DigitalTwinDestroyed(id));
                let total = <Total<T>>::get().unwrap_or(0);
                <Total<T>>::put(total - 1);
                <DigitalTwin<T>>::remove(id);
                Ok(().into())
            } else {
                Err("digital twin doesn't exist".into())
            }
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
