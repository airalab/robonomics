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
//! Chain specification and utils.

use node_primitives::{AccountId, Balance, Block};
use robonomics_parachain_runtime::{
    BalancesConfig, ElectionsConfig, GenesisConfig, IndicesConfig, SudoConfig, SystemConfig,
    WASM_BINARY,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

pub const ROBONOMICS_PARACHAIN_ID: &str = "robonomics";
const ROBONOMICS_PROTOCOL_ID: &str = "xrt";
const ROBONOMICS_PROPERTIES: &str = r#"
    {
        "ss58Format": 32,
        "tokenDecimals": 9,
        "tokenSymbol": "XRT"
    }"#;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/*
/// Robonomics testnet config.
pub fn robonomics_parachain_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../../res/robonomics_parachain.json")[..]).unwrap()
}
*/

/// Helper function to create GenesisConfig for parachain
fn mk_genesis(
    balances: Vec<(AccountId, Balance)>,
    sudo_key: AccountId,
    code: Vec<u8>,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code,
            changes_trie_config: Default::default(),
        }),
        pallet_indices: Some(IndicesConfig {
            indices: vec![],
        }),
        pallet_balances: Some(BalancesConfig {
            balances,
        }),
        pallet_generic_asset: Some(Default::default()),
        pallet_elections_phragmen: Some(ElectionsConfig { members: vec![] }),
        pallet_collective_Instance1: Some(Default::default()),
        pallet_treasury: Some(Default::default()),
        pallet_sudo: Some(SudoConfig { key: sudo_key }),
    }
}

/// Robonomics parachain genesis.
fn robonomics_parachain_genesis() -> GenesisConfig {
    use robonomics_parachain_runtime::constants::currency;
    use hex_literal::hex;

    // akru
    let sudo_key: AccountId =
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into();

    let mut balances = currency::STAKE_HOLDERS.clone();
    balances.extend(vec![(sudo_key.clone(), 50_000 * currency::XRT)]);

    mk_genesis(balances.to_vec(), sudo_key, WASM_BINARY.to_vec())
}

/// Robonomics parachain config.
pub fn robonomics_parachain_config() -> ChainSpec {
    let boot_nodes = vec![];
    ChainSpec::from_genesis(
        "Robonomics",
        ROBONOMICS_PARACHAIN_ID,
        ChainType::Live,
        robonomics_parachain_genesis,
        boot_nodes,
        Some(sc_telemetry::TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)]).unwrap()),
        Some(ROBONOMICS_PROTOCOL_ID),
        Some(serde_json::from_str(ROBONOMICS_PROPERTIES).unwrap()),
        Default::default(),
    )
}
