///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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

use codec::{Decode, Encode};
use frame_support::traits::Currency;
use sp_runtime::RuntimeDebug;

/// Simple market as approach: liability has a price of execution.
#[derive(Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct SimpleMarket<AccountId, C: Currency<AccountId>>(pub C::Balance);
