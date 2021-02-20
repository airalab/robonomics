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
//! XCM version of Robonomics datalog runtime module.
//! This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use xcm::v0::{Error as XcmError, Junction, OriginKind, SendXcm, Xcm};

    #[pallet::config]
    pub trait Config: frame_system::Config + datalog::pallet::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The XCM sender module.
        type XcmSender: SendXcm;
        /// Runtime Call type, used for cross-messaging calls.
        type Call: Encode + From<datalog::pallet::Call<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// Record sended to another location.
        RecordSentSuccess(T::AccountId),
        /// Record didn't sent, error attached.
        RecordSentFailure(T::AccountId, XcmError),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(5_000_000)]
        fn record(
            origin: OriginFor<T>,
            parachain_id: u32,
            record: T::Record,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let location = Junction::Parachain { id: parachain_id };
            let call: <T as Config>::Call = datalog::pallet::Call::<T>::record(record).into();
            let message = Xcm::Transact {
                origin_type: OriginKind::SovereignAccount,
                call: call.encode(),
            };
            match T::XcmSender::send_xcm(location.into(), message.into()) {
                Ok(()) => Self::deposit_event(Event::RecordSentSuccess(sender)),
                Err(e) => Self::deposit_event(Event::RecordSentFailure(sender, e)),
            }
            Ok(().into())
        }
    }
}
