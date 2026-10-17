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
//! Resource accounting interface for resource consumers.

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{DispatchError, DispatchResult};

/// Which accounting state a charge was applied to.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub enum ChargeSource<LocalId> {
    /// Global (direct) accounting only.
    Global,
    /// Global and local accounting in the given local context.
    Local(LocalId),
}

/// Reference shape of a charge receipt.
///
/// `ResourceAccounting::Charge` is opaque and implementations may use a
/// private representation. Whatever the representation, it must bind a
/// correction to the original accounting epoch: a correction of generation
/// `N` must never credit generation `N + 1`.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub struct ResourceCharge<AccountId, LocalId, ResourceId, Generation, Usage> {
    /// Owner of the charged subscription.
    pub owner: AccountId,
    /// Generation the usage was charged in.
    pub generation: Generation,
    /// Charged resource.
    pub resource: ResourceId,
    /// Accounting state the usage was charged to.
    pub source: ChargeSource<LocalId>,
    /// Usage that was charged.
    pub charged: Usage,
}

/// Narrow accounting interface for resource consumers.
///
/// Consumers provide only the `Usage` of an operation. They never provide
/// the local `Limit`: for local accounting the implementation obtains that
/// policy through `ResourceLimit`. `Usage` is resource-specific and need not
/// equal the `Limit` type.
pub trait ResourceAccounting<AccountId> {
    /// Local accounting and resource-limit context identifier.
    type LocalId;
    /// Resource identifier.
    type ResourceId;
    /// Resource-specific usage of one operation.
    type Usage;
    /// Accounting epoch identifier.
    type Generation;
    /// Opaque receipt sufficient for a safe [`Self::correct`].
    type Charge;

    /// Current generation of the subscription of `owner`, if it has one.
    fn generation(owner: &AccountId) -> Option<Self::Generation>;

    /// Charge `usage` to the global accounting of `owner`.
    ///
    /// Fails if `generation` is not the current generation.
    fn consume_global(
        owner: &AccountId,
        generation: Self::Generation,
        resource: Self::ResourceId,
        usage: Self::Usage,
    ) -> Result<Self::Charge, DispatchError>;

    /// Charge `usage` to both the local and the global accounting of `owner`.
    ///
    /// Fails if `generation` is not the current generation. The receipt keeps
    /// enough context to correct global and local state consistently.
    fn consume_local(
        owner: &AccountId,
        local: &Self::LocalId,
        generation: Self::Generation,
        resource: Self::ResourceId,
        usage: Self::Usage,
    ) -> Result<Self::Charge, DispatchError>;

    /// Replace the charged usage with the `actual_usage` measured after the
    /// operation.
    ///
    /// The correction applies to the generation recorded in `charge` and must
    /// never affect a later generation.
    fn correct(charge: Self::Charge, actual_usage: Self::Usage) -> DispatchResult;
}
