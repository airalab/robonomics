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
//! Pallet weights trait & utils.

use frame_support::weights::Weight;

/// Weight information for pallet extrinsics.
///
/// Provides benchmark-derived weights for each extrinsic in the pallet.
pub trait WeightInfo {
    fn create_node() -> Weight;
    fn set_meta() -> Weight;
    fn set_payload() -> Weight;
    fn delete_node() -> Weight;
    fn create_scope() -> Weight;
    fn delete_scope() -> Weight;
    fn grant_access() -> Weight;
    fn revoke_access() -> Weight;
    /// Cost of removing `items` `Access` entries via a single bounded
    /// `clear_prefix` call during background GC.
    fn gc_access(items: u32) -> Weight;
    /// Cost of the final GC phase: removing `ScopeOwner` / `ScopeRoot` and
    /// dequeuing the completed cleanup task.
    fn gc_metadata() -> Weight;
}

/// Test weight implementation that returns zero weight for all operations.
///
/// Used in testing environments where actual weight calculations are not needed.
pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
    fn create_node() -> Weight {
        Weight::zero()
    }
    fn set_meta() -> Weight {
        Weight::zero()
    }
    fn set_payload() -> Weight {
        Weight::zero()
    }
    fn delete_node() -> Weight {
        Weight::zero()
    }
    fn create_scope() -> Weight {
        Weight::zero()
    }
    fn delete_scope() -> Weight {
        Weight::zero()
    }
    fn grant_access() -> Weight {
        Weight::zero()
    }
    fn revoke_access() -> Weight {
        Weight::zero()
    }
    fn gc_access(_items: u32) -> Weight {
        // Nonzero so `on_idle` weight-budget tests (insufficient weight,
        // exact bounded batch math) are meaningful; other extrinsics'
        // weights are irrelevant to dispatch success/failure in tests.
        Weight::from_parts(1_000, 0)
    }
    fn gc_metadata() -> Weight {
        Weight::from_parts(1_000, 0)
    }
}
