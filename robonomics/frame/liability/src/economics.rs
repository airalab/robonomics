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
//! Set of approaches to handle economical aspects of agreement.

use frame_support::traits::Currency;
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

/// Open market as approach for liability price estimation.
pub struct OpenMarket<T, A>(sp_std::marker::PhantomData<(T, A)>);
impl<T: Currency<A>, A> Economical for OpenMarket<T, A> {
    // Price as economical parameter for liability.
    type Parameter = <T as Currency<A>>::Balance;
}
