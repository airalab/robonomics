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
//! Stable resource identifiers.

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// Identifier of an accounting epoch of a subscription.
///
/// Every new subscription allocation of an owner starts a new generation;
/// charges made in one generation must never be credited to another.
pub type ResourceGeneration = u64;

/// Resources that can be governed by a subscription.
///
/// The SCALE indexes are part of the deployed interface and must never be
/// changed or reused. New resources must be appended with a new index.
///
/// How a resource is limited and measured is resource-specific and is
/// expressed by the `Limit` and `Usage` types chosen by the runtime.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
)]
pub enum Resource {
    /// Transaction execution, limited as a rate of baseline XRT per second.
    #[codec(index = 0)]
    Transaction,
    /// Data storage.
    #[codec(index = 1)]
    Storage,
    /// Computation.
    #[codec(index = 2)]
    Compute,
}
