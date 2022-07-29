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
//! Chain specification and utils.

use cumulus_primitives_core::ParaId;
use robonomics_primitives::{AccountId, Balance};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::sr25519;

use crate::chain_spec::get_account_id_from_seed;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

/// Specialized `AlphaChainSpec`.
pub type AlphaChainSpec = sc_service::GenericChainSpec<alpha_runtime::GenesisConfig, Extensions>;

/// Specialized `IpciChainSpec`.
pub type IpciChainSpec = sc_service::GenericChainSpec<ipci_runtime::GenesisConfig, Extensions>;

/// Specialized `MainChainSpec`.
#[cfg(feature = "kusama")]
pub type MainChainSpec = sc_service::GenericChainSpec<main_runtime::GenesisConfig, Extensions>;

pub fn alpha_chain_spec(id: ParaId) -> AlphaChainSpec {
    let balances = vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
        get_account_id_from_seed::<sr25519::Public>("Eve"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
    ];
    AlphaChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        move || {
            mk_genesis_alpha(
                balances
                    .iter()
                    .cloned()
                    .map(|a| (a, 1_000_000_000_000u128))
                    .collect(),
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                id,
            )
        },
        vec![],
        None,
        None,
        None,
        None,
        Extensions {
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

pub fn ipci_chain_spec(id: ParaId) -> IpciChainSpec {
    let balances = vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
        get_account_id_from_seed::<sr25519::Public>("Eve"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
    ];
    IpciChainSpec::from_genesis(
        "Ipci Testnet",
        "ipci_testnet",
        ChainType::Local,
        move || {
            mk_genesis_ipci(
                balances
                    .iter()
                    .cloned()
                    .map(|a| (a, 1_000_000_000_000u128))
                    .collect(),
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                id,
            )
        },
        vec![],
        None,
        None,
        None,
        None,
        Extensions {
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

#[cfg(feature = "kusama")]
pub fn main_chain_spec(id: ParaId) -> MainChainSpec {
    let balances = vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
        get_account_id_from_seed::<sr25519::Public>("Eve"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
    ];
    MainChainSpec::from_genesis(
        "Main Testnet",
        "main_testnet",
        ChainType::Local,
        move || {
            mk_genesis_main(
                balances
                    .iter()
                    .cloned()
                    .map(|a| (a, 1_000_000_000_000u128))
                    .collect(),
                id,
            )
        },
        vec![],
        None,
        None,
        None,
        None,
        Extensions {
            relay_chain: "westend-dev".into(),
            para_id: id.into(),
        },
    )
}

/// Helper function to create GenesisConfig for alpha parachain
fn mk_genesis_alpha(
    balances: Vec<(AccountId, Balance)>,
    sudo_key: AccountId,
    parachain_id: ParaId,
) -> alpha_runtime::GenesisConfig {
    let bonus = balances.clone();
    alpha_runtime::GenesisConfig {
        system: alpha_runtime::SystemConfig {
            code: alpha_runtime::wasm_binary_unwrap().to_vec(),
        },
        assets: Default::default(),
        balances: alpha_runtime::BalancesConfig { balances },
        elections: Default::default(),
        council: Default::default(),
        treasury: Default::default(),
        staking: alpha_runtime::StakingConfig { bonus },
        sudo: alpha_runtime::SudoConfig {
            key: Some(sudo_key),
        },
        parachain_info: alpha_runtime::ParachainInfoConfig { parachain_id },
        parachain_system: Default::default(),
        transaction_payment: Default::default(),
        polkadot_xcm: Default::default(),
    }
}

/// Helper function to create GenesisConfig for ipci parachain
fn mk_genesis_ipci(
    balances: Vec<(AccountId, Balance)>,
    sudo_key: AccountId,
    parachain_id: ParaId,
) -> ipci_runtime::GenesisConfig {
    ipci_runtime::GenesisConfig {
        system: ipci_runtime::SystemConfig {
            code: ipci_runtime::wasm_binary_unwrap().to_vec(),
        },
        assets: Default::default(),
        carbon_assets: Default::default(),
        balances: ipci_runtime::BalancesConfig { balances },
        sudo: ipci_runtime::SudoConfig {
            key: Some(sudo_key),
        },
        parachain_info: ipci_runtime::ParachainInfoConfig { parachain_id },
        parachain_system: Default::default(),
        transaction_payment: Default::default(),
    }
}

/// Helper function to create GenesisConfig for main parachain
#[cfg(feature = "kusama")]
fn mk_genesis_main(
    balances: Vec<(AccountId, Balance)>,
    parachain_id: ParaId,
) -> main_runtime::GenesisConfig {
    main_runtime::GenesisConfig {
        system: main_runtime::SystemConfig {
            code: main_runtime::wasm_binary_unwrap().to_vec(),
        },
        parachain_system: Default::default(),
        balances: main_runtime::BalancesConfig { balances },
        assets: Default::default(),
        vesting: Default::default(),
        staking: main_runtime::StakingConfig { bonus: vec![] },
        parachain_info: main_runtime::ParachainInfoConfig { parachain_id },
        democracy: main_runtime::DemocracyConfig::default(),
        treasury: Default::default(),
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        polkadot_xcm: Default::default(),
        transaction_payment: Default::default(),
    }
}

/*
/// Kusama parachain genesis.
fn robonomics_parachain_genesis() -> main_runtime::GenesisConfig {
    use hex_literal::hex;
    use main_runtime::constants::currency;

    // akru
    let sudo_key: AccountId =
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into();

    let balances = vec![(sudo_key.clone(), 1000 * currency::XRT)];
    mk_genesis_main(balances.to_vec(), sudo_key, KUSAMA_ID.into())
}

/// Kusama parachain config.
pub fn robonomics_parachain_config() -> MainChainSpec {
    let boot_nodes = vec![];
    MainChainSpec::from_genesis(
        "Robonomics",
        "robonomics",
        ChainType::Live,
        kusama_parachain_genesis,
        boot_nodes,
        None,
        Some(ROBONOMICS_PROTOCOL_ID),
        None,
        Extensions {
            relay_chain: "kusama".into(),
            para_id: KUSAMA_ID.into(),
        },
    )
}
*/

/// Mercury parachain confing.
pub fn mercury_parachain_config() -> AlphaChainSpec {
    AlphaChainSpec::from_json_bytes(&include_bytes!("../../../chains/mercury.raw.json")[..]).unwrap()
}

/// Uranus parachain confing.
pub fn ipci_parachain_config() -> IpciChainSpec {
    IpciChainSpec::from_json_bytes(&include_bytes!("../../../chains/ipci.raw.json")[..]).unwrap()
}

/// Robonomics parachain confing.
#[cfg(feature = "kusama")]
pub fn robonomics_parachain_config() -> MainChainSpec {
    MainChainSpec::from_json_bytes(&include_bytes!("../../../chains/robonomics.raw.json")[..]).unwrap()
}
