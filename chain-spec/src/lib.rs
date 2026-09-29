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
//! Robonomics chain specs, embedded as static text.
//!
//! This crate has no runtime dependencies: it simply bundles the raw JSON chain spec
//! files committed to this repository as `&'static str` constants.
//!
//! Downstream tools (e.g. [`robins`](https://github.com/airalab/robins)) can
//! depend on this crate to get an always-in-sync copy of the Robonomics
//! chain specs without having to vendor JSON files, fetch them from GitHub,
//! or keep a local checkout of this repository around.

/// Raw chain spec for the Polkadot relay chain.
pub const POLKADOT_RELAY_RAW: &str = include_str!("../polkadot-relay.raw.json");

/// Raw chain spec for the Robonomics parachain on the Polkadot relay chain.
pub const POLKADOT_PARACHAIN_RAW: &str = include_str!("../polkadot-parachain.raw.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_specs_are_valid_json() {
        for spec in [POLKADOT_RELAY_RAW, POLKADOT_PARACHAIN_RAW] {
            let value: serde_json::Value =
                serde_json::from_str(spec).expect("chain spec must be valid JSON");
            assert!(value.is_object(), "chain spec root must be a JSON object");
            assert!(
                value.get("genesis").is_some(),
                "chain spec must contain a `genesis` field"
            );
        }
    }
}
