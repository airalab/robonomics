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
//! Transaction-pool priority classes.
//!
//! `TransactionPriority` is a `u64` that the pool orders descending. It is
//! split into disjoint bands so that the payment mode, not the tip, decides
//! the scheduling class:
//!
//! ```text
//! Operational   [2^63, u64::MAX]   any payment mode
//! Subscription  [2^62, 2^63)       Normal class, Subscription payment
//! Fee           [0,    2^62)       Normal class, Fee payment
//! ```
//!
//! A Fee transaction priority is clamped to its band, so no tip can cross the
//! Subscription boundary. Inside the Subscription band the order is
//! deterministic and independent from any tip: a transaction with a smaller
//! baseline usage ranks higher.
//!
//! The bands assume that the other transaction extensions of the runtime add
//! no priority (`CheckNonce` and `CheckWeight` contribute zero).

use frame_support::{dispatch::DispatchClass, pallet_prelude::TransactionPriority};

/// Size of the Fee and of the Subscription band.
pub const BAND_SIZE: TransactionPriority = 1 << 62;

/// Lowest priority of the Subscription band.
pub const SUBSCRIPTION_BASE: TransactionPriority = BAND_SIZE;

/// Lowest priority of the Operational band.
pub const OPERATIONAL_BASE: TransactionPriority = 1 << 63;

/// Priority of a Fee-mode transaction given the priority computed by
/// `pallet-transaction-payment` (which includes the tip).
pub fn fee_priority(class: DispatchClass, priority: TransactionPriority) -> TransactionPriority {
    match class {
        DispatchClass::Operational => {
            OPERATIONAL_BASE.saturating_add(priority.min(OPERATIONAL_BASE - 1))
        }
        DispatchClass::Normal | DispatchClass::Mandatory => priority.min(BAND_SIZE - 1),
    }
}

/// Priority of a Subscription-mode transaction with baseline `usage`.
///
/// Smaller usage ranks higher inside the band.
pub fn subscription_priority(class: DispatchClass, usage: u128) -> TransactionPriority {
    let rank = (BAND_SIZE - 1) - usage.min((BAND_SIZE - 1) as u128) as TransactionPriority;
    let base = match class {
        DispatchClass::Operational => OPERATIONAL_BASE,
        DispatchClass::Normal | DispatchClass::Mandatory => SUBSCRIPTION_BASE,
    };
    base + rank
}
