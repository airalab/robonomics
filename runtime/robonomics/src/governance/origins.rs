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

#[frame_support::pallet]
pub mod pallet_custom_origins {
    use frame_support::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The custom origins recognized by the Robonomics governance model.
    #[derive(
        PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug,
    )]
    #[pallet::origin]
    pub enum Origin {
        /// Origin able to dispatch a call whose hash has been whitelisted by
        /// the Core Team collective, once approved by a Whitelisted Caller
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
