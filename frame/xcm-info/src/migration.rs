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
//! Storage migrations for pallet-xcm-info

use super::*;
use frame_support::{
    pallet_prelude::*,
    traits::{OnRuntimeUpgrade, StorageVersion},
    weights::Weight,
};

#[cfg(feature = "try-runtime")]
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

/// Migration from v3::MultiLocation to v5::Location
pub mod v1 {
    use super::*;

    /// Old storage format using v3 MultiLocation
    pub mod v0 {
        use super::*;
        use frame_support::storage_alias;
        use xcm::v3::MultiLocation;

        #[storage_alias]
        pub type LocationOf<T: pallet::Config> = StorageMap<
            pallet::Pallet<T>,
            Blake2_128Concat,
            <T as pallet::Config>::AssetId,
            MultiLocation,
        >;

        #[storage_alias]
        pub type AssetIdOf<T: pallet::Config> = StorageMap<
            pallet::Pallet<T>,
            Blake2_128Concat,
            MultiLocation,
            <T as pallet::Config>::AssetId,
        >;
    }

    /// Migrate from MultiLocation to Location
    pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

    impl<T: pallet::Config> OnRuntimeUpgrade for MigrateToV1<T> {
        fn on_runtime_upgrade() -> Weight {
            let mut weight = Weight::zero();

            // Check current version
            let onchain_version = pallet::Pallet::<T>::on_chain_storage_version();
            if onchain_version >= 1 {
                return weight;
            }

            // Migrate LocationOf storage

            v0::LocationOf::<T>::translate::<xcm::v3::MultiLocation, _>(
                |_asset_id, old_location| {
                    weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                    // Convert v3::MultiLocation to latest Location using VersionedLocation
                    let versioned = xcm::VersionedLocation::V3(old_location);
                    versioned.try_into().ok()
                },
            );

            // Migrate AssetIdOf storage - need to handle key change
            let drained_asset_ids: sp_std::vec::Vec<_> = v0::AssetIdOf::<T>::drain().collect();
            let _asset_id_of_count = drained_asset_ids.len();
            for (old_location, asset_id) in drained_asset_ids {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 2));

                // Convert v3::MultiLocation to latest Location using VersionedLocation
                let versioned = xcm::VersionedLocation::V3(old_location);
                if let Ok(new_location) = <xcm::VersionedLocation as TryInto<
                    xcm::latest::prelude::Location,
                >>::try_into(versioned)
                {
                    pallet::AssetIdOf::<T>::insert(new_location, asset_id);
                }
            }

            // Update storage version
            STORAGE_VERSION.put::<pallet::Pallet<T>>();

            weight
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
            let location_of_count = v0::LocationOf::<T>::iter().count();
            let asset_id_of_count = v0::AssetIdOf::<T>::iter().count();

            Ok((location_of_count as u32, asset_id_of_count as u32).encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
            let (old_location_count, old_asset_id_count): (u32, u32) =
                Decode::decode(&mut &state[..])
                    .map_err(|_| "Failed to decode pre-upgrade state")?;

            let new_location_count = pallet::LocationOf::<T>::iter().count() as u32;
            let new_asset_id_count = pallet::AssetIdOf::<T>::iter().count() as u32;

            ensure!(
                new_location_count <= old_location_count,
                "LocationOf entries count mismatch after migration"
            );
            ensure!(
                new_asset_id_count <= old_asset_id_count,
                "AssetIdOf entries count mismatch after migration"
            );

            let version = pallet::Pallet::<T>::on_chain_storage_version();
            ensure!(version == 1, "Storage version should be 1 after migration");

            Ok(())
        }
    }
}
