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
//! Signed liability implementation.

use frame_support::{
    dispatch,
    traits::{BalanceStatus, ReservableCurrency},
};
use frame_system::offchain::AppCrypto;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use sp_core::crypto::{Pair, Public};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    DispatchResult, RuntimeDebug,
};
use sp_std::marker::PhantomData;

use crate::economics::SimpleMarket;
use crate::traits::*;

/// Agreement that could be proven by asymmetric cryptography.
#[derive(
    Encode,
    Decode,
    Clone,
    PartialEq,
    Eq,
    TypeInfo,
    RuntimeDebug,
    MaxEncodedLen,
    DecodeWithMemTracking,
)]
pub struct SignedAgreement<T, E, AccountId, Signature> {
    pub technics: T,
    pub economics: E,
    pub promisee: AccountId,
    pub promisor: AccountId,
    pub promisee_signature: Signature,
    pub promisor_signature: Signature,
}

// No economical parameters for agreement.
impl<T, A, S> Processing for SignedAgreement<T, (), A, S> {
    fn on_start(&self) -> DispatchResult {
        Ok(())
    }
    fn on_finish(&self, _success: bool) -> DispatchResult {
        Ok(())
    }
}

impl<T, C, A, S> Processing for SignedAgreement<T, SimpleMarket<A, C>, A, S>
where
    C: ReservableCurrency<A>,
{
    fn on_start(&self) -> DispatchResult {
        C::reserve(&self.promisee, self.economics.price)
    }

    fn on_finish(&self, success: bool) -> DispatchResult {
        if success {
            C::repatriate_reserved(
                &self.promisee,
                &self.promisor,
                self.economics.price,
                BalanceStatus::Free,
            )
            .map(|_| ())
        } else {
            if C::unreserve(&self.promisee, self.economics.price) == self.economics.price {
                Ok(())
            } else {
                Err("reserved less than expected")?
            }
        }
    }
}

impl<T, E, A, V, I> Agreement<I> for SignedAgreement<T, E, I, V>
where
    A: IdentifyAccount<AccountId = I>,
    V: Verify<Signer = A> + dispatch::Parameter,
    I: dispatch::Parameter,
    T: dispatch::Parameter,
    E: dispatch::Parameter,
{
    type Technical = T;
    type Economical = E;

    fn technical(&self) -> Self::Technical {
        self.technics.clone()
    }

    fn economical(&self) -> Self::Economical {
        self.economics.clone()
    }

    fn promisee(&self) -> I {
        self.promisee.clone()
    }

    fn promisor(&self) -> I {
        self.promisor.clone()
    }

    fn verify(&self) -> bool {
        #[cfg(feature = "runtime-benchmarks")]
        {
            // In benchmark mode, skip signature verification
            return true;
        }

        #[cfg(not(feature = "runtime-benchmarks"))]
        {
            (self.technics.clone(), self.economics.clone()).using_encoded(|encoded| {
                self.promisee_signature.verify(encoded, &self.promisee)
                    && self.promisor_signature.verify(encoded, &self.promisor)
            })
        }
    }
}

/// Report that could be proven by asymmetric cryptography.
#[derive(
    Encode,
    Decode,
    Clone,
    PartialEq,
    Eq,
    TypeInfo,
    RuntimeDebug,
    MaxEncodedLen,
    DecodeWithMemTracking,
)]
pub struct SignedReport<Index, AccountId, Signature, Message> {
    pub index: Index,
    pub sender: AccountId,
    pub payload: Message,
    pub signature: Signature,
}

impl<I, A, S, M> RealWorldOracle for SignedReport<I, A, S, M> {
    fn is_confirmed(&self) -> Option<bool> {
        // Confirm all by default
        Some(true)
    }
}

impl<Index, A, V, I, M> Report<Index, I> for SignedReport<Index, I, V, M>
where
    Index: dispatch::Parameter,
    A: IdentifyAccount<AccountId = I>,
    V: Verify<Signer = A> + dispatch::Parameter,
    M: dispatch::Parameter,
    I: dispatch::Parameter,
{
    type Message = M;

    fn index(&self) -> Index {
        self.index.clone()
    }

    fn sender(&self) -> I {
        self.sender.clone()
    }

    fn verify(&self) -> bool {
        #[cfg(feature = "runtime-benchmarks")]
        {
            // In benchmark mode, skip signature verification
            return true;
        }

        #[cfg(not(feature = "runtime-benchmarks"))]
        {
            (self.index.clone(), self.payload.clone())
                .using_encoded(|encoded| self.signature.verify(encoded, &self.sender))
        }
    }
}

/// Runtime AppCrypto proof builder.
pub struct AppProofSigner<T>(PhantomData<T>);

impl<T, E, A, AccountId, Signature, AppSigner> AgreementProofBuilder<T, E, AccountId, Signature>
    for AppProofSigner<AppSigner>
where
    AppSigner: AppCrypto<AccountId, Signature>,
    A: IdentifyAccount<AccountId = AccountId>,
    Signature: Verify<Signer = A>,
    AccountId: Clone,
    T: Encode,
    E: Encode,
{
    fn proof(technics: &T, economics: &E, sender: &AccountId) -> Signature {
        (technics, economics)
            .using_encoded(|params| AppSigner::sign(params, sender.clone()))
            .expect("unable to sign using runtime application key")
    }
}

impl<Index, A, AccountId, Signature, AppSigner, M>
    ReportProofBuilder<Index, M, AccountId, Signature> for AppProofSigner<AppSigner>
where
    AppSigner: AppCrypto<AccountId, Signature>,
    A: IdentifyAccount<AccountId = AccountId>,
    Signature: Verify<Signer = A>,
    AccountId: Clone,
    Index: Encode,
    M: Encode,
{
    fn proof(index: &Index, message: &M, sender: &AccountId) -> Signature {
        (index, message)
            .using_encoded(|params| AppSigner::sign(params, sender.clone()))
            .expect("unable to sign using runtime application key")
    }
}

/// Core crypto proof builder.
#[cfg(feature = "std")]
pub struct ProofSigner<T>(std::marker::PhantomData<T>);

#[cfg(feature = "std")]
impl<T, E, Account, AccountId, Signature, TPair> AgreementProofBuilder<T, E, TPair, Signature>
    for ProofSigner<TPair>
where
    T: Encode,
    E: Encode,
    TPair: Pair<Public = Account, Signature = Signature>,
    Account: IdentifyAccount<AccountId = AccountId> + Public + std::hash::Hash,
    Signature: Verify<Signer = Account>,
{
    fn proof(technics: &T, economics: &E, sender: &TPair) -> Signature {
        (technics, economics).using_encoded(|params| sender.sign(params))
    }
}

#[cfg(feature = "std")]
impl<Index, Account, AccountId, Signature, TPair, M> ReportProofBuilder<Index, M, TPair, Signature>
    for ProofSigner<TPair>
where
    Index: Encode,
    TPair: Pair<Public = Account, Signature = Signature>,
    Account: IdentifyAccount<AccountId = AccountId> + Public + std::hash::Hash,
    Signature: Verify<Signer = Account>,
    M: Encode,
{
    fn proof(index: &Index, message: &M, sender: &TPair) -> Signature {
        (index, message).using_encoded(|params| sender.sign(params))
    }
}
