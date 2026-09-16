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
//! Custom origins for governance interventions.

pub use pallet_custom_origins::*;

use super::*;

/// `SortedMembers` implementation so `EnsureSignedBy` can compare against the
/// core-team multisig account currently held in `Origins` pallet storage.
///
/// The account is a storage item (see [`pallet_custom_origins::CoreTeamMultisig`])
/// rather than a `parameter_types!` constant, so it can be rotated by a
/// `Root`-authorized `set_core_team_multisig` extrinsic without a runtime
/// upgrade. Until it is set, this returns no members and `WhitelistOrigin`
/// falls back to `Root` only.
pub struct CoreTeamMultisigOnly;
impl frame_support::traits::SortedMembers<AccountId> for CoreTeamMultisigOnly {
    fn sorted_members() -> Vec<AccountId> {
        Origins::core_team_multisig().into_iter().collect()
    }
}

#[frame_support::pallet]
pub mod pallet_custom_origins {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    /// Storage version for migrations
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    /// The core-team multisig account.
    #[pallet::storage]
    pub type CoreTeamMultisig<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Update the core-team multisig account.
        ///
        /// Only callable by `Root` (e.g. via a passed referendum), so the
        /// account can be rotated without a runtime upgrade.
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn set_core_team_multisig(origin: OriginFor<T>, new: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            CoreTeamMultisig::<T>::put(new.clone());
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Returns the currently configured core-team multisig account, if any.
        pub fn core_team_multisig() -> Option<T::AccountId> {
            CoreTeamMultisig::<T>::get()
        }
    }

    /// The custom origins recognized by the Robonomics governance model.
    #[derive(
        PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug,
    )]
    #[pallet::origin]
    pub enum Origin {
        /// Origin able to dispatch a call whose hash has been whitelisted by
        /// the core-team multisig, once approved by a Whitelisted Caller
        /// referendum. Whitelisting alone never grants this origin the
        /// ability to dispatch anything: the referendum must still pass.
        WhitelistedCaller,
    }

    /// Ensures the `WhitelistedCaller` origin.
    pub struct WhitelistedCaller;
    impl<O: OriginTrait + From<Origin> + Clone> EnsureOrigin<O> for WhitelistedCaller
    where
        for<'a> &'a O::PalletsOrigin: TryInto<&'a Origin>,
    {
        type Success = ();

        fn try_origin(o: O) -> Result<Self::Success, O> {
            if let Ok(Origin::WhitelistedCaller) = o.clone().caller().try_into() {
                Ok(())
            } else {
                Err(o)
            }
        }

        #[cfg(feature = "runtime-benchmarks")]
        fn try_successful_origin() -> Result<O, ()> {
            Ok(O::from(Origin::WhitelistedCaller))
        }
    }
}
