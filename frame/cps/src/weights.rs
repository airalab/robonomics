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
//! Weight functions of `pallet-robonomics-cps`.

use frame_support::weights::Weight;

/// Weights of the pallet's calls. Runtimes implement it with weights
/// generated from the pallet's benchmarks.
pub trait WeightInfo {
    /// `create_node` with `meta_bytes` bytes of metadata and `payload_bytes`
    /// bytes of payload (`0` for `None`).
    fn create_node(meta_bytes: u32, payload_bytes: u32) -> Weight;
    /// `set_meta` with `bytes` bytes of metadata (`0` for `None`, i.e.
    /// removal).
    fn set_meta(bytes: u32) -> Weight;
    /// `set_payload` with `bytes` bytes of payload (`0` for `None`, i.e.
    /// removal).
    fn set_payload(bytes: u32) -> Weight;
    /// `delete_node` deleting `access_items` Access entries of the node's
    /// Scope (`0` if the node does not root a Scope).
    fn delete_node(access_items: u32) -> Weight;
    /// `create_scope` deleting `access_items` Access entries of the replaced
    /// Scope (`0` if the node did not root a Scope).
    fn create_scope(access_items: u32) -> Weight;
    /// `grant_access`.
    fn grant_access() -> Weight;
    /// `revoke_access`.
    fn revoke_access() -> Weight;
}

/// Zero weight for every call. For tests only.
pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
    fn create_node(_meta_bytes: u32, _payload_bytes: u32) -> Weight {
        Weight::zero()
    }
    fn set_meta(_bytes: u32) -> Weight {
        Weight::zero()
    }
    fn set_payload(_bytes: u32) -> Weight {
        Weight::zero()
    }
    fn delete_node(_access_items: u32) -> Weight {
        Weight::zero()
    }
    fn create_scope(_access_items: u32) -> Weight {
        Weight::zero()
    }
    fn grant_access() -> Weight {
        Weight::zero()
    }
    fn revoke_access() -> Weight {
        Weight::zero()
    }
}
