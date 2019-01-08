// Copyright 2019 Airalab
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use srml_support::{StorageValue};
use system::ensure_signed;
use runtime_primitives::traits::Convert;

pub trait Trait: consensus::Trait + system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type ConvertAccountIdToSessionKey: Convert<Self::AccountId, Self::SessionKey>;
}

decl_module! {
    // Simple declaration of the `Module` type. Lets the macro know what its working on.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Add authority to validation round.
        fn append(origin, account: T::AccountId) {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;
            ensure!(sender == Self::key(), "only the current authority key can append authority");

            let mut items = <consensus::Module<T>>::authorities();
            items.push(T::ConvertAccountIdToSessionKey::convert(account));
            <consensus::Module<T>>::set_authorities(&items);
        }

        /// Set authority of validation round.
        fn set_authority(origin, ix: u32, account: T::AccountId) {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;
            ensure!(sender == Self::key(), "only the current authority key can append authority");

            <consensus::Module<T>>::set_authority(ix, &T::ConvertAccountIdToSessionKey::convert(account));
        }

        fn set_key(origin, new: T::AccountId) {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;
            ensure!(sender == Self::key(), "only the current authority key can change the authority key");

            Self::deposit_event(RawEvent::KeyChanged(Self::key()));
            <Key<T>>::put(new);
        }
    }
}

/// An event in this module.
decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        /// The sudoer just switched identity; the old key is supplied.
        KeyChanged(AccountId),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as AuthorityKey {
        Key get(key) config(): T::AccountId;
    }
}
