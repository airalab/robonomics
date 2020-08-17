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
//! Simple parachain validator management system.

use frame_support::{decl_event, decl_module, decl_storage};
use frame_system::ensure_root;
use sp_staking::SessionIndex;
use sp_std::prelude::*;

/// Robonomics parachain validator accounting.
pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as frame_system::Trait>::AccountId,
    {
        /// New validators list.
        NewValidators(Vec<AccountId>),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Datalog {
        /// List of accounts elected as validators. 
        Validators get(fn validators): Vec<T::AccountId>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Set new validators list. 
        #[weight = 50_000]
        fn set_validators(origin, validators: Vec<T::AccountId>) {
            let _ = ensure_root(origin)?;
            <Validators<T>>::put(validators.clone());
            Self::deposit_event(RawEvent::NewValidators(validators));
        }
    }
}

pub struct SessionManager<T>(sp_std::marker::PhantomData<T>);
impl<T: Trait> pallet_session::SessionManager<T::AccountId> for SessionManager<T> {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        Some(<Validators<T>>::get())
	}
	fn end_session(_: SessionIndex) {}
	fn start_session(_: SessionIndex) {}
}
