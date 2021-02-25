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
//! SubXt compatible robonomics-datalog pallet abstration.

use codec::{Codec, Decode, Encode, EncodeLike};
use core::marker::PhantomData;
use sp_runtime::traits::Member;
use std::fmt::Debug;
use substrate_subxt::system::System;
use substrate_subxt_proc_macro::{module, Call, Event, Store};

/// The subset of the `pallet_robonomics_datalog::Trait` that a client must implement.
#[module]
pub trait Datalog: System {
    type Record: Codec + EncodeLike + Member + Default;
}

/// Send new data record into blockchain.
#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct RecordCall<T: Datalog> {
    record: T::Record,
}

/// New datalog record created.
#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct NewRecordEvent<T: Datalog> {
    /// Sender account.
    pub sender: <T as System>::AccountId,
    /// Inblock time stamp.
    pub timestamp: u64,
    /// Data record.
    pub record: T::Record,
}

/// Erease all stored data.
#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct EreaseCall<T: Datalog> {
    /// Runtime marker.
    pub _runtime: PhantomData<T>,
}

/// Account datalog storage ereased.
#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct ErasedEvent<T: Datalog> {
    pub sender: <T as System>::AccountId,
}

///
#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct DatalogStore<'a, T: Datalog> {
    #[store(returns = Vec<(u64, T::Record)>)]
    account_id: &'a <T as System>::AccountId,
}
