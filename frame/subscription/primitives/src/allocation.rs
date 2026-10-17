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
//! Normalized subscription allocation payload and allocator interface.

use core::fmt::Debug;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{traits::Get, BoundedVec, DispatchResult};

/// Entitlement to one resource.
///
/// The meaning of `limit` is resource-specific; for Transaction it is a rate
/// in baseline XRT per second.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub struct ResourceAllocation<ResourceId, Limit> {
    /// Resource the limit applies to.
    pub resource: ResourceId,
    /// Requested resource-specific limit.
    pub limit: Limit,
}

/// Requested resource entitlements, independent of subscription storage.
///
/// The payload carries entitlements only. It must not carry accounting state
/// (generation, start or expiry time, consumed amounts, token-bucket state),
/// allocator identity, or authorization data: the subscription pallet owns
/// those.
#[derive(Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(MaxResources))]
pub struct SubscriptionAllocation<ResourceId, Limit, Moment, MaxResources>
where
    MaxResources: Get<u32>,
{
    /// Validity duration in the runtime clock unit.
    pub valid_for: Moment,
    /// Resource entitlements, bounded by `MaxResources`.
    pub resources: BoundedVec<ResourceAllocation<ResourceId, Limit>, MaxResources>,
}

// Implemented by hand so that `MaxResources`, a type-level bound that is only
// a marker, does not itself need to implement these traits.
impl<ResourceId: Clone, Limit: Clone, Moment: Clone, MaxResources: Get<u32>> Clone
    for SubscriptionAllocation<ResourceId, Limit, Moment, MaxResources>
{
    fn clone(&self) -> Self {
        Self {
            valid_for: self.valid_for.clone(),
            resources: self.resources.clone(),
        }
    }
}

impl<ResourceId: PartialEq, Limit: PartialEq, Moment: PartialEq, MaxResources: Get<u32>> PartialEq
    for SubscriptionAllocation<ResourceId, Limit, Moment, MaxResources>
{
    fn eq(&self, other: &Self) -> bool {
        self.valid_for == other.valid_for && self.resources == other.resources
    }
}

impl<ResourceId: Eq, Limit: Eq, Moment: Eq, MaxResources: Get<u32>> Eq
    for SubscriptionAllocation<ResourceId, Limit, Moment, MaxResources>
{
}

impl<ResourceId: Debug, Limit: Debug, Moment: Debug, MaxResources: Get<u32>> Debug
    for SubscriptionAllocation<ResourceId, Limit, Moment, MaxResources>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SubscriptionAllocation")
            .field("valid_for", &self.valid_for)
            .field("resources", &self.resources)
            .finish()
    }
}

/// Interface through which external allocation mechanisms (governance,
/// purchase, auction, ...) create a new subscription generation.
///
/// Allocators request resource limits and never touch subscription storage.
/// The subscription pallet decides generation, timestamps, capacity
/// admission, and accounting state.
pub trait SubscriptionAllocator<AccountId> {
    /// Resource identifier.
    type ResourceId;
    /// Resource-specific limit.
    type Limit;
    /// Runtime clock unit.
    type Moment;
    /// Maximum number of resources in one allocation.
    type MaxResources: Get<u32>;

    /// Start a new subscription generation for `owner`.
    fn allocate(
        owner: &AccountId,
        allocation: SubscriptionAllocation<
            Self::ResourceId,
            Self::Limit,
            Self::Moment,
            Self::MaxResources,
        >,
    ) -> DispatchResult;

    /// Revoke the current subscription of `owner`.
    fn revoke(owner: &AccountId) -> DispatchResult;
}
