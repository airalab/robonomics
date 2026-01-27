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
    traits::SignedExtension,
    transaction_validity::{
        InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransaction,
    },
};
use sp_std::prelude::*;

use crate::pallet::{Config, Pallet};
use frame_support::{
    dispatch::{DispatchInfo, DispatchResult, GetDispatchInfo, PostDispatchInfo},
    weights::Weight,
};

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

impl<T> SignedExtension for ChargeSubscriptionTransaction<T>
where
    T: Config + Send + Sync,
    <T as Config>::Call: GetDispatchInfo + sp_runtime::traits::Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
{
    type AccountId = T::AccountId;
    type Call = <T as Config>::Call;
    type AdditionalSigned = ();
    type Pre = RwsPreDispatch<T::AccountId>;

    const IDENTIFIER: &'static str = "ChargeSubscriptionTransaction";

    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        _call: &Self::Call,
        info: &DispatchInfo,
        _len: usize,
    ) -> TransactionValidity {
        match self {
            Self::Enabled {
                subscription_owner,
                subscription_id,
            } => {
                // Validate subscription and permissions
                Self::validate_subscription(who, subscription_owner, *subscription_id, info)?;

                Ok(ValidTransaction::default())
            }
            Self::Disabled => {
                // No validation needed for normal fee payment
                Ok(ValidTransaction::default())
            }
        }
    }

    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        info: &DispatchInfo,
        len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        // Re-validate to ensure nothing changed between validate and pre_dispatch
        self.validate(who, call, info, len)?;

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

    fn post_dispatch(
        pre: Option<Self::Pre>,
        info: &DispatchInfo,
        post_info: &PostDispatchInfo,
        _len: usize,
        _result: &DispatchResult,
    ) -> Result<(), TransactionValidityError> {
        if let Some(pre) = pre {
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
        }

        Ok(())
    }
}
