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
//! On-chain XCM setup & information.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migration;
// pub mod weights;

pub use pallet::*;
// pub use weights::WeightInfo;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::{ensure_root, pallet_prelude::*};
    use sp_runtime::traits::MaybeEquivalence;
    use xcm::latest::prelude::*;

    /// The current storage version
    const STORAGE_VERSION: frame_support::traits::StorageVersion = 
        frame_support::traits::StorageVersion::new(1);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// AssetId type for asset<>location linkage setup.
        type AssetId: Parameter + Copy + Default + MaxEncodedLen;
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Updated Relay XCM identifier.
        RelayNetworkChanged(NetworkId),
        /// Added new asset XCM location.
        AssetLinkAdded(T::AssetId, Location),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    /// Relay network identifier.
    #[pallet::storage]
    #[pallet::getter(fn relay_network)]
    pub(super) type RelayNetwork<T> = StorageValue<_, NetworkId>;

    /// AssetId to location mapping.
    #[pallet::storage]
    #[pallet::getter(fn location_of)]
    pub(super) type LocationOf<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, Location>;

    /// Location to AssetId mapping.
    #[pallet::storage]
    #[pallet::getter(fn assetid_of)]
    pub(super) type AssetIdOf<T: Config> =
        StorageMap<_, Blake2_128Concat, Location, T::AssetId>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight({10_000})]
        pub fn set_relay_network(origin: OriginFor<T>, network_id: NetworkId) -> DispatchResult {
            ensure_root(origin)?;

            <RelayNetwork<T>>::put(network_id);
            Self::deposit_event(Event::<T>::RelayNetworkChanged(network_id));

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight({10_000})]
        pub fn set_asset_link(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            location: Location,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let location_clone = location.clone();
            <LocationOf<T>>::insert(asset_id, location_clone.clone());
            <AssetIdOf<T>>::insert(location_clone, asset_id);
            Self::deposit_event(Event::<T>::AssetLinkAdded(asset_id, location));

            Ok(())
        }
    }

    impl<T: Config> MaybeEquivalence<Location, T::AssetId> for Pallet<T> {
        fn convert(id: &Location) -> Option<T::AssetId> {
            <AssetIdOf<T>>::get(id)
        }
        fn convert_back(what: &T::AssetId) -> Option<Location> {
            <LocationOf<T>>::get(what)
        }
    }
}
