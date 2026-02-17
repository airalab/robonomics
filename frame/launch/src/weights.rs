///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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
//! Pallet weights trait & utils.

use frame_support::weights::Weight;

/// Weight information for pallet extrinsics.
///
/// Provides benchmark-derived weights for each extrinsic in the pallet.
pub trait WeightInfo {
    fn launch() -> Weight;
}

/// Test weight implementation that returns zero weight for all operations.
///
/// Used in testing environments where actual weight calculations are not needed.
pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
    fn launch() -> Weight {
        Weight::zero()
    }
}
