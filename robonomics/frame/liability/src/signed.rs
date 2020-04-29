///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life> 
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

use codec::{Encode, Decode};
use sp_runtime::{
    traits::{Verify, IdentifyAccount},
    RuntimeDebug, DispatchResult
};
#[cfg(feature = "std")]
use sp_core::crypto::{Pair, Public};
use frame_system::offchain::AppCrypto;
use frame_support::{dispatch, traits::{ReservableCurrency, BalanceStatus}};

use crate::economics::{Communism, OpenMarket};
use crate::traits::*;

/// Agreement that could be proven by asymmetric cryptography.
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug)]
pub struct SignedLiability<T, E, V, A, I> where
    T: Technical,
    E: Economical,
    V: Verify<Signer=A>,
    A: IdentifyAccount<AccountId=I>,
    I: dispatch::Parameter,
{
    technics:  T::Parameter,
    economics: E::Parameter,
    promisee:  I,
    promisor:  I,
    _phantom:  sp_std::marker::PhantomData<V>,
}

impl<T, V, A, I> Processing for SignedLiability<T, Communism, V, A, I> where
    T: Technical,
    V: Verify<Signer=A>,
    A: IdentifyAccount<AccountId=I>,
    I: dispatch::Parameter,
{
    fn on_start(&self) -> DispatchResult { Ok(()) }
    fn on_finish(&self, _success: bool) -> DispatchResult { Ok(()) }
}

impl<T, V, A, I, C> Processing for SignedLiability<T, OpenMarket<C, I>, V, A, I> where 
    T: Technical,
    V: Verify<Signer=A>,
    A: IdentifyAccount<AccountId=I>,
    C: ReservableCurrency<I>,
    I: dispatch::Parameter,
{
    fn on_start(&self) -> DispatchResult {
        C::reserve(&self.promisee, self.economics)
    }

    fn on_finish(&self, success: bool) -> DispatchResult {
        if success {
            C::repatriate_reserved(
                &self.promisee,
                &self.promisor,
                self.economics,
                BalanceStatus::Free,
            ).map(|_| ())
        } else {
            if C::unreserve(&self.promisee, self.economics) == self.economics {
                Ok(())
            } else {
                Err("reserved less than expected")?
            }
        }
    }
}

impl<T, E, V, A, I> Agreement<T, E> for SignedLiability<T, E, V, A, I> where
    T: Technical,
    E: Economical,
    A: IdentifyAccount<AccountId=I>,
    V: Verify<Signer=A> + dispatch::Parameter,
    I: dispatch::Parameter,
{
    type Index = u64;
    type AccountId = I;
    type Proof = V;

    fn new(
        technics:  T::Parameter,
        economics: E::Parameter,
        promisee:  I,
        promisor:  I,
    ) -> Self {
        SignedLiability {
            technics,
            economics,
            promisee,
            promisor,
            _phantom: Default::default()
        }
    }

    fn check_params(
        &self,
        proof: &Self::Proof,
        sender: &Self::AccountId,
    ) -> bool {
        (self.technics.clone(), self.economics.clone())
            .using_encoded(|params| proof.verify(params, sender))
    }

    fn check_report(
        &self,
        index: &Self::Index,
        report: &T::Report,
        proof: &Self::Proof,
    ) -> bool {
        (index.clone(), report.clone())
            .using_encoded(|params| proof.verify(params, &self.promisor))
    }
}

/// Runtime AppCrypto proof builder.
pub struct AppProofSigner<T>(sp_std::marker::PhantomData<T>);
impl<T, E, I, AccountId, Signature, AppSigner> ProofBuilder<T, E, I, AccountId, Signature> for AppProofSigner<AppSigner> where
    T: Technical,
    E: Economical,
    I: dispatch::Parameter,
    AppSigner: AppCrypto<AccountId, Signature>,
{
    fn proof_params(
        technics: &T::Parameter,
        economics: &E::Parameter,
        sender: AccountId,
    ) -> Signature {
        (technics, economics)
            .using_encoded(|params| AppSigner::sign(&params, sender))
            .expect("unable to sign using runtime application key")
    }

    fn proof_report(
        index: &I,
        report: &T::Report, 
        sender: AccountId,
    ) -> Signature {
        (index, report)
            .using_encoded(|params| AppSigner::sign(&params, sender))
            .expect("unable to sign using runtime application key")
    }
}

/// Core crypto proof builder.
#[cfg(feature = "std")]
pub struct ProofSigner<T>(std::marker::PhantomData<T>);

#[cfg(feature = "std")]
impl<T, E, I, Account, AccountId, Signature, TPair> ProofBuilder<T, E, I, TPair, Signature> for ProofSigner<TPair> where
    T: Technical,
    E: Economical,
    I: dispatch::Parameter,
    TPair: Pair<Public=Account, Signature=Signature>,
    Account: IdentifyAccount<AccountId=AccountId> + Public + std::hash::Hash,
    AccountId: dispatch::Parameter,
    Signature: dispatch::Parameter + AsRef<[u8]>,
{
    fn proof_params(
        technics: &T::Parameter,
        economics: &E::Parameter,
        sender: TPair,
    ) -> Signature {
        (technics, economics)
            .using_encoded(|params| sender.sign(&params))
    }

    fn proof_report(
        index: &I,
        report: &T::Report, 
        sender: TPair,
    ) -> Signature {
        (index, report)
            .using_encoded(|params| sender.sign(&params))
    }
}
