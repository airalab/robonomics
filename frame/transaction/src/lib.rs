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
//! # Robonomics Transaction Pallet
//!
//! The first consumer of Subscription resources: it routes the payment of a
//! transaction between the ordinary fee market and the `Transaction` resource
//! of a subscription.
//!
//! The pallet has no extrinsics and **no storage**. Its single entry point is
//! the [`ChargeTransaction`] transaction extension, which replaces
//! `pallet_transaction_payment::ChargeTransactionPayment` in the runtime
//! extension tuple. It owns:
//!
//! - the baseline transaction usage ([`Pallet::baseline_cost`]);
//! - routing between [`Payment::Fee`] and [`Payment::Subscription`];
//! - resource authorization and generation checks;
//! - pre-dispatch charging and post-dispatch correction;
//! - pool priority classes ([`priority`]).
//!
//! It does not own the subscription lifecycle, allocations, token-bucket
//! state, local limits, CPS topology, or allocators. Those live behind
//! [`ResourceAuthorization`] and [`ResourceAccounting`].
//!
//! ## Payment mode
//!
//! The mode is part of the signed transaction and never falls back silently:
//!
//! ```text
//! Fee { tip }                      -> pallet-transaction-payment, unchanged
//! Subscription { context, generation } -> Transaction resource of a subscription
//! ```
//!
//! ## Usage
//!
//! The Subscription usage is the baseline cost
//! `base fee + length fee + weight fee`. It excludes the congestion
//! multiplier and the tip, so identical execution costs identical usage under
//! any network load. The Transaction `Limit` is a rate interpreted by the
//! subscription pallet; this pallet only supplies the usage.
//!
//! ## Pipeline
//!
//! ```text
//! validate   read-only: authorize, check generation, check that the usage
//!            appears currently admissible, assign the pool priority
//! prepare    authoritative: authorize again, check the generation again and
//!            charge global (no local context) or local accounting
//! dispatch   the origin is unchanged: funding never grants the authority of
//!            the subscription owner
//! post       correct the charge to the actual usage through the opaque
//!            charge, which binds the correction to the original generation
//! ```
//!
//! Pool validation is advisory: many pending transactions may pass it while
//! competing for the same allowance, and only pre-dispatch decides.
//!
//! ## Scheduling
//!
//! See [`priority`]: Operational, then Subscription, then Fee. The classes
//! only order the pool; block weight, length and the Operational reserve stay
//! enforced by `CheckWeight`, so Fee transactions use all capacity that
//! Subscription transactions leave free.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod extension;
pub mod priority;
mod usage;
pub mod weights;

#[cfg(test)]
mod tests;

pub use extension::{ChargeTransaction, Payment, Pre, Val};
pub use pallet::*;
pub use weights::WeightInfo;

/// Native currency amount used by `pallet-transaction-payment`.
pub type BalanceOf<T> = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as pallet_transaction_payment::OnChargeTransaction<T>>::Balance;

/// Usage type of the configured accounting.
pub type UsageOf<T> =
    <<T as Config>::Accounting as robonomics_subscription_primitives::ResourceAccounting<
        <T as frame_system::Config>::AccountId,
    >>::Usage;

/// Charge receipt type of the configured accounting.
pub type ChargeOf<T> =
    <<T as Config>::Accounting as robonomics_subscription_primitives::ResourceAccounting<
        <T as frame_system::Config>::AccountId,
    >>::Charge;

/// Invalid-transaction code: the owner of the context has no active
/// subscription.
pub const NO_ACTIVE_SUBSCRIPTION: u8 = 1;

/// Invalid-transaction code: the signed generation is not the current
/// generation of the subscription.
pub const GENERATION_MISMATCH: u8 = 2;

/// Invalid-transaction code: the signed payment does not match the validated
/// state. Not expected to happen.
pub const PAYMENT_MISMATCH: u8 = 3;

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use robonomics_subscription_primitives::{
        Resource, ResourceAccounting, ResourceAuthorization, ResourceGeneration,
    };
    use sp_runtime::traits::Convert;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Authorization context signed by the user, for example
        /// `AuthorizationContext<AccountId, NodeId>`.
        type ContextId: Parameter + MaxEncodedLen + Send + Sync;

        /// Local accounting context, for example a CPS `ScopeId`.
        type LocalId: Parameter + MaxEncodedLen;

        /// Decides who may use the Transaction resource in a context.
        type Authorization: ResourceAuthorization<
            Self::AccountId,
            ContextId = Self::ContextId,
            LocalId = Self::LocalId,
            ResourceId = Resource,
        >;

        /// Accounting of the Transaction resource.
        type Accounting: ResourceAccounting<
            Self::AccountId,
            LocalId = Self::LocalId,
            ResourceId = Resource,
            Generation = ResourceGeneration,
        >;

        /// Converts the baseline cost in the native currency to the usage
        /// of the accounting.
        type ToUsage: Convert<BalanceOf<Self>, UsageOf<Self>>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A transaction was funded by a subscription.
        SubscriptionTransactionPaid {
            /// Signer of the transaction.
            who: T::AccountId,
            /// Owner of the subscription that funded it.
            owner: T::AccountId,
            /// Local accounting context, if one was used.
            local_id: Option<T::LocalId>,
            /// Generation the usage was charged in.
            generation: ResourceGeneration,
            /// Actual baseline cost of the transaction.
            usage: BalanceOf<T>,
        },
    }
}
