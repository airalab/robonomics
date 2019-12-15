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
//! Set of approaches to handle economical aspects of agreement.

use crate::traits::Economical;

///
/// Well, when we get communism it'll all be fucking great.
/// It will come soon, we just have to wait.
/// Everything will be free there, everything will be an upper.
/// We'll probably not even have to die.
///
pub struct Communism;
impl Economical for Communism {
    // No parameters, because everything is free.
    type Parameter = ();
}

/*
/// Open market as approach for liability price estimation.
pub struct OpenMarket<T>(BalanceOf<T>);
impl<T> Economical for OpenMarket<T> {
    // Price as economical parameter for liability.
    type Parameter = BalanceOf<T>;
    // Token processing parameter.
    type Processing = NativeToken<T>;
}

pub struct NativeToken<T>();
impl<T> Processing for NativeToken<T> {
}
    {
        if self.using_encoded(|agreement| proof.verify(&mut &agreement[..], account)) {
            Ok(())
        } else {
            // TODO: explain error
            Err("bad signature")
        }
    }

/// Type synonym for balances in processing currency.
type ProcessingBalanceOf<T> =
    <<T as Processing>::Currency as Currency<<T as Processing>::AccountId>>::Balance;

impl
    /// Economical image of parties is their accounts.
    type AccountId;

    /// Processing currency.
    type Currency: ReservableCurrency<Self::AccountId>;

        Self::Currency::reserve(&promisee, cost).map_err(|_| "promisee's balance too low")
        Self::Currency::repatriate_reserved(&promisee, &promisor, cost)

type AccountId<T: Verify> = <T::Signer as IdentifyAccount>::AccountId;
pub type Proof<T: Verify> = (T, AccountId<T>);
pub fn verify<T: Verify, L: Lazy<[u8]>>(msg: L, proof: Proof<T>) -> Option<AccountId<T>> {
    let (signature, account) = proof;
    if signature.verify(msg, &account) {
        Some(account)
    } else {
        None
    }
}
*/
