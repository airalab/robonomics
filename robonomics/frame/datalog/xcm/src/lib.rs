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
//! Simple Robonomics datalog runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use xcm::v0::{SendXcm, Error as XcmError, Xcm, Junction, OriginKind};
use frame_support::{decl_event, decl_module};
use frame_system::ensure_signed;
use sp_std::prelude::*;

/// Datalog XCM module main trait.
pub trait Config: datalog::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// The XCM sender module.
    type XcmSender: SendXcm;
    /// Runtime Call type, used for cross-messaging calls.
	type Call: Encode + From<datalog::Call<Self>>;
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as frame_system::Config>::AccountId,
    {
        /// Record sended to another location.
        RecordSentSuccess(AccountId),
        /// Record didn't sent, error attached.
        RecordSentFailure(AccountId, XcmError),
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = 5_000_000]
        fn record(origin, parachain_id: u32, record: T::Record) {
            let sender = ensure_signed(origin)?;
            let location = Junction::Parachain { id: parachain_id };
            let call: <T as Config>::Call = datalog::Call::<T>::record(record).into();
            let message = Xcm::Transact { origin_type: OriginKind::Native, call: call.encode() };
            match T::XcmSender::send_xcm(location.into(), message.into()) {
                Ok(()) => Self::deposit_event(RawEvent::RecordSentSuccess(sender)),
                Err(e) => Self::deposit_event(RawEvent::RecordSentFailure(sender, e)),
            }
        }
    }
}
