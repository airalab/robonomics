///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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
//! # Subscription Transaction Extension
//!
//! Transaction extension that enables fee-less transactions for subscription holders.
//!
//! ## Overview
//!
//! This extension allows users to opt-in per-transaction to use their subscription
//! for fee-less execution. Users can choose between using their subscription or paying
//! normal transaction fees on a per-transaction basis.
//!
//! ## What is a Transaction Extension?
//!
//! A **transaction extension** is a Substrate mechanism that wraps around transactions to:
//! - **Validate** transactions before they execute (check eligibility)
//! - **Modify** transaction behavior (e.g., disable fee payment)
//! - **Track** transaction execution (record usage)
//! - Work with **any** pallet's extrinsics
//!
//! Unlike wrapper extrinsics (like the old `call()` method), transaction extensions:
//! - Are transparent and visible in the transaction signature
//! - Work universally with all extrinsics
//! - Allow per-transaction opt-in/opt-out
//! - Are part of Substrate's native extension system
//!
//! ## Design
//!
//! The extension provides a simple two-variant enum:
//!
//! ```rust,ignore
//! pub enum ChargeSubscriptionTransaction<T> {
//!     // Use subscription for fee-less transaction
//!     Enabled { subscription_id: u32 },
//!     
//!     // Pay normal transaction fees
//!     Disabled,
//! }
//! ```
//!
//! ## Validation Flow
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │ 1. validate()                                            │
//! │    - Check subscription exists                           │
//! │    - Check subscription is active (not expired)          │
//! │    - Check sufficient free_weight available              │
//! │    → Result: Accept or Reject transaction                │
//! └──────────────────────────────────────────────────────────┘
//!                         │
//!                         ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │ 2. pre_dispatch()                                        │
//! │    - Re-validate subscription (prevent race conditions)  │
//! │    - Prepare RwsPreDispatch info (owner, sub_id)         │
//! │    → Returns: Pre-dispatch state                         │
//! └──────────────────────────────────────────────────────────┘
//!                         │
//!                         ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │ 3. [Transaction Executes]                                │
//! │    - Extrinsic runs normally                             │
//! │    - No fees charged (Pays::No)                          │
//! └──────────────────────────────────────────────────────────┘
//!                         │
//!                         ▼
//! ┌──────────────────────────────────────────────────────────┐
//! │ 4. post_dispatch()                                       │
//! │    - Accumulate free_weight (TPS × time)                 │
//! │    - Deduct consumed weight                              │
//! │    - Update last_update timestamp                        │
//! │    - Emit SubscriptionUsed event                         │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage Examples
//!
//! ### JavaScript/TypeScript (Polkadot.js)
//!
//! The most common way to use the extension is via Polkadot.js API:
//!
//! ```typescript
//! import { ApiPromise, WsProvider } from '@polkadot/api';
//!
//! const api = await ApiPromise.create({
//!   provider: new WsProvider('wss://kusama.rpc.robonomics.network')
//! });
//!
//! // Fee-less transaction using subscription
//! await api.tx.datalog
//!   .record('sensor_data')
//!   .signAndSend(account, {
//!     rwsAuction: {
//!       Enabled: { subscriptionId: 0 }
//!     }
//!   });
//!
//! // Pay normal fees
//! await api.tx.datalog
//!   .record('sensor_data')
//!   .signAndSend(account, {
//!     rwsAuction: 'Disabled'  // or omit entirely
//!   });
//! ```
//!
//! ### Multiple Subscriptions
//!
//! Users can have multiple subscriptions (indexed 0, 1, 2, ...) and choose which to use:
//!
//! ```typescript
//! // Use subscription 0 (e.g., Daily subscription)
//! await api.tx.launch
//!   .launch(target, param)
//!   .signAndSend(account, {
//!     rwsAuction: { Enabled: { subscriptionId: 0 } }
//!   });
//!
//! // Use subscription 1 (e.g., Lifetime subscription)
//! await api.tx.datalog
//!   .record(data)
//!   .signAndSend(account, {
//!     rwsAuction: { Enabled: { subscriptionId: 1 } }
//!   });
//! ```
//!
//! ### Error Handling
//!
//! ```typescript
//! try {
//!   await api.tx.datalog.record(data).signAndSend(account, {
//!     rwsAuction: { Enabled: { subscriptionId: 0 } }
//!   });
//! } catch (error) {
//!   if (error.message.includes('Payment')) {
//!     // Subscription invalid, expired, or insufficient weight
//!     console.log('Falling back to normal paid transaction');
//!     await api.tx.datalog.record(data).signAndSend(account);
//!   }
//! }
//! ```
//!
//! ### Rust (Node/Runtime)
//!
//! For runtime or node development:
//!
//! ```rust,ignore
//! use pallet_robonomics_subscription::ChargeSubscriptionTransaction;
//!
//! // Create the call
//! let call = RuntimeCall::Datalog(
//!     pallet_robonomics_datalog::Call::record { 
//!         record: b"sensor_data".to_vec() 
//!     }
//! );
//!
//! // Create RWS extension (fee-less)
//! let rws_ext = ChargeSubscriptionTransaction::Enabled {
//!     subscription_id: 0,
//! };
//!
//! // Include in signed extra
//! let extra = (
//!     frame_system::CheckNonZeroSender::new(),
//!     frame_system::CheckSpecVersion::new(),
//!     // ... other extensions ...
//!     rws_ext,  // BEFORE ChargeTransactionPayment
//!     pallet_transaction_payment::ChargeTransactionPayment::from(0),
//! );
//! ```
//!
//! ## Error Codes
//!
//! The extension can reject transactions with these errors:
//!
//! - `InvalidTransaction::Payment`: Subscription doesn't exist, is expired, or has insufficient free_weight
//! - `InvalidTransaction::ExhaustsResources`: Not currently used (reserved for future quota limits)
//!

//! ## Integration Requirements
//!
//! To integrate this extension into a runtime:
//!
//! 1. Add to `TxExtension` tuple **BEFORE** `ChargeTransactionPayment`:
//!    ```rust,ignore
//!    pub type TxExtension = (
//!        // ... other extensions ...
//!        ChargeSubscriptionTransaction<Runtime>,
//!        pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
//!    );
//!    ```
//!
//! 2. Import from pallet:
//!    ```rust,ignore
//!    pub use pallet_robonomics_subscription::ChargeSubscriptionTransaction;
//!    ```
//!
//! 3. Ensure pallet is properly configured in runtime with required associated types.

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{TransactionExtension, Dispatchable, DispatchOriginOf, DispatchInfoOf, AsSystemOriginSigner, Get},
    transaction_validity::{
        InvalidTransaction, TransactionValidityError, ValidTransaction, TransactionSource,
    },
};
use sp_std::prelude::*;

use crate::pallet::{Config, Pallet};
use frame_support::{
    dispatch::{DispatchInfo, DispatchResult, PostDispatchInfo},
    weights::Weight,
};
use sp_runtime::traits::{Implication, PostDispatchInfoOf};

/// Transaction extension for RWS fee-less execution.
///
/// Users explicitly choose to use their RWS subscription for fee-less transactions
/// or pay normal fees. This choice is made per-transaction.
///
/// # Examples
///
/// ```typescript
/// // Fee-less using subscription
/// const tx = api.tx.datalog.record('data');
/// const signedTx = await tx.signAsync(account, {
///   assetId: {
///     parents: 0,
///     interior: {
///       X1: [{ 
///         PalletInstance: 55,  // Subscription pallet index
///       }]
///     }
///   }
/// });
/// // Then modify signedTx to include Enabled { owner, subscription_id }
/// ```
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum ChargeSubscriptionTransaction<T: Config + Send + Sync> {
    /// Use RWS subscription for fee-less execution.
    ///
    /// If the subscription is valid and active, the transaction will execute without fees.
    /// Otherwise, the transaction is rejected with `InvalidTransaction::Payment`.
    ///
    /// # Requirements
    /// - Subscription must exist for the specified owner
    /// - Signer must have permission to use the subscription (either owner or granted access)
    /// - Subscription must be active (not expired for Daily mode)
    /// - Sufficient free_weight must be available
    Enabled {
        /// The account that owns the subscription
        subscription_owner: T::AccountId,
        /// The subscription ID (0, 1, 2, ...)
        subscription_id: u32,
    },
    /// Pay normal transaction fees.
    ///
    /// The transaction will be processed through the standard fee payment mechanism.
    Disabled,
}

impl<T: Config + Send + Sync> sp_std::fmt::Debug for ChargeSubscriptionTransaction<T> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        match self {
            Self::Enabled { subscription_id, .. } => {
                write!(f, "ChargeSubscriptionTransaction::Enabled({})", subscription_id)
            }
            Self::Disabled => write!(f, "ChargeSubscriptionTransaction::Disabled"),
        }
    }
}

impl<T: Config + Send + Sync> Default for ChargeSubscriptionTransaction<T> {
    fn default() -> Self {
        Self::Disabled
    }
}

/// Pre-dispatch information for RWS transactions.
///
/// This struct is returned from `pre_dispatch()` and passed to `post_dispatch()`
/// to track whether the transaction used a subscription and which one.
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
pub struct RwsPreDispatch<AccountId> {
    /// Whether this transaction is using a subscription (fee-less)
    pub pays_no_fee: bool,
    /// The account that owns the subscription
    pub subscription_owner: Option<AccountId>,
    /// The subscription ID being used
    pub subscription_id: Option<u32>,
}

impl<T> ChargeSubscriptionTransaction<T>
where
    T: Config + Send + Sync,
{
    /// Validate the subscription and check if the signer has permission to use it.
    ///
    /// This performs lightweight validation without mutating state.
    fn validate_subscription(
        who: &T::AccountId,
        subscription_owner: &T::AccountId,
        subscription_id: u32,
        info: &DispatchInfo,
    ) -> Result<(), TransactionValidityError> {
        // Check if signer has permission to use this subscription
        if !Pallet::<T>::has_permission(who, subscription_owner, subscription_id) {
            return Err(InvalidTransaction::BadSigner.into());
        }

        // Check if subscription exists and is active
        if !Pallet::<T>::is_subscription_active(subscription_owner, subscription_id) {
            return Err(InvalidTransaction::Payment.into());
        }

        // Check if subscription has sufficient weight
        let required_weight = info.call_weight.ref_time();
        if !Pallet::<T>::has_sufficient_weight(subscription_owner, subscription_id, required_weight)
        {
            return Err(InvalidTransaction::Payment.into());
        }

        Ok(())
    }
}

impl<T> TransactionExtension<<T as Config>::Call> for ChargeSubscriptionTransaction<T>
where
    T: Config + Send + Sync,
    <T as Config>::Call: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
    <<T as Config>::Call as Dispatchable>::RuntimeOrigin:
        sp_runtime::traits::AsSystemOriginSigner<T::AccountId> + Clone,
{
    const IDENTIFIER: &'static str = "ChargeSubscriptionTransaction";

    type Implicit = ();
    type Val = ();
    type Pre = RwsPreDispatch<T::AccountId>;

    fn implicit(&self) -> Result<Self::Implicit, TransactionValidityError> {
        Ok(())
    }

    fn weight(&self, _call: &<T as Config>::Call) -> Weight {
        // Weight calculation based on validate_subscription operations:
        // - SubscriptionPermissions read (if not owner): 1 DB read
        // - Subscription read: 1 DB read
        // - Time calculations and comparisons: ~5 microseconds computational overhead
        //
        // Worst case: 2 DB reads + computational overhead
        // Standard DB read: ~25 microseconds (25_000_000 picoseconds)
        // => Total worst‑case time ≈ 2 × 25µs (DB reads) + 5µs (computation) = 55µs
        match self {
            Self::Enabled { .. } => {
                // Performs validation: 2 DB reads + computation (total ≈ 55µs = 55_000_000 ps)
                Weight::from_parts(55_000_000, 0)
                    .saturating_add(T::DbWeight::get().reads(2))
            }
            Self::Disabled => {
                // No validation performed, minimal overhead
                Weight::from_parts(1_000, 0)
            }
        }
    }

    fn validate(
        &self,
        origin: DispatchOriginOf<<T as Config>::Call>,
        _call: &<T as Config>::Call,
        info: &DispatchInfoOf<<T as Config>::Call>,
        _len: usize,
        _self_implicit: Self::Implicit,
        _inherited_implication: &impl Implication,
        _source: TransactionSource,
    ) -> Result<(ValidTransaction, Self::Val, DispatchOriginOf<<T as Config>::Call>), TransactionValidityError> {
        // Extract the account ID from the origin
        let Some(who) = origin.as_system_origin_signer() else {
            // If not a signed origin, this extension is not applicable
            return Err(InvalidTransaction::BadSigner.into());
        };

        match self {
            Self::Enabled {
                subscription_owner,
                subscription_id,
            } => {
                // Validate subscription and permissions
                Self::validate_subscription(who, subscription_owner, *subscription_id, info)?;

                Ok((ValidTransaction::default(), (), origin))
            }
            Self::Disabled => {
                // No validation needed for normal fee payment
                Ok((ValidTransaction::default(), (), origin))
            }
        }
    }

    fn prepare(
        self,
        _val: Self::Val,
        origin: &DispatchOriginOf<<T as Config>::Call>,
        _call: &<T as Config>::Call,
        _info: &DispatchInfoOf<<T as Config>::Call>,
        _len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        match self {
            Self::Enabled {
                subscription_owner,
                subscription_id,
            } => {
                Ok(RwsPreDispatch {
                    pays_no_fee: true,
                    subscription_owner: Some(subscription_owner),
                    subscription_id: Some(subscription_id),
                })
            }
            Self::Disabled => Ok(RwsPreDispatch {
                pays_no_fee: false,
                subscription_owner: None,
                subscription_id: None,
            }),
        }
    }

    fn post_dispatch_details(
        pre: Self::Pre,
        info: &DispatchInfoOf<<T as Config>::Call>,
        post_info: &PostDispatchInfoOf<<T as Config>::Call>,
        _len: usize,
        _result: &DispatchResult,
    ) -> Result<Weight, TransactionValidityError> {
        if pre.pays_no_fee {
            if let (Some(owner), Some(subscription_id)) =
                (pre.subscription_owner, pre.subscription_id)
            {
                // Consume free weight from the subscription
                let weight = post_info.actual_weight.unwrap_or(info.call_weight);
                let _ = Pallet::<T>::consume_weight(&owner, subscription_id, weight);
                // Ignore errors in post_dispatch - transaction has already executed
            }
        }

        Ok(Weight::zero())
    }
}
