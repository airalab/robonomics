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
//! # Robonomics Subscription Governance Allocator
//!
//! A privileged allocation mechanism for the subscription pallet. A
//! configurable origin assigns an arbitrary normalized
//! [`SubscriptionAllocation`] to a beneficiary, which is useful for bootstrap
//! allocations, grants, testing and emergency provisioning.
//!
//! The pallet decides only *who* receives a subscription, *for how long* and
//! *with which resource limits*. It has no storage and talks to the
//! subscription pallet exclusively through the generic
//! [`SubscriptionAllocator`] interface, so it never computes generations,
//! timestamps, capacity admission or accounting state, and it depends on
//! neither CPS nor any resource consumer.
//!
//! Every successful allocation replaces the current subscription generation
//! of the beneficiary: nothing is topped up, extended, merged or rolled over.
//! Allocation failures, including capacity admission failures, are propagated
//! unchanged.

#![cfg_attr(not(feature = "std"), no_std)]

use polkadot_sdk::frame_support;

#[cfg(test)]
mod tests;
pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use super::WeightInfo;
    use polkadot_sdk::frame_support::pallet_prelude::*;
    use polkadot_sdk::frame_system::{self, pallet_prelude::*};
    use robonomics_subscription_primitives::{SubscriptionAllocation, SubscriptionAllocator};

    /// Allocation accepted by the configured [`Config::Allocator`].
    pub type AllocationOf<T> = SubscriptionAllocation<
        <<T as Config>::Allocator as SubscriptionAllocator<
            <T as frame_system::Config>::AccountId,
        >>::ResourceId,
        <<T as Config>::Allocator as SubscriptionAllocator<
            <T as frame_system::Config>::AccountId,
        >>::Limit,
        <<T as Config>::Allocator as SubscriptionAllocator<
            <T as frame_system::Config>::AccountId,
        >>::Moment,
        <<T as Config>::Allocator as SubscriptionAllocator<
            <T as frame_system::Config>::AccountId,
        >>::MaxResources,
    >;

    #[pallet::config]
    pub trait Config: polkadot_sdk::frame_system::Config {
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

        /// Origin allowed to allocate and revoke subscriptions.
        type AllocateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Subscription allocation backend.
        type Allocator: SubscriptionAllocator<
            Self::AccountId,
            ResourceId: Parameter,
            Limit: Parameter,
            Moment: Parameter,
            MaxResources: 'static,
        >;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A subscription was allocated by governance.
        Allocated { beneficiary: T::AccountId },
        /// A subscription was revoked by governance.
        Revoked { beneficiary: T::AccountId },
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Allocate `allocation` to `beneficiary`, replacing its current
        /// subscription generation.
        ///
        /// Fails with the allocator's error when the allocation is invalid or
        /// cannot be admitted, leaving no state change behind.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::allocate(allocation.resources.len() as u32))]
        pub fn allocate(
            origin: OriginFor<T>,
            beneficiary: T::AccountId,
            allocation: AllocationOf<T>,
        ) -> DispatchResult {
            T::AllocateOrigin::ensure_origin(origin)?;
            T::Allocator::allocate(&beneficiary, allocation)?;
            Self::deposit_event(Event::Allocated { beneficiary });
            Ok(())
        }

        /// Revoke the current subscription of `beneficiary`.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::revoke(
            <T::Allocator as SubscriptionAllocator<T::AccountId>>::MaxResources::get()
        ))]
        pub fn revoke(origin: OriginFor<T>, beneficiary: T::AccountId) -> DispatchResult {
            T::AllocateOrigin::ensure_origin(origin)?;
            T::Allocator::revoke(&beneficiary)?;
            Self::deposit_event(Event::Revoked { beneficiary });
            Ok(())
        }
    }
}
