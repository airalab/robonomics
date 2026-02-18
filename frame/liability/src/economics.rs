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
//! Set of approaches to handle economical aspects of agreement.

use frame_support::traits::Currency;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

/// Simple market as approach: liability has a price of execution.
#[derive(
    Encode,
    Decode,
    PartialEq,
    Clone,
    Eq,
    TypeInfo,
    RuntimeDebug,
    MaxEncodedLen,
    DecodeWithMemTracking,
)]
#[scale_info(skip_type_params(C))]
pub struct SimpleMarket<AccountId, C: Currency<AccountId>> {
    #[codec(compact)]
    pub price: C::Balance,
}
