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
//! Digital twin runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_event, decl_module, decl_storage, ensure};
use frame_system::ensure_signed;
use sp_core::H256;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;

/// Datalog module main trait.
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as frame_system::Config>::AccountId,
    {
        /// New digital twin was registered: [sender, id].
        NewDigitalTwin(AccountId, u32),
        /// Digital twin topic was changed: [sender, id, topic, source]
        TopicChanged(AccountId, u32, H256, AccountId),
    }
}

decl_storage! {
    trait Store for Module<T: Config> as DigitalTwin {
        /// Total count of registered digital twins.
        Total get(fn total): u32;
        /// Get owner of digital twin with given id.
        Owner get(fn owner): map hasher(blake2_128_concat)
                             u32 => T::AccountId;
        /// Get internal structure of difital twin in format: topic hash -> source account.
        DigitalTwin get(fn digital_twin): map hasher(blake2_128_concat)
                                          u32 => BTreeMap<H256, T::AccountId>;
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Create new digital twin.
        #[weight = 50_000]
        fn create(origin) {
            let sender = ensure_signed(origin)?;
            let id = <Total>::get();
            <Owner<T>>::insert(id, sender.clone());
            <Total>::put(id + 1);
            Self::deposit_event(RawEvent::NewDigitalTwin(sender, id));
        }

        /// Set data source account for difital twin.
        #[weight = 50_000]
        fn set_source(origin, id: u32, topic: H256, source: T::AccountId) {
            let sender = ensure_signed(origin)?;
            ensure!(<Owner<T>>::get(id) == sender, "sender should be a twin owner");
            Self::deposit_event(RawEvent::TopicChanged(sender, id, topic, source.clone()));
            <DigitalTwin<T>>::mutate(id, |m| m.insert(topic, source));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use frame_support::{assert_err, assert_ok, impl_outer_origin, parameter_types};
    use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchError};

    impl_outer_origin! {
        pub enum Origin for Runtime {}
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Runtime;

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    impl frame_system::Config for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Call = ();
        type Hash = H256;
        type Hashing = sp_runtime::traits::BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = ();
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
        type Event = ();
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();
        storage.into()
    }

    type DigitalTwin = Module<Runtime>;

    #[test]
    fn test_create() {
        new_test_ext().execute_with(|| {
            assert_eq!(DigitalTwin::total(), 0);
            let sender = 1;
            assert_ok!(DigitalTwin::create(Origin::signed(sender)));
            assert_eq!(DigitalTwin::total(), 1);
            assert_eq!(DigitalTwin::owner(0), sender);
        })
    }

    #[test]
    fn test_set_source() {
        new_test_ext().execute_with(|| {
            let sender = 1;
            let bad_sender = 2;
            assert_ok!(DigitalTwin::create(Origin::signed(sender)));
            assert_err!(
                DigitalTwin::set_source(
                    Origin::signed(bad_sender),
                    0,
                    Default::default(),
                    bad_sender
                ),
                DispatchError::Other("sender should be a twin owner")
            );
            assert_ok!(DigitalTwin::set_source(
                Origin::signed(sender),
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
                DigitalTwin::create(Origin::none()),
                DispatchError::BadOrigin
            );
        })
    }
}
