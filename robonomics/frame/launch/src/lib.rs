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
//! Simple robot launch runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode, EncodeLike};
use cumulus_primitives::{
    xcmp::{XCMPMessageHandler, XCMPMessageSender},
    ParaId,
};
use frame_support::{decl_event, decl_module, decl_storage};
use frame_system::ensure_signed;
use sp_runtime::traits::Member;
use sp_std::prelude::*;

/// Launch module main trait.
pub trait Trait: frame_system::Trait {
    /// This module can send XCMP messages.
    type XCMPMessageSender: XCMPMessageSender<XCMPMessage<Self::AccountId, Self::Parameter>>;
    /// Robot launch parameter data type.
    type Parameter: Codec + EncodeLike + Member;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

/// Robot launch XCMP message trait.
#[derive(Encode, Decode)]
pub enum XCMPMessage<AccountId, Parameter> {
    /// Launch robot with given launch parameter.
    LaunchRobot(AccountId, AccountId, Parameter),
}

decl_event! {
    pub enum Event<T>
    where AccountId = <T as frame_system::Trait>::AccountId,
          Parameter = <T as Trait>::Parameter,
    {
        /// Launch a robot with given parameter: sender, robot, parameter.
        NewLaunch(AccountId, AccountId, Parameter),
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Launch {}
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Launch a robot with given parameter.
        #[weight = 1_000_000]
        fn launch(origin, robot: T::AccountId, param: T::Parameter) {
            let sender = ensure_signed(origin)?;
            Self::deposit_event(RawEvent::NewLaunch(sender, robot, param));
        }

        /// Launch a robot in parachain with given parameter.
        #[weight = 10_000_000]
        fn launch_on(origin, para_id: ParaId, robot: T::AccountId, param: T::Parameter) {
            let sender = ensure_signed(origin)?;
            let msg = XCMPMessage::LaunchRobot(sender, robot, param);
            T::XCMPMessageSender::send_xcmp_message(para_id, &msg).expect("should not fail");
        }
    }
}

impl<T: Trait> XCMPMessageHandler<XCMPMessage<T::AccountId, T::Parameter>> for Module<T> {
    fn handle_xcmp_message(_src: ParaId, msg: &XCMPMessage<T::AccountId, T::Parameter>) {
        match msg {
            XCMPMessage::LaunchRobot(sender, robot, param) => Self::deposit_event(
                RawEvent::NewLaunch(sender.clone(), robot.clone(), param.clone()),
            ),
        }
    }
}
