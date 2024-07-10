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

use super::{get_account_id_from_seed, Extensions};
use main_runtime::{
    wasm_binary_unwrap, BalancesConfig, DemocracyConfig, ParachainInfoConfig, RuntimeGenesisConfig,
    SystemConfig,
};
use robonomics_primitives::{AccountId, Balance, CommunityAccount};

use cumulus_primitives_core::ParaId;
use sc_chain_spec::ChainType;
use sp_core::sr25519;
use sp_runtime::traits::IdentifyAccount;

/// Robonomics Mainnet Chain Specification.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

fn main_genesis(
    balances: Vec<(AccountId, Balance)>,
    parachain_id: ParaId,
    code: Vec<u8>,
) -> RuntimeGenesisConfig {
    RuntimeGenesisConfig {
        system: SystemConfig {
            code,
            ..Default::default()
        },
        balances: BalancesConfig { balances },
        assets: Default::default(),
        vesting: Default::default(),
        parachain_info: ParachainInfoConfig {
            parachain_id,
            ..Default::default()
        },
        parachain_system: Default::default(),
        polkadot_xcm: Default::default(),
        democracy: DemocracyConfig::default(),
        treasury: Default::default(),
        technical_committee: Default::default(),
        technical_membership: Default::default(),
        transaction_payment: Default::default(),
    }
}

/// Create Mainnet GenesisConfig.
pub fn genesis(
    endowed_accounts: Option<Vec<AccountId>>,
    parachain_id: ParaId,
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

    main_genesis(
        endowed_accounts,
        parachain_id,
        wasm_binary_unwrap().to_vec(),
    )
}

/// Create Mainnet Chain Specification (single validator Alice)
pub fn config(parachain_id: ParaId) -> ChainSpec {
    let mk_genesis = move || genesis(None, parachain_id);

    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "XRT".into());
    properties.insert("tokenDecimals".into(), 9.into());

    ChainSpec::from_genesis(
        "Robonomics Mainnet",
        "robonomics",
        ChainType::Live,
        mk_genesis,
        vec![],
        None,
        None,
        None,
        Some(properties),
        Extensions {
            relay_chain: "kusama".into(),
            para_id: parachain_id.into(),
        },
    )
}

pub fn kusama_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../../../chains/kusama-parachain.raw.json")[..])
        .unwrap()
}
