///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! Simple Robonomics launch runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use xcm::v0::{Error as XcmError, Junction, OriginKind, SendXcm, Xcm};

    #[pallet::config]
    pub trait Config: frame_system::Config + launch::pallet::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The XCM sender module.
        type XcmSender: SendXcm;
        /// Runtime Call type, used for cross-messaging calls.
        type Call: Encode + From<launch::pallet::Call<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// Launch request sended to another location.
        LaunchSentSuccess(T::AccountId),
        /// Launch request didn't sent, error attached.
        LaunchSentFailure(T::AccountId, XcmError),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(5_000_000)]
        pub fn launch(
            origin: OriginFor<T>,
            parachain_id: u32,
            robot: T::AccountId,
            param: T::Parameter,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let location = Junction::Parachain { id: parachain_id };
            let call: <T as Config>::Call = launch::pallet::Call::<T>::launch(robot, param).into();
            let message = Xcm::<<T as Config>::Call>::Transact {
                origin_type: OriginKind::Native,
                require_weight_at_most: 1_000_000,
                call: call.encode().into(),
            };
            match T::XcmSender::send_xcm(location.into(), message.into()) {
                Ok(()) => Self::deposit_event(Event::LaunchSentSuccess(sender)),
                Err(e) => Self::deposit_event(Event::LaunchSentFailure(sender, e)),
            }
            Ok(().into())
        }
    }
}
