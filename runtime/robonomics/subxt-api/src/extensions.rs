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
//! Transaction extensions of the Robonomics runtime.
//!
//! The runtime replaces `ChargeTransactionPayment` with `ChargeTransaction`,
//! whose signed value selects how the transaction is paid:
//!
//! ```text
//! Fee { tip }                              ordinary fee market
//! Subscription { context, generation }     Transaction resource of a subscription
//! ```
//!
//! Subxt has no typed support for `ChargeTransaction`, so it is provided as a
//! custom extension. [`RobonomicsTransactionExtensions`] adds the default
//! payment, `Fee { tip: 0 }`, when no payment is given, so
//! `sign_and_submit_default` keeps working. Use
//! [`ExtrinsicParamsBuilderExt`] to choose the payment explicitly.

use scale_info::PortableRegistry;
use subxt::{
    config::{
        ClientState, Config, DefaultExtrinsicParams, DefaultExtrinsicParamsBuilder,
        DefaultTransactionExtensions, TransactionExtensions,
    },
    error::TransactionExtensionError,
    ext::{
        frame_decode::extrinsics::TransactionExtensionsError, scale_encode::EncodeAsType,
        scale_value::Value,
    },
    utils::AccountId32,
};

/// Identifier of the payment routing extension in the runtime metadata.
pub const CHARGE_TRANSACTION: &str = "ChargeTransaction";

/// Authorization context of a Subscription payment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthorizationContext {
    /// The signer uses the subscription of this account.
    Account(AccountId32),
    /// The signer acts in the context of a CPS node.
    Node(u64),
    /// The signer acts in the context of the CPS node that this account is
    /// the identity of.
    NodeIdentity(AccountId32),
}

impl From<AuthorizationContext> for Value {
    fn from(context: AuthorizationContext) -> Self {
        let account = |account: AccountId32| Value::from_bytes(account.0);
        match context {
            AuthorizationContext::Account(id) => Value::unnamed_variant("Account", [account(id)]),
            AuthorizationContext::Node(id) => {
                Value::unnamed_variant("Node", [Value::unnamed_composite([Value::u128(id.into())])])
            }
            AuthorizationContext::NodeIdentity(id) => {
                Value::unnamed_variant("NodeIdentity", [account(id)])
            }
        }
    }
}

/// How a transaction pays for its execution.
///
/// The choice is signed with the transaction: there is no fallback between
/// the two modes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payment {
    /// Ordinary fee market with an optional tip.
    Fee {
        /// Tip for the block author.
        tip: u128,
    },
    /// The `Transaction` resource of a subscription.
    Subscription {
        /// Authorization context the signer acts in.
        context: AuthorizationContext,
        /// Subscription generation the transaction was prepared against.
        generation: u64,
    },
}

impl Payment {
    /// Pay fees without a tip, the default.
    pub fn fee() -> Self {
        Payment::Fee { tip: 0 }
    }

    /// Pay with the subscription of `owner` itself.
    pub fn account(owner: AccountId32, generation: u64) -> Self {
        Payment::Subscription {
            context: AuthorizationContext::Account(owner),
            generation,
        }
    }

    /// Pay with the subscription that owns the CPS Scope of `node`.
    pub fn node(node: u64, generation: u64) -> Self {
        Payment::Subscription {
            context: AuthorizationContext::Node(node),
            generation,
        }
    }

    /// Pay with the subscription that owns the CPS Scope of the node whose
    /// identity is `identity`.
    pub fn node_identity(identity: AccountId32, generation: u64) -> Self {
        Payment::Subscription {
            context: AuthorizationContext::NodeIdentity(identity),
            generation,
        }
    }
}

impl From<Payment> for Value {
    fn from(payment: Payment) -> Self {
        match payment {
            Payment::Fee { tip } => Value::named_variant("Fee", [("tip", Value::u128(tip))]),
            Payment::Subscription {
                context,
                generation,
            } => Value::named_variant(
                "Subscription",
                [
                    ("context", Value::from(context)),
                    ("generation", Value::u128(generation.into())),
                ],
            ),
        }
    }
}

/// Selects the [`Payment`] of a transaction on the extrinsic params builder.
pub trait ExtrinsicParamsBuilderExt: Sized {
    /// Pay the transaction with `payment`.
    fn payment(self, payment: Payment) -> Self;
}

impl<T: Config> ExtrinsicParamsBuilderExt for DefaultExtrinsicParamsBuilder<T> {
    fn payment(self, payment: Payment) -> Self {
        self.custom_extension(CHARGE_TRANSACTION, payment)
    }
}

/// Default transaction extensions that also know `ChargeTransaction`.
///
/// Without an explicit [`Payment`] the transaction pays fees without a tip.
pub struct RobonomicsTransactionExtensions<T: Config>(DefaultTransactionExtensions<T>);

impl<T: Config> RobonomicsTransactionExtensions<T> {
    fn has_explicit_payment(&self, name: &str) -> bool {
        name == CHARGE_TRANSACTION && frame_decode_contains(&self.0, name)
    }
}

fn frame_decode_contains<T: Config>(
    extensions: &DefaultTransactionExtensions<T>,
    name: &str,
) -> bool {
    subxt::ext::frame_decode::extrinsics::TransactionExtensions::contains_extension(
        extensions, name,
    )
}

impl<T: Config> TransactionExtensions<T> for RobonomicsTransactionExtensions<T> {
    type Params = DefaultExtrinsicParams<T>;

    fn new(
        client: &ClientState<T>,
        params: Self::Params,
    ) -> Result<Self, TransactionExtensionError> {
        <DefaultTransactionExtensions<T> as TransactionExtensions<T>>::new(client, params).map(Self)
    }

    fn inject_signature(&mut self, account_id: &T::AccountId, signature: &T::Signature) {
        self.0.inject_signature(account_id, signature);
    }
}

impl<T: Config> subxt::ext::frame_decode::extrinsics::TransactionExtensions<PortableRegistry>
    for RobonomicsTransactionExtensions<T>
{
    fn contains_extension(&self, name: &str) -> bool {
        name == CHARGE_TRANSACTION || frame_decode_contains(&self.0, name)
    }

    fn is_authorization_extension(&self, name: &str) -> bool {
        subxt::ext::frame_decode::extrinsics::TransactionExtensions::<
            PortableRegistry,
        >::is_authorization_extension(&self.0, name)
    }

    fn encode_extension_value_to(
        &self,
        name: &str,
        type_id: u32,
        type_resolver: &PortableRegistry,
        out: &mut Vec<u8>,
    ) -> Result<(), TransactionExtensionsError> {
        if name == CHARGE_TRANSACTION && !self.has_explicit_payment(name) {
            return Value::from(Payment::fee())
                .encode_as_type_to(type_id, type_resolver, out)
                .map_err(|error| TransactionExtensionsError::Other {
                    extension_name: name.to_owned(),
                    error: Box::new(error),
                });
        }
        subxt::ext::frame_decode::extrinsics::TransactionExtensions::encode_extension_value_to(
            &self.0,
            name,
            type_id,
            type_resolver,
            out,
        )
    }

    fn encode_extension_implicit_to(
        &self,
        name: &str,
        type_id: u32,
        type_resolver: &PortableRegistry,
        out: &mut Vec<u8>,
    ) -> Result<(), TransactionExtensionsError> {
        // `ChargeTransaction` has no implicit data.
        if name == CHARGE_TRANSACTION && !self.has_explicit_payment(name) {
            return Ok(());
        }
        subxt::ext::frame_decode::extrinsics::TransactionExtensions::encode_extension_implicit_to(
            &self.0,
            name,
            type_id,
            type_resolver,
            out,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::runtime_types::{
        pallet_robonomics_transaction::extension as generated,
        robonomics_subscription_primitives::authorization::AuthorizationContext as GeneratedContext,
    };
    use parity_scale_codec::Encode;
    use subxt::ext::scale_encode::EncodeAsType;

    /// Encode `payment` as the metadata type of the `ChargeTransaction` extension.
    fn encode_with_metadata(payment: Payment) -> Vec<u8> {
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/metadata.scale"));
        let metadata = subxt::metadata::Metadata::decode_from(bytes).expect("metadata decodes");
        let version = metadata
            .extrinsic()
            .transaction_extension_version_to_use_for_encoding();
        let extra_ty = metadata
            .extrinsic()
            .transaction_extensions_by_version(version)
            .expect("extension version exists")
            .find(|extension| extension.identifier() == CHARGE_TRANSACTION)
            .expect("the runtime declares ChargeTransaction")
            .extra_ty();
        let mut out = Vec::new();
        Value::from(payment)
            .encode_as_type_to(extra_ty, metadata.types(), &mut out)
            .expect("the payment matches the metadata type");
        out
    }

    fn account(byte: u8) -> AccountId32 {
        AccountId32([byte; 32])
    }

    #[test]
    fn fee_payment_matches_the_runtime_type() {
        let expected = generated::ChargeTransaction(generated::Payment::Fee { tip: 7 });
        assert_eq!(
            encode_with_metadata(Payment::Fee { tip: 7 }),
            expected.encode()
        );
        assert_eq!(encode_with_metadata(Payment::fee())[0], 0);
    }

    #[test]
    fn subscription_payments_match_the_runtime_type() {
        let subscription = |context| {
            generated::ChargeTransaction(generated::Payment::Subscription {
                context,
                generation: 9,
            })
            .encode()
        };
        assert_eq!(
            encode_with_metadata(Payment::account(account(1), 9)),
            subscription(GeneratedContext::Account(account(1)))
        );
        assert_eq!(
            encode_with_metadata(Payment::node(5, 9)),
            subscription(GeneratedContext::Node(
                crate::api::runtime_types::pallet_robonomics_cps::NodeId(5)
            ))
        );
        assert_eq!(
            encode_with_metadata(Payment::node_identity(account(2), 9)),
            subscription(GeneratedContext::NodeIdentity(account(2)))
        );
    }
}
