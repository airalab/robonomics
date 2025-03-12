///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
//! Robonomics mainnet chain specification.

/// Robonomics Mainnet Chain Specification.
pub type ChainSpec = sc_service::GenericChainSpec<super::Extensions>;

/// Robonomics parachain on Kusama.
pub fn kusama_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../../../chains/kusama-parachain.raw.json")[..])
        .unwrap()
}

/// Robonomics parachain on Polkadot.
pub fn polkadot_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../../../chains/polkadot-parachain.raw.json")[..])
        .unwrap()
}
