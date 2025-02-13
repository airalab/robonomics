///////////////////////////////////////////////////////////////////////////////

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
//! DevNet chain specification.

use super::{get_account_id_from_seed, get_from_seed, Extensions};
use dev_runtime::{
    wasm_binary_unwrap, AuraConfig, AuraId, BalancesConfig, DemocracyConfig, GrandpaConfig,
    GrandpaId, RuntimeGenesisConfig, SudoConfig, SystemConfig,
};
use robonomics_primitives::{AccountId, Balance, CommunityAccount};

use sc_chain_spec::ChainType;
use sp_core::sr25519;
use sp_runtime::traits::IdentifyAccount;

// /// DevNet Chain Specification.
// pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;
// pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

fn get_authority_keys_from_seed(seed: &str) -> (AuraId, GrandpaId) {
    (
        get_from_seed::<AuraId>(seed),
        get_from_seed::<GrandpaId>(seed),
    )
}

fn devnet_genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    balances: Vec<(AccountId, Balance)>,
    sudo_key: AccountId,
    code: Vec<u8>,
) -> RuntimeGenesisConfig {
    RuntimeGenesisConfig {
        system: SystemConfig {
            // code,
            ..Default::default()
        },
        balances: BalancesConfig { balances },
        aura: AuraConfig {
            authorities: initial_authorities.iter().map(|x| x.0.clone()).collect(),
        },
        assets: Default::default(),
        grandpa: GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
            ..Default::default()
        },
        sudo: SudoConfig {
            key: Some(sudo_key),
        },
        vesting: Default::default(),
        democracy: DemocracyConfig::default(),
        treasury: Default::default(),
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        transaction_payment: Default::default(),
    }
}

/// Create DevNet GenesisConfig.
pub fn genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    endowed_accounts: Option<Vec<AccountId>>,
    sudo_key: AccountId,
) -> RuntimeGenesisConfig {
    const ENDOWMENT: Balance = 1_000_000_000_000_000_000;

    let endowed_accounts: Vec<(AccountId, Balance)> = endowed_accounts
        .unwrap_or_else(|| {
            vec![
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_account_id_from_seed::<sr25519::Public>("Charlie"),
                get_account_id_from_seed::<sr25519::Public>("Dave"),
                get_account_id_from_seed::<sr25519::Public>("Eve"),
                get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                CommunityAccount::Treasury.into_account(),
            ]
        })
        .iter()
        .cloned()
        .map(|acc| (acc, ENDOWMENT))
        .collect();

    devnet_genesis(
        initial_authorities,
        endowed_accounts,
        sudo_key,
        wasm_binary_unwrap().to_vec(),
    )
}

// /// Create DevNet Chain Specification (single validator Alice)
// pub fn config() -> ChainSpec {
//     let mk_genesis = || {
//         genesis(
//             vec![get_authority_keys_from_seed("Alice")],
//             None,
//             get_account_id_from_seed::<sr25519::Public>("Alice"),
//         )
//     };
//
//     let mut properties = sc_chain_spec::Properties::new();
//     properties.insert("tokenSymbol".into(), "XRT".into());
//     properties.insert("tokenDecimals".into(), 9.into());
//
//     // ChainSpec::from_genesis(
//     //     "Development",
//     //     "dev",
//     //     ChainType::Development,
//     //     mk_genesis,
//     //     vec![],
//     //     None,
//     //     None,
//     //     None,
//     //     Some(properties),
//     //     Default::default(),
//     // )
//
// }

pub fn config() -> ChainSpec {
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "XRT".into());
    properties.insert("tokenDecimals".into(), 9.into());

    ChainSpec::builder(
        wasm_binary_unwrap(),
        // genesis(
        //     vec![get_authority_keys_from_seed("Alice")],
        //     None,
        //     get_account_id_from_seed::<sr25519::Public>("Alice"),
        // ),
        Extensions {
            // ???
            relay_chain: "kusama".into(),
            // You MUST set this to the correct network!
            // ???
            para_id: 1000,
        },
    )
    .with_name("Development")
    .with_id("dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
    // .with_extensions()
    .build()
}
