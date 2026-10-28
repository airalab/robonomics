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
//! The `ChargeTransaction` transaction extension.

use crate::weights::WeightInfo as _;
use crate::{
    priority, BalanceOf, ChargeOf, Config, Event, Pallet, GENERATION_MISMATCH,
    NO_ACTIVE_SUBSCRIPTION, PAYMENT_MISMATCH,
};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use polkadot_sdk::frame_support::{
    dispatch::{DispatchInfo, PostDispatchInfo},
    weights::Weight,
};
use polkadot_sdk::frame_system;
use polkadot_sdk::pallet_transaction_payment::{self, ChargeTransactionPayment};
use polkadot_sdk::sp_runtime::{
    traits::{
        Convert, DispatchInfoOf, DispatchOriginOf, Dispatchable, Implication, PostDispatchInfoOf,
        TransactionExtension, UniqueSaturatedInto,
    },
    transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidityError, ValidTransaction,
    },
    DispatchResult,
};
use robonomics_subscription_primitives::{
    Resource, ResourceAccounting, ResourceAuthorization, ResourceContext, ResourceGeneration,
};
use scale_info::TypeInfo;

/// How a transaction pays for its execution.
///
/// The mode is part of the signed payload: there is no silent fallback
/// between the two modes.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, Eq, PartialEq, Debug)]
pub enum Payment<ContextId, Generation, Balance> {
    /// Ordinary fee market: baseline cost times the congestion multiplier,
    /// plus `tip`, withdrawn from the signer.
    #[codec(index = 0)]
    Fee {
        /// Tip for the block author.
        #[codec(compact)]
        tip: Balance,
    },
    /// The `Transaction` resource of a subscription, accounted by the
    /// baseline cost only.
    #[codec(index = 1)]
    Subscription {
        /// Authorization context the signer acts in.
        context: ContextId,
        /// Subscription generation the transaction was prepared against.
        generation: Generation,
    },
}

/// Payment routing extension, a replacement of
/// `pallet_transaction_payment::ChargeTransactionPayment`.
///
/// [`Payment::Fee`] is handled by `ChargeTransactionPayment` itself and only
/// has its pool priority moved into the Fee band. [`Payment::Subscription`]
/// charges the `Transaction` resource of the subscription that the
/// authorization provider resolves for the signed context.
///
/// The dispatch origin is never changed.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, Eq, PartialEq)]
#[scale_info(skip_type_params(T))]
pub struct ChargeTransaction<T: Config>(
    pub Payment<T::ContextId, ResourceGeneration, BalanceOf<T>>,
);

impl<T: Config> ChargeTransaction<T> {
    /// Pay with fees and `tip`.
    pub fn fee(tip: BalanceOf<T>) -> Self {
        Self(Payment::Fee { tip })
    }

    /// Pay with the subscription visible in `context` at `generation`.
    pub fn subscription(context: T::ContextId, generation: ResourceGeneration) -> Self {
        Self(Payment::Subscription {
            context,
            generation,
        })
    }
}

impl<T: Config> Default for ChargeTransaction<T> {
    fn default() -> Self {
        Self::fee(Default::default())
    }
}

impl<T: Config> core::fmt::Debug for ChargeTransaction<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "ChargeTransaction({:?})", self.0)
    }
}

/// Data passed from `validate` to `prepare`.
pub enum Val<T: Config> {
    /// Value of `ChargeTransactionPayment`.
    Fee(pallet_transaction_payment::Val<T>),
    /// A validated Subscription payment of `who`.
    Subscription { who: T::AccountId },
}

/// Data passed from `prepare` to `post_dispatch`.
pub enum Pre<T: Config> {
    /// Value of `ChargeTransactionPayment`.
    Fee(pallet_transaction_payment::Pre<T>),
    /// A charged Subscription payment.
    Subscription {
        /// Signer of the transaction.
        who: T::AccountId,
        /// Owner of the charged subscription.
        owner: T::AccountId,
        /// Local accounting context, if one was charged.
        local_id: Option<T::LocalId>,
        /// Generation that was charged.
        generation: ResourceGeneration,
        /// Opaque receipt that binds the correction to `generation`.
        charge: ChargeOf<T>,
    },
}

impl<T: Config> Pallet<T> {
    /// Authorize `who` in `context` and check that the owner has an active
    /// subscription of the signed `generation`.
    ///
    /// Shared by validation and pre-dispatch so that both apply the same
    /// rules.
    fn authorize_subscription(
        who: &T::AccountId,
        context: &T::ContextId,
        generation: ResourceGeneration,
    ) -> Result<ResourceContext<T::AccountId, T::LocalId>, TransactionValidityError> {
        let resolved = T::Authorization::authorize(who, context, Resource::Transaction)
            .map_err(|_| InvalidTransaction::BadSigner)?;
        match T::Accounting::generation(&resolved.owner) {
            None => Err(InvalidTransaction::Custom(NO_ACTIVE_SUBSCRIPTION).into()),
            Some(current) if current != generation => {
                Err(InvalidTransaction::Custom(GENERATION_MISMATCH).into())
            }
            Some(_) => Ok(resolved),
        }
    }
}

impl<T: Config> TransactionExtension<T::RuntimeCall> for ChargeTransaction<T>
where
    T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
{
    const IDENTIFIER: &'static str = "ChargeTransaction";
    type Implicit = ();
    type Val = Val<T>;
    type Pre = Pre<T>;

    fn implicit(&self) -> Result<Self::Implicit, TransactionValidityError> {
        Ok(())
    }

    fn weight(&self, call: &T::RuntimeCall) -> Weight {
        match &self.0 {
            Payment::Fee { tip } => ChargeTransactionPayment::<T>::from(*tip).weight(call),
            Payment::Subscription { .. } => <T as Config>::WeightInfo::subscription_charge(),
        }
    }

    fn validate(
        &self,
        origin: DispatchOriginOf<T::RuntimeCall>,
        call: &T::RuntimeCall,
        info: &DispatchInfoOf<T::RuntimeCall>,
        len: usize,
        _self_implicit: Self::Implicit,
        inherited_implication: &impl Implication,
        source: TransactionSource,
    ) -> Result<
        (
            ValidTransaction,
            Self::Val,
            DispatchOriginOf<T::RuntimeCall>,
        ),
        TransactionValidityError,
    > {
        match &self.0 {
            Payment::Fee { tip } => {
                let (mut valid, val, origin) = ChargeTransactionPayment::<T>::from(*tip).validate(
                    origin,
                    call,
                    info,
                    len,
                    (),
                    inherited_implication,
                    source,
                )?;
                valid.priority = priority::fee_priority(info.class, valid.priority);
                Ok((valid, Val::Fee(val), origin))
            }
            Payment::Subscription {
                context,
                generation,
            } => {
                let who = frame_system::ensure_signed(origin.clone())
                    .map_err(|_| InvalidTransaction::BadSigner)?;
                let cost = Pallet::<T>::estimated_cost(len, info);
                let resolved = Pallet::<T>::authorize_subscription(&who, context, *generation)?;

                // Advisory only: the authoritative charge happens in `prepare`.
                let usage = T::ToUsage::convert(cost);
                match &resolved.local_id {
                    None => T::Accounting::can_consume_global(
                        &resolved.owner,
                        *generation,
                        Resource::Transaction,
                        usage,
                    ),
                    Some(local) => T::Accounting::can_consume_local(
                        &resolved.owner,
                        local,
                        *generation,
                        Resource::Transaction,
                        usage,
                    ),
                }
                .map_err(|_| InvalidTransaction::Payment)?;

                let valid = ValidTransaction {
                    priority: priority::subscription_priority(
                        info.class,
                        cost.unique_saturated_into(),
                    ),
                    ..Default::default()
                };
                Ok((valid, Val::Subscription { who }, origin))
            }
        }
    }

    fn prepare(
        self,
        val: Self::Val,
        origin: &DispatchOriginOf<T::RuntimeCall>,
        call: &T::RuntimeCall,
        info: &DispatchInfoOf<T::RuntimeCall>,
        len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {
        match (self.0, val) {
            (Payment::Fee { tip }, Val::Fee(val)) => ChargeTransactionPayment::<T>::from(tip)
                .prepare(val, origin, call, info, len)
                .map(Pre::Fee),
            (
                Payment::Subscription {
                    context,
                    generation,
                },
                Val::Subscription { who },
            ) => {
                // Repeat the authoritative checks: state may have changed
                // since validation.
                let resolved = Pallet::<T>::authorize_subscription(&who, &context, generation)?;
                let usage = T::ToUsage::convert(Pallet::<T>::estimated_cost(len, info));
                let charge = match &resolved.local_id {
                    None => T::Accounting::consume_global(
                        &resolved.owner,
                        generation,
                        Resource::Transaction,
                        usage,
                    ),
                    Some(local) => T::Accounting::consume_local(
                        &resolved.owner,
                        local,
                        generation,
                        Resource::Transaction,
                        usage,
                    ),
                }
                .map_err(|_| InvalidTransaction::Payment)?;

                Ok(Pre::Subscription {
                    who,
                    owner: resolved.owner,
                    local_id: resolved.local_id,
                    generation,
                    charge,
                })
            }
            _ => Err(InvalidTransaction::Custom(PAYMENT_MISMATCH).into()),
        }
    }

    fn post_dispatch_details(
        pre: Self::Pre,
        info: &DispatchInfoOf<T::RuntimeCall>,
        post_info: &PostDispatchInfoOf<T::RuntimeCall>,
        len: usize,
        result: &DispatchResult,
    ) -> Result<Weight, TransactionValidityError> {
        match pre {
            Pre::Fee(pre) => <ChargeTransactionPayment<T> as TransactionExtension<
                T::RuntimeCall,
            >>::post_dispatch_details(
                pre, info, post_info, len, result
            ),
            Pre::Subscription {
                who,
                owner,
                local_id,
                generation,
                charge,
            } => {
                let actual = Pallet::<T>::actual_cost(len, info, post_info);
                // The call has already been dispatched, so a failed
                // correction (for example the call started a new generation,
                // which the charge must never credit) leaves the estimate
                // charged instead of invalidating the transaction.
                let _ = T::Accounting::correct(charge, T::ToUsage::convert(actual));
                Pallet::<T>::deposit_event(Event::SubscriptionTransactionPaid {
                    who,
                    owner,
                    local_id,
                    generation,
                    usage: actual,
                });
                Ok(Weight::zero())
            }
        }
    }
}

impl<T: Config> core::fmt::Debug for Val<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Val::Fee(val) => write!(f, "Val::Fee({val:?})"),
            Val::Subscription { who } => write!(f, "Val::Subscription({who:?})"),
        }
    }
}

impl<T: Config> core::fmt::Debug for Pre<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Pre::Fee(pre) => write!(f, "Pre::Fee({pre:?})"),
            Pre::Subscription {
                who,
                owner,
                local_id,
                generation,
                ..
            } => write!(
                f,
                "Pre::Subscription {{ who: {who:?}, owner: {owner:?}, local_id: {local_id:?}, generation: {generation} }}"
            ),
        }
    }
}
