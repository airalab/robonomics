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
//! # Subscription Primitives
//!
//! Shared types and traits that connect independently developed components
//! without direct pallet-to-pallet dependencies:
//!
//! ```text
//! robonomics-subscription-primitives
//!     -> authorization providers   (ResourceAuthorization, ResourceLimit)
//!     -> pallet-subscription       (ResourceAccounting, SubscriptionAllocator)
//!     -> resource consumers        (consume authorization and accounting)
//!     -> subscription allocators   (consume SubscriptionAllocator)
//! ```
//!
//! The crate defines interfaces only. It has no storage and no pallet logic,
//! and it must not depend on CPS, the subscription pallet, the transaction
//! pallet, or any allocator.
//!
//! ## Core invariants
//!
//! - [`Resource`] identifiers are stable: their SCALE indexes never change.
//! - A resource `Limit` (entitlement or policy boundary) and a resource
//!   `Usage` (what one operation consumes) are distinct associated types.
//!   Nothing here assumes `Limit == Usage` or a rate-based model.
//! - `ContextId` (authorization context) and `LocalId` (local accounting
//!   context) are distinct. Node identity is an authorization address only
//!   and never becomes a `LocalId`.
//! - Authorization ([`ResourceAuthorization`]) and limit lookup
//!   ([`ResourceLimit`]) are separate operations.
//! - Consumers supply only `Usage`; the applicable local limit is obtained by
//!   the accounting implementation through [`ResourceLimit`].
//! - A charge receipt binds a correction to the generation it was charged in.
//! - Admission queries (`can_consume_*`) are read-only; consumption stays
//!   authoritative.

#![cfg_attr(not(feature = "std"), no_std)]

mod accounting;
mod allocation;
mod authorization;
mod resource;

pub use accounting::{ChargeSource, ResourceAccounting, ResourceCharge};
pub use allocation::{ResourceAllocation, SubscriptionAllocation, SubscriptionAllocator};
pub use authorization::{
    AuthorizationContext, ResourceAuthorization, ResourceContext, ResourceLimit,
};
pub use resource::{Resource, ResourceGeneration};

#[cfg(test)]
mod tests;
