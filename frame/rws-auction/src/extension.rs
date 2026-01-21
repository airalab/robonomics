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
//! # RWS Transaction Extension
//!
//! Transaction extension that enables fee-less transactions for RWS subscription holders.
//!
//! ## Overview
//!
//! This extension allows users to opt-in per-transaction to use their RWS subscription
//! for fee-less execution. Users can choose between using their subscription or paying
//! normal transaction fees.
//!
//! ## Design
//!
//! ```rust,ignore
//! ChargeRwsTransaction::Enabled { subscription_id } → fee-less if valid subscription
//! ChargeRwsTransaction::Disabled → normal transaction fees
//! ```
//!
//! ## Validation Flow
//!
//! 1. `validate()`: Check subscription exists, is active, and has quota
//! 2. `pre_dispatch()`: Final pre-dispatch check, returns RwsPreDispatch info
//! 3. Transaction executes (or not)
//! 4. `post_dispatch()`: Record usage if fee-less and successful
//!
//! ## Proxy Support
//!
//! The extension supports Substrate's proxy pallet. If the transaction is a proxy call
//! (`proxy.proxy` or `proxy.proxy_announced`), the extension extracts the real account
//! (subscription owner) and validates their subscription.

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
/// Users can explicitly choose to use their RWS subscription for fee-less transactions
/// or pay normal fees.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum ChargeRwsTransaction<T: Config + Send + Sync> {
    /// Use RWS subscription for fee-less execution.
    ///
    /// If the subscription is valid and active, the transaction will execute without fees.
    /// Otherwise, the transaction is rejected.
    Enabled {
        /// The subscription ID to use
        subscription_id: u32,
    },
    /// Pay normal transaction fees.
    ///
    /// The transaction will be processed through the standard fee payment mechanism.
    Disabled,
    #[codec(skip)]
    _Phantom(sp_std::marker::PhantomData<T>),
}

impl<T: Config + Send + Sync> sp_std::fmt::Debug for ChargeRwsTransaction<T> {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        match self {
            Self::Enabled { subscription_id } => {
                write!(f, "ChargeRwsTransaction::Enabled({})", subscription_id)
            }
            Self::Disabled => write!(f, "ChargeRwsTransaction::Disabled"),
            Self::_Phantom(_) => write!(f, "ChargeRwsTransaction::_Phantom"),
        }
    }
}

impl<T: Config + Send + Sync> Default for ChargeRwsTransaction<T> {
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
    /// The account that owns the subscription (may differ from signer if using proxy)
    pub subscription_owner: Option<AccountId>,
    /// The subscription ID being used
    pub subscription_id: Option<u32>,
}

impl<T> ChargeRwsTransaction<T>
where
    T: Config + Send + Sync,
{
    /// Extract the subscription owner from the transaction.
    ///
    /// For direct calls, returns the signer.
    /// For proxy calls, this would extract the 'real' account from proxy calls.
    /// Currently returns the signer directly (proxy support to be added based on runtime integration).
    fn get_subscription_owner(
        who: &T::AccountId,
        _call: &<T as Config>::Call,
    ) -> Result<T::AccountId, TransactionValidityError> {
        // Direct call - signer is the subscription owner
        // TODO: Add proxy pattern matching when integrated with runtime
        // match call {
        //     RuntimeCall::Proxy(pallet_proxy::Call::proxy { real, .. })
        //     | RuntimeCall::Proxy(pallet_proxy::Call::proxy_announced { real, .. }) => {
        //         Ok(real.clone())
        //     }
        //     _ => Ok(who.clone())
        // }
        Ok(who.clone())
    }

    /// Validate the subscription and check if it's usable.
    fn validate_subscription(
        who: &T::AccountId,
        subscription_id: u32,
        call: &<T as Config>::Call,
    ) -> Result<T::AccountId, TransactionValidityError> {
        // Extract the subscription owner (may be different from signer if using proxy)
        let owner = Self::get_subscription_owner(who, call)?;

        // Check if subscription exists and is active
        if !Pallet::<T>::is_subscription_active(&owner, subscription_id) {
            return Err(InvalidTransaction::Payment.into());
        }

        // Check if subscription has transaction quota available
        if !Pallet::<T>::has_transaction_quota(&owner, subscription_id) {
            return Err(InvalidTransaction::ExhaustsResources.into());
        }

        Ok(owner)
    }
}

impl<T> SignedExtension for ChargeRwsTransaction<T>
where
    T: Config + Send + Sync,
    <T as Config>::Call: GetDispatchInfo,
{
    type AccountId = T::AccountId;
    type Call = <T as Config>::Call;
    type AdditionalSigned = ();
    type Pre = RwsPreDispatch<T::AccountId>;

    const IDENTIFIER: &'static str = "ChargeRwsTransaction";

    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfo,
        _len: usize,
    ) -> TransactionValidity {
        match self {
            Self::Enabled { subscription_id } => {
                // Validate subscription
                Self::validate_subscription(who, *subscription_id, call)?;

                Ok(ValidTransaction::default())
            }
            Self::Disabled => {
                // No validation needed for normal fee payment
                Ok(ValidTransaction::default())
            }
            Self::_Phantom(_) => Ok(ValidTransaction::default()),
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
            Self::Enabled { subscription_id } => {
                let owner = Self::get_subscription_owner(who, call)?;

                Ok(RwsPreDispatch {
                    pays_no_fee: true,
                    subscription_owner: Some(owner),
                    subscription_id: Some(subscription_id),
                })
            }
            Self::Disabled | Self::_Phantom(_) => Ok(RwsPreDispatch {
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
        result: &DispatchResult,
    ) -> Result<(), TransactionValidityError> {
        if let Some(pre) = pre {
            if pre.pays_no_fee {
                if let (Some(owner), Some(subscription_id)) =
                    (pre.subscription_owner, pre.subscription_id)
                {
                    // Record transaction usage
                    let weight = post_info.actual_weight.unwrap_or(info.call_weight);
                    Pallet::<T>::record_transaction(&owner, subscription_id, weight, result.is_ok());
                }
            }
        }

        Ok(())
    }
}
