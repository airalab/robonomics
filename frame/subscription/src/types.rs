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
//! Types of `pallet-robonomics-subscription`.

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use robonomics_subscription_primitives::{Resource, ResourceCharge, ResourceGeneration};
use scale_info::TypeInfo;

/// Amount of a rate-accounted resource.
///
/// For Transaction this is a baseline cost in `Wn`, the smallest unit of XRT
/// (`1 XRT = 10^9 Wn`). A rate is expressed in `Wn` per millisecond of
/// runtime time.
pub type Amount = u128;

/// Current subscription of an owner.
///
/// The lifetime is expressed in runtime time (`Time::now()`), never in
/// blocks. The subscription is active while `now < expires_at`.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub struct Subscription<Moment> {
    /// Accounting epoch of this subscription.
    pub generation: ResourceGeneration,
    /// Time the subscription was allocated at.
    pub started_at: Moment,
    /// Time the subscription stops being active.
    pub expires_at: Moment,
}

/// Resource-specific entitlement carried by an allocation and by a local
/// limit.
///
/// The variants are not required to share a representation: new resources add
/// their own variant with their own semantics.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Debug,
)]
pub enum Limit {
    /// Transaction rate in baseline `Wn` per millisecond.
    #[codec(index = 0)]
    Transaction(Amount),
}

impl Limit {
    /// Resource this limit belongs to.
    pub fn resource(&self) -> Resource {
        match self {
            Limit::Transaction(_) => Resource::Transaction,
        }
    }
}

/// Amount of a resource consumed by one operation.
///
/// Deliberately a different type from [`Limit`]: a limit is a policy value,
/// a usage is the cost of one concrete operation.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Debug,
)]
pub enum Usage {
    /// Baseline cost of one transaction in `Wn`.
    #[codec(index = 0)]
    Transaction(Amount),
}

impl Usage {
    /// Resource this usage belongs to.
    pub fn resource(&self) -> Resource {
        match self {
            Usage::Transaction(_) => Resource::Transaction,
        }
    }
}

/// Token-bucket state of a rate-accounted resource.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub struct RateState<Moment> {
    /// Currently available amount.
    pub available: Amount,
    /// Time `available` was last brought up to date.
    pub updated_at: Moment,
}

/// Resource-specific accounting state.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub enum ResourceState<Moment> {
    /// Token bucket of a rate-based resource.
    #[codec(index = 0)]
    Rate(RateState<Moment>),
}

/// Accounting state bound to one subscription generation.
///
/// State whose `generation` differs from the current generation of the owner
/// is logically empty and is replaced on first use.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub struct GenerationState<Moment> {
    /// Generation the state belongs to.
    pub generation: ResourceGeneration,
    /// Resource-specific state.
    pub state: ResourceState<Moment>,
}

/// Opaque receipt of a successful consumption.
///
/// Only the pallet creates charges, and a charge binds a later correction to
/// the owner, generation, resource and accounting context it was made in.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub struct Charge<AccountId, LocalId>(
    pub(crate) ResourceCharge<AccountId, LocalId, Resource, ResourceGeneration, Usage>,
);

impl<AccountId, LocalId> Charge<AccountId, LocalId> {
    /// Generation the usage was charged in.
    pub fn generation(&self) -> ResourceGeneration {
        self.0.generation
    }

    /// Usage that was charged.
    pub fn charged(&self) -> Usage {
        self.0.charged
    }
}
