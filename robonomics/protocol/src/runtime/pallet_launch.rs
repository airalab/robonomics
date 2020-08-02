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
//! SubXt compatible robonomics-launch pallet abstration.

use codec::{Codec, Decode, Encode, EncodeLike};
use core::marker::PhantomData;
use sp_runtime::traits::Member;
use std::fmt::Debug;
use substrate_subxt::system::{System, SystemEventsDecoder};
use substrate_subxt_proc_macro::{module, Call, Event, Store};

/// The subset of the `pallet_robonomics_launch::Trait` that a client must implement.
#[module]
pub trait Launch: System {
    type Parameter: Codec + EncodeLike + Member + Default;
}

/// Send launch request to robot with given parameter.
#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct LaunchCall<T: Launch> {
    robot: T::AccountId,
    param: T::Parameter,
}

/// New launch request sent.
#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct NewLaunch<T: Launch> {
    /// Sender account.
    pub sender: <T as System>::AccountId,
    /// Robot account with request to launch.
    pub robot: <T as System>::AccountId,
    /// Robot launch parameter.
    pub param: T::Parameter,
}
