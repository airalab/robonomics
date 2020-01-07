///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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

use sp_runtime::{
    traits::{Verify, IdentifyAccount},
    RuntimeDebug, DispatchResult,
};
use frame_support::{dispatch, traits::ReservableCurrency};
use codec::{Encode, Decode};

use crate::economics::{Communism, OpenMarket};
use crate::traits::*;

/// Agreement that could be proven by asymmetric cryptography.
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug)]
pub struct SignedLiability<T: Technical, E: Economical, V, A, AccountId>
    where V: Verify<Signer=A>,
          A: IdentifyAccount<AccountId=AccountId>,
          AccountId: dispatch::Parameter,
{
    technics:  T::Parameter,
    economics: E::Parameter,
    promisee:  AccountId,
    promisor:  AccountId,
    _phantom:  sp_std::marker::PhantomData<V>,
}

impl<T: Technical, V, A, AccountId> Processing for SignedLiability<T, Communism, V, A, AccountId>
    where V: Verify<Signer=A>,
          A: IdentifyAccount<AccountId=AccountId>,
          AccountId: dispatch::Parameter,
{
    fn on_start(&self) -> DispatchResult { Ok(()) }
    fn on_finish(&self, _success: bool) -> DispatchResult { Ok(()) }
}

impl<T: Technical, V, A, AccountId, C> Processing for SignedLiability<T, OpenMarket<C, AccountId>, V, A, AccountId>
    where V: Verify<Signer=A>,
          A: IdentifyAccount<AccountId=AccountId>,
          C: ReservableCurrency<AccountId>,
          AccountId: dispatch::Parameter,
{
    fn on_start(&self) -> DispatchResult {
        C::reserve(&self.promisee, self.economics)
    }

    fn on_finish(&self, success: bool) -> DispatchResult {
        if success {
            C::repatriate_reserved(&self.promisee, &self.promisor, self.economics)
                .map(|_| ())
        } else {
            if C::unreserve(&self.promisee, self.economics) == self.economics {
                Ok(())
            } else {
                Err("reserved less than expected")?
            }
        }
    }
}

impl<T, E, V, A, AccountId> Agreement<T, E, AccountId> for SignedLiability<T, E, V, A, AccountId>
    where T: Technical,
          E: Economical,
          A: IdentifyAccount<AccountId=AccountId>,
          V: Verify<Signer=A> + dispatch::Parameter,
          AccountId: dispatch::Parameter,
{
    type Proof = V;

    fn new(
        technics:  T::Parameter,
        economics: E::Parameter,
        promisee:  AccountId,
        promisor:  AccountId,
    ) -> Self {
        SignedLiability {
            technics,
            economics,
            promisee,
            promisor,
            _phantom: Default::default()
        }
    }

    fn verify(
        &self,
        target: ProofTarget<T>,
        proof: &Self::Proof
    ) -> bool {
        match target {
            ProofTarget::Promisee => {
                let order = (self.technics.clone(), self.economics.clone());
                order.using_encoded(|params| proof.verify(params, &self.promisee))
            },
            ProofTarget::Promisor => {
                let order = (self.technics.clone(), self.economics.clone());
                order.using_encoded(|params| proof.verify(params, &self.promisor))
            },
            ProofTarget::Report(report) =>
                report.using_encoded(|params| proof.verify(params, &self.promisor))
        }
    }
}
