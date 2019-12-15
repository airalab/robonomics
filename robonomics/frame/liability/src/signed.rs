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

use sp_runtime::traits::{Verify, IdentifyAccount};
use codec::{Codec, Encode, Decode};
use support::dispatch;

use crate::economics::Communism;
use crate::traits::*;

/// Agreement that could be proven by asymmetric cryptography.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct SignedLiability<T: Technical, E: Economical, A, V>
    where V: Verify<Signer=A>,
          A: IdentifyAccount,
          A::AccountId: Codec
{
    technics:  T::Parameter,
    economics: E::Parameter,
    promisor:  A::AccountId,
    promisee:  A::AccountId,
    _phantom:  sp_std::marker::PhantomData<V>,
}

impl<T: Technical, A, V> Processing for SignedLiability<T, Communism, A, V>
    where V: Verify<Signer=A>,
          A: IdentifyAccount,
          A::AccountId: Codec
{
    fn on_start(&self) -> dispatch::Result { Ok(()) }
    fn on_finish(&self, _success: bool) -> dispatch::Result { Ok(()) }
}

impl<T, E, A, V> Agreement<T, E> for SignedLiability<T, E, A, V>
    where T: Technical,
          E: Economical,
          A: IdentifyAccount,
          A::AccountId: dispatch::Parameter,
          V: Verify<Signer=A> + dispatch::Parameter,
{
    type AccountId = A::AccountId;
    type Proof = V;

    fn new(
        technics:  T::Parameter,
        economics: E::Parameter,
        promisor:  Self::AccountId,
        promisee:  Self::AccountId,
    ) -> Self {
        SignedLiability {
            technics,
            economics,
            promisor,
            promisee,
            _phantom: Default::default()
        }
    }

    fn verify(
        &self,
        target: ProofTarget<T>,
        proof: &Self::Proof
    ) -> dispatch::Result {
        if match target {
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
        } { Ok(()) } else { Err("bad signature") }
    }
}
