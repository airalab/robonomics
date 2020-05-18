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
//! Set of approaches to handle technical aspects of agreement.

use crate::traits::{RealWorldOracle, Technical};
use sp_std::prelude::Vec;

/// Using IPFS to handle technical aspects of agreement without confirmation.
pub struct PureIPFS;
impl Technical for PureIPFS {
    // IPFS hash of objective as parameter for liability.
    type Parameter = Vec<u8>;
    // IPFS hash of work results as report for liability.
    type Report = Vec<u8>;
    // No confirmation from real world (unsafe, be careful).
    type Oracle = ();
}

/// Noop oracle.
impl RealWorldOracle for () {}
