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
//! Types of `pallet-robonomics-subscription-purchase`.

use core::fmt::Debug;
use frame_support::{traits::Get, BoundedVec};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use robonomics_subscription_primitives::{ResourceAllocation, SubscriptionAllocation};
use scale_info::TypeInfo;

/// Identifier of a purchasable product.
pub type ProductId = u32;

/// Template of a subscription that can be purchased.
///
/// A product is only a template for a future allocation: it reserves no
/// network capacity, and active subscriptions never reference it.
#[derive(Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(MaxResources))]
pub struct SubscriptionProduct<Balance, Moment, ResourceId, Limit, MaxResources>
where
    MaxResources: Get<u32>,
{
    /// Price charged to the payer, in the payment asset.
    pub price: Balance,
    /// Validity duration of the purchased subscription.
    pub valid_for: Moment,
    /// Resource entitlements of the purchased subscription.
    pub resources: BoundedVec<ResourceAllocation<ResourceId, Limit>, MaxResources>,
    /// Whether the product can be purchased.
    pub enabled: bool,
}

impl<Balance, Moment: Clone, ResourceId: Clone, Limit: Clone, MaxResources: Get<u32>>
    SubscriptionProduct<Balance, Moment, ResourceId, Limit, MaxResources>
{
    /// Normalized allocation a purchase of this product requests.
    pub fn allocation(&self) -> SubscriptionAllocation<ResourceId, Limit, Moment, MaxResources> {
        SubscriptionAllocation {
            valid_for: self.valid_for.clone(),
            resources: self.resources.clone(),
        }
    }
}

// Implemented by hand so that `MaxResources`, a type-level bound that is only
// a marker, does not itself need to implement these traits.
impl<Balance: Clone, Moment: Clone, ResourceId: Clone, Limit: Clone, MaxResources: Get<u32>> Clone
    for SubscriptionProduct<Balance, Moment, ResourceId, Limit, MaxResources>
{
    fn clone(&self) -> Self {
        Self {
            price: self.price.clone(),
            valid_for: self.valid_for.clone(),
            resources: self.resources.clone(),
            enabled: self.enabled,
        }
    }
}

impl<
        Balance: PartialEq,
        Moment: PartialEq,
        ResourceId: PartialEq,
        Limit: PartialEq,
        MaxResources,
    > PartialEq for SubscriptionProduct<Balance, Moment, ResourceId, Limit, MaxResources>
where
    MaxResources: Get<u32>,
{
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
            && self.valid_for == other.valid_for
            && self.resources == other.resources
            && self.enabled == other.enabled
    }
}

impl<Balance: Eq, Moment: Eq, ResourceId: Eq, Limit: Eq, MaxResources: Get<u32>> Eq
    for SubscriptionProduct<Balance, Moment, ResourceId, Limit, MaxResources>
{
}

impl<Balance: Debug, Moment: Debug, ResourceId: Debug, Limit: Debug, MaxResources: Get<u32>> Debug
    for SubscriptionProduct<Balance, Moment, ResourceId, Limit, MaxResources>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SubscriptionProduct")
            .field("price", &self.price)
            .field("valid_for", &self.valid_for)
            .field("resources", &self.resources)
            .field("enabled", &self.enabled)
            .finish()
    }
}
