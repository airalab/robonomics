///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2023 Robonomics Network <research@robonomics.network>
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
//! SubXt compatible RWS pallet.

use codec::{Decode, Encode};
use sp_runtime::{DispatchResult, Perbill};
use std::fmt::Debug;
use substrate_subxt::{system::System, Encoded};
use substrate_subxt_proc_macro::{module, Call, Event};

/// The subset of the `pallet_robonomics_rws::Config` that a client must implement.
#[module]
pub trait RWS: System {}

/// Wrap extrinsic call.
#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct CallCall<'a, T: RWS> {
    pub subscription: &'a T::AccountId,
    pub call: &'a Encoded,
}

/// Updated bandwidth for an account.
#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct BandwidthEvent<T: RWS> {
    pub subscription: T::AccountId,
    pub ratio: Perbill,
}

/// Registered RWS subscription.
#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SubscriptionEvent<T: RWS> {
    pub subscription: T::AccountId,
    pub devices: Vec<T::AccountId>,
}

/// Runtime method executed using RWS subscription.
#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct NewCallEvent<T: RWS> {
    pub subscription: T::AccountId,
    pub result: DispatchResult,
}
