///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
//! Minimal Pallet that injects a ParachainId into Runtime storage from

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use cumulus_primitives_core::{NetworkId, ParaId};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _config: core::marker::PhantomData<T>,
        pub parachain_id: ParaId,
        pub relay_network: NetworkId,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                parachain_id: 100.into(),
                relay_network: NetworkId::Polkadot,
                _config: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            ParachainId::<T>::put(self.parachain_id);
            RelayNetwork::<T>::put(self.relay_network);
        }
    }

    #[pallet::type_value]
    pub(super) fn DefaultForParachainId() -> ParaId {
        100.into()
    }

    #[pallet::type_value]
    pub(super) fn DefaultForRelayNetwork() -> NetworkId {
        NetworkId::Kusama
    }

    #[pallet::storage]
    pub(super) type ParachainId<T: Config> =
        StorageValue<_, ParaId, ValueQuery, DefaultForParachainId>;

    #[pallet::storage]
    pub(super) type RelayNetwork<T: Config> =
        StorageValue<_, NetworkId, ValueQuery, DefaultForRelayNetwork>;

    impl<T: Config> Get<ParaId> for Pallet<T> {
        fn get() -> ParaId {
            ParachainId::<T>::get()
        }
    }

    impl<T: Config> Get<NetworkId> for Pallet<T> {
        fn get() -> NetworkId {
            RelayNetwork::<T>::get()
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn parachain_id() -> ParaId {
            ParachainId::<T>::get()
        }
        pub fn relay_network() -> NetworkId {
            RelayNetwork::<T>::get()
        }
    }
}
