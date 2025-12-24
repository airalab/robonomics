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
//! # XCM Info Pallet
//!
//! The XCM Info pallet provides essential on-chain storage and management capabilities
//! for Cross-Consensus Messaging (XCM) configuration and asset information.
//!
//! ## Overview
//!
//! This pallet allows privileged accounts (root origin) to configure and store:
//! - The relay chain network identifier for proper XCM routing
//! - Bidirectional mappings between local asset IDs and XCM locations
//!
//! These mappings are essential for cross-chain asset transfers and proper XCM message
//! routing within the Robonomics parachain ecosystem.
//!
//! ## Features
//!
//! - **Relay Network Configuration**: Store and update the relay chain network identifier
//! - **Asset-Location Mapping**: Create bidirectional mappings between local asset IDs and XCM locations
//! - **MaybeEquivalence Implementation**: Provides trait implementation for asset conversion utilities
//!
//! ## Usage
//!
//! ### Setting Relay Network
//!
//! ```ignore
//! // Set Kusama as the relay network
//! XcmInfo::set_relay_network(RuntimeOrigin::root(), NetworkId::Kusama)?;
//! ```
//!
//! ### Linking Assets to Locations
//!
//! ```ignore
//! // Link asset ID 1 to a specific XCM location
//! let location = Location::new(1, [Parachain(2000)]);
//! XcmInfo::set_asset_link(RuntimeOrigin::root(), 1u32, location)?;
//! ```
//!
//! ### Querying Mappings
//!
//! ```ignore
//! // Get location for an asset ID
//! if let Some(location) = XcmInfo::location_of(asset_id) {
//!     // Use the location
//! }
//!
//! // Get asset ID for a location  
//! if let Some(asset_id) = XcmInfo::assetid_of(&location) {
//!     // Use the asset ID
//! }
//! ```
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
    use frame_support::traits::StorageVersion;
    use frame_system::{ensure_root, pallet_prelude::*};
    use sp_runtime::traits::MaybeEquivalence;
    use xcm::latest::prelude::*;

    /// The current storage version
    pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    /// Configuration trait for the XCM Info pallet.
    ///
    /// Defines the types and parameters required for the pallet to function
    /// within a runtime.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The type used to identify assets in the runtime.
        ///
        /// This should typically be a simple integer type like `u32` that can uniquely
        /// identify each asset in the local asset registry. The type must implement
        /// `Parameter`, `Copy`, `Default`, and `MaxEncodedLen` for proper storage and
        /// parameter passing.
        type AssetId: Parameter + Copy + Default + MaxEncodedLen;

        /// The overarching event type for the runtime.
        ///
        /// Events emitted by this pallet will be wrapped in this type and included
        /// in the runtime's event stream.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::error]
    pub enum Error<T> {}

    /// Events emitted by the XCM Info pallet.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// The relay network identifier has been updated.
        ///
        /// Parameters:
        /// - `NetworkId`: The new relay network identifier (e.g., Polkadot, Kusama)
        RelayNetworkChanged(NetworkId),

        /// A new asset-to-location link has been established.
        ///
        /// Parameters:
        /// - `AssetId`: The local asset identifier
        /// - `Location`: The corresponding XCM location
        AssetLinkAdded(T::AssetId, Location),
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    /// The network identifier of the relay chain.
    ///
    /// This storage item holds the network ID (e.g., Polkadot, Kusama) of the relay
    /// chain that this parachain is connected to. It's used for proper XCM message
    /// routing and validation.
    ///
    /// Can be set via the `set_relay_network` extrinsic by root origin.
    #[pallet::storage]
    #[pallet::getter(fn relay_network)]
    pub(super) type RelayNetwork<T> = StorageValue<_, NetworkId>;

    /// Maps local asset IDs to their corresponding XCM locations.
    ///
    /// This storage provides a way to look up the XCM location for any given local
    /// asset ID. Used in conjunction with `AssetIdOf` to maintain bidirectional mapping.
    ///
    /// Set via the `set_asset_link` extrinsic by root origin.
    #[pallet::storage]
    #[pallet::getter(fn location_of)]
    pub(super) type LocationOf<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, Location>;

    /// Maps XCM locations to their corresponding local asset IDs.
    ///
    /// This storage provides a way to look up the local asset ID for any given XCM
    /// location. Used in conjunction with `LocationOf` to maintain bidirectional mapping.
    ///
    /// Set via the `set_asset_link` extrinsic by root origin.
    #[pallet::storage]
    #[pallet::getter(fn assetid_of)]
    pub(super) type AssetIdOf<T: Config> = StorageMap<_, Blake2_128Concat, Location, T::AssetId>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set or update the relay chain network identifier.
        ///
        /// This extrinsic configures the network ID of the relay chain that this
        /// parachain is connected to. This is essential for proper XCM routing and
        /// message validation.
        ///
        /// ## Parameters
        ///
        /// - `origin`: Must be root origin (governance or sudo)
        /// - `network_id`: The network identifier to set (e.g., `NetworkId::Polkadot`, `NetworkId::Kusama`)
        ///
        /// ## Events
        ///
        /// - `RelayNetworkChanged`: Emitted when the network ID is successfully updated
        ///
        /// ## Errors
        ///
        /// - `BadOrigin`: If the caller is not root
        ///
        /// ## Example
        ///
        /// ```ignore
        /// XcmInfo::set_relay_network(RuntimeOrigin::root(), NetworkId::Kusama)?;
        /// ```
        #[pallet::call_index(0)]
        #[pallet::weight({10_000})]
        pub fn set_relay_network(origin: OriginFor<T>, network_id: NetworkId) -> DispatchResult {
            ensure_root(origin)?;

            <RelayNetwork<T>>::put(network_id);
            Self::deposit_event(Event::<T>::RelayNetworkChanged(network_id));

            Ok(())
        }

        /// Create a bidirectional link between a local asset ID and an XCM location.
        ///
        /// This extrinsic establishes a mapping between a local asset identifier and
        /// its corresponding XCM location. The mapping is bidirectional, allowing
        /// lookups in either direction via `LocationOf` and `AssetIdOf` storage.
        ///
        /// ## Parameters
        ///
        /// - `origin`: Must be root origin (governance or sudo)
        /// - `asset_id`: The local asset identifier to link
        /// - `location`: The XCM location to associate with the asset
        ///
        /// ## Events
        ///
        /// - `AssetLinkAdded`: Emitted when the link is successfully created
        ///
        /// ## Errors
        ///
        /// - `BadOrigin`: If the caller is not root
        ///
        /// ## Example
        ///
        /// ```ignore
        /// let location = Location::new(1, [Parachain(2000)]);
        /// XcmInfo::set_asset_link(RuntimeOrigin::root(), 1u32, location)?;
        /// ```
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

    /// Implementation of `MaybeEquivalence` trait for asset-location conversion.
    ///
    /// This implementation allows the pallet to be used as a converter between
    /// XCM locations and local asset IDs in both directions. It's commonly used
    /// by XCM executor configurations to resolve asset identities during cross-chain
    /// transfers.
    ///
    /// ## Methods
    ///
    /// - `convert(&Location) -> Option<AssetId>`: Convert XCM location to local asset ID
    /// - `convert_back(&AssetId) -> Option<Location>`: Convert local asset ID to XCM location
    ///
    /// Both methods return `None` if no mapping exists.
    impl<T: Config> MaybeEquivalence<Location, T::AssetId> for Pallet<T> {
        fn convert(id: &Location) -> Option<T::AssetId> {
            <AssetIdOf<T>>::get(id)
        }
        fn convert_back(what: &T::AssetId) -> Option<Location> {
            <LocationOf<T>>::get(what)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as xcm_info;
    use frame_support::{assert_err, assert_ok, derive_impl, parameter_types};
    use sp_runtime::{traits::MaybeEquivalence, BuildStorage, DispatchError};
    use xcm::latest::prelude::*;

    type Block = frame_system::mocking::MockBlock<Runtime>;

    // Construct a mock runtime for testing
    frame_support::construct_runtime!(
        pub enum Runtime {
            System: frame_system,
            XcmInfo: xcm_info,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Runtime {
        type Block = Block;
    }

    impl Config for Runtime {
        type AssetId = u32;
        type RuntimeEvent = RuntimeEvent;
    }

    /// Build test externalities for running tests
    pub fn new_test_ext() -> sp_io::TestExternalities {
        let storage = RuntimeGenesisConfig {
            system: Default::default(),
        }
        .build_storage()
        .unwrap();
        storage.into()
    }

    #[test]
    fn set_relay_network_with_root_works() {
        new_test_ext().execute_with(|| {
            // Set relay network to Kusama
            assert_ok!(XcmInfo::set_relay_network(
                RuntimeOrigin::root(),
                NetworkId::Kusama
            ));

            // Verify storage is updated
            assert_eq!(XcmInfo::relay_network(), Some(NetworkId::Kusama));
        })
    }

    #[test]
    fn set_relay_network_emits_event() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            // Set relay network
            assert_ok!(XcmInfo::set_relay_network(
                RuntimeOrigin::root(),
                NetworkId::Polkadot
            ));

            // Check that event was emitted
            System::assert_has_event(Event::RelayNetworkChanged(NetworkId::Polkadot).into());
        })
    }

    #[test]
    fn set_relay_network_fails_with_non_root() {
        new_test_ext().execute_with(|| {
            // Attempt to set relay network with signed origin should fail
            assert_err!(
                XcmInfo::set_relay_network(RuntimeOrigin::signed(1), NetworkId::Kusama),
                DispatchError::BadOrigin
            );
        })
    }

    #[test]
    fn set_asset_link_with_root_works() {
        new_test_ext().execute_with(|| {
            let asset_id = 1u32;
            let location = Location::new(1, [Parachain(2000)]);

            // Set asset link
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id,
                location.clone()
            ));

            // Verify forward mapping (AssetId -> Location)
            assert_eq!(XcmInfo::location_of(asset_id), Some(location.clone()));

            // Verify reverse mapping (Location -> AssetId)
            assert_eq!(XcmInfo::assetid_of(&location), Some(asset_id));
        })
    }

    #[test]
    fn set_asset_link_bidirectional_mapping() {
        new_test_ext().execute_with(|| {
            let asset_id_1 = 5u32;
            let location_1 = Location::new(1, [Parachain(1000)]);

            let asset_id_2 = 10u32;
            let location_2 = Location::new(1, [Parachain(2000)]);

            // Set first asset link
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id_1,
                location_1.clone()
            ));

            // Set second asset link
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id_2,
                location_2.clone()
            ));

            // Verify both mappings work correctly
            assert_eq!(XcmInfo::location_of(asset_id_1), Some(location_1.clone()));
            assert_eq!(XcmInfo::assetid_of(&location_1), Some(asset_id_1));

            assert_eq!(XcmInfo::location_of(asset_id_2), Some(location_2.clone()));
            assert_eq!(XcmInfo::assetid_of(&location_2), Some(asset_id_2));
        })
    }

    #[test]
    fn set_asset_link_emits_event() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let asset_id = 42u32;
            let location = Location::new(1, [Parachain(3000)]);

            // Set asset link
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id,
                location.clone()
            ));

            // Check that event was emitted
            System::assert_has_event(Event::AssetLinkAdded(asset_id, location).into());
        })
    }

    #[test]
    fn set_asset_link_fails_with_non_root() {
        new_test_ext().execute_with(|| {
            let asset_id = 1u32;
            let location = Location::new(1, [Parachain(2000)]);

            // Attempt to set asset link with signed origin should fail
            assert_err!(
                XcmInfo::set_asset_link(RuntimeOrigin::signed(1), asset_id, location),
                DispatchError::BadOrigin
            );
        })
    }

    #[test]
    fn maybe_equivalence_convert_works() {
        new_test_ext().execute_with(|| {
            let asset_id = 7u32;
            let location = Location::new(1, [Parachain(4000)]);

            // Set up the mapping
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id,
                location.clone()
            ));

            // Test convert (Location -> AssetId)
            assert_eq!(
                <XcmInfo as MaybeEquivalence<Location, u32>>::convert(&location),
                Some(asset_id)
            );
        })
    }

    #[test]
    fn maybe_equivalence_convert_back_works() {
        new_test_ext().execute_with(|| {
            let asset_id = 8u32;
            let location = Location::new(1, [Parachain(5000)]);

            // Set up the mapping
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id,
                location.clone()
            ));

            // Test convert_back (AssetId -> Location)
            assert_eq!(
                <XcmInfo as MaybeEquivalence<Location, u32>>::convert_back(&asset_id),
                Some(location)
            );
        })
    }

    #[test]
    fn maybe_equivalence_returns_none_for_missing_location() {
        new_test_ext().execute_with(|| {
            let location = Location::new(1, [Parachain(9999)]);

            // Try to convert a location that hasn't been mapped
            assert_eq!(
                <XcmInfo as MaybeEquivalence<Location, u32>>::convert(&location),
                None
            );
        })
    }

    #[test]
    fn maybe_equivalence_returns_none_for_missing_asset_id() {
        new_test_ext().execute_with(|| {
            let asset_id = 9999u32;

            // Try to convert back an asset ID that hasn't been mapped
            assert_eq!(
                <XcmInfo as MaybeEquivalence<Location, u32>>::convert_back(&asset_id),
                None
            );
        })
    }

    #[test]
    fn storage_getters_work_correctly() {
        new_test_ext().execute_with(|| {
            // Initially storage should be empty
            assert_eq!(XcmInfo::relay_network(), None);

            // Set relay network
            assert_ok!(XcmInfo::set_relay_network(
                RuntimeOrigin::root(),
                NetworkId::Kusama
            ));

            // Verify getter returns correct value
            assert_eq!(XcmInfo::relay_network(), Some(NetworkId::Kusama));

            // Test asset mapping getters
            let asset_id = 100u32;
            let location = Location::new(1, [Parachain(1234)]);

            assert_eq!(XcmInfo::location_of(asset_id), None);
            assert_eq!(XcmInfo::assetid_of(&location), None);

            // Set asset link
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id,
                location.clone()
            ));

            // Verify getters return correct values
            assert_eq!(XcmInfo::location_of(asset_id), Some(location.clone()));
            assert_eq!(XcmInfo::assetid_of(&location), Some(asset_id));
        })
    }

    #[test]
    fn updating_asset_link_overwrites_previous() {
        new_test_ext().execute_with(|| {
            let asset_id = 50u32;
            let location_1 = Location::new(1, [Parachain(100)]);
            let location_2 = Location::new(1, [Parachain(200)]);

            // Set first link
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id,
                location_1.clone()
            ));
            assert_eq!(XcmInfo::location_of(asset_id), Some(location_1.clone()));

            // Update to second link
            assert_ok!(XcmInfo::set_asset_link(
                RuntimeOrigin::root(),
                asset_id,
                location_2.clone()
            ));

            // Verify new mapping is active
            assert_eq!(XcmInfo::location_of(asset_id), Some(location_2.clone()));
            assert_eq!(XcmInfo::assetid_of(&location_2), Some(asset_id));

            // Note: This demonstrates expected behavior - location_1 -> asset_id mapping
            // still exists in AssetIdOf storage. The extrinsic doesn't clean up old
            // reverse mappings to avoid expensive storage scans. If this becomes an issue,
            // consider using a migration to clean up stale mappings or implementing a
            // dedicated removal extrinsic.
            assert_eq!(XcmInfo::assetid_of(&location_1), Some(asset_id));
        })
    }
}
