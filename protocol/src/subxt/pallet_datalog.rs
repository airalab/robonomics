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
    pub record: T::Record,
}

/// New datalog record created.
#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct NewRecordEvent<T: Datalog> {
    /// Sender account.
    pub sender: T::AccountId,
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
    pub sender: T::AccountId,
}

/// Deprecated!!!
#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct DatalogStore<'a, T: Datalog> {
    #[store(returns = Vec<(u64, T::Record)>)]
    account_id: &'a T::AccountId,
}

/// Datalog index type copy.
#[derive(Encode, Decode, Default)]
pub struct RingBufferIndex {
    #[codec(compact)]
    pub start: u64,
    #[codec(compact)]
    pub end: u64,
}

///
impl RingBufferIndex {
    #[inline]
    fn next(val: &mut u64, max: u64) {
        *val += 1;
        if *val == max {
            *val = 0
        }
    }

    /// Returns the ring buffer item iterator
    pub fn iter(&mut self, max: u64) -> RingBufferIterator<'_> {
        RingBufferIterator { inner: self, max }
    }
}

///
pub struct RingBufferIterator<'a> {
    inner: &'a mut RingBufferIndex,
    max: u64,
}

///
impl Iterator for RingBufferIterator<'_> {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.end == self.inner.start {
            None
        } else {
            let u = self.inner.start;
            RingBufferIndex::next(&mut self.inner.start, self.max);
            Some(u)
        }
    }
}

/// Get datalog index.
#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct DatalogIndexStore<'a, T: Datalog> {
    #[store(returns = RingBufferIndex)]
    account_id: &'a T::AccountId,
}

/// Datalog item type copy.
#[derive(Encode, Decode, Clone)]
pub struct RingBufferItem(#[codec(compact)] pub u64, pub Vec<u8>);

/// Get data records from blockchain.
#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct DatalogItemStore<'a, T: Datalog> {
    #[store(returns = RingBufferItem)]
    record: (&'a T::AccountId, u64),
}
