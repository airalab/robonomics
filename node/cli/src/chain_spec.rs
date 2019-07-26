///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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

use grandpa::AuthorityId as GrandpaId;
use babe_primitives::AuthorityId as BabeId;
use primitives::{ed25519, sr25519, crypto::Pair};
use robonomics_runtime::{
    GenesisConfig, SystemConfig, SessionConfig, BabeConfig, StakingConfig,
    IndicesConfig, ImOnlineConfig, BalancesConfig, GrandpaConfig, SudoConfig,
    SessionKeys, Perbill, StakerStatus, WASM_BINARY,
};
use robonomics_runtime::constants::{time::*, currency::*};
use robonomics_runtime::types::{AccountId, Balance};
use substrate_service::{self, Properties};
use serde_json::json;

use hex_literal::hex;
use primitives::crypto::UncheckedInto;
use telemetry::TelemetryEndpoints;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialised `ChainSpec`. This is a specialisation of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum ChainOpt {
    /// Whatever the current runtime is, with just Alice as an auth.
    Development,
    /// Whatever the current runtime is, with simple Alice/Bob auths.
    LocalTestnet,
    /// Robonomics public testnet.
    Robonomics,
}

impl ChainOpt {
    /// Get an actual chain config from one of the alternatives.
    pub(crate) fn load(self) -> Result<ChainSpec, String> {
        Ok(match self {
            ChainOpt::Development => development_config(),
            ChainOpt::LocalTestnet => local_testnet_config(),
            ChainOpt::Robonomics => robonomics_testnet_config(),
        })
    }

    pub(crate) fn from(s: &str) -> Option<Self> {
        match s {
            "dev" => Some(ChainOpt::Development),
            "local" => Some(ChainOpt::LocalTestnet),
            "" | "robonomics" => Some(ChainOpt::Robonomics),
            _ => None,
        }
    }
}

/// Helper function to generate AccountId from seed
pub fn get_account_id_from_seed(seed: &str) -> AccountId {
    sr25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate BabeId from seed
pub fn get_babe_id_from_seed(seed: &str) -> BabeId {
    sr25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate GrandpaId from seed
pub fn get_grandpa_id_from_seed(seed: &str) -> GrandpaId {
    ed25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, BabeId, GrandpaId) {
    (
        get_account_id_from_seed(&format!("{}//stash", seed)),
        get_account_id_from_seed(seed),
        get_babe_id_from_seed(seed),
        get_grandpa_id_from_seed(seed),
    )
}


fn session_keys(ed_key: ed25519::Public, sr_key: sr25519::Public) -> SessionKeys {
    SessionKeys {
        ed25519: ed_key,
        sr25519: sr_key,
    }
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
    let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
        vec![
            get_account_id_from_seed("Alice"),
            get_account_id_from_seed("Bob"),
            get_account_id_from_seed("Charlie"),
            get_account_id_from_seed("Dave"),
            get_account_id_from_seed("Eve"),
            get_account_id_from_seed("Ferdie"),
            get_account_id_from_seed("Alice//stash"),
            get_account_id_from_seed("Bob//stash"),
            get_account_id_from_seed("Charlie//stash"),
            get_account_id_from_seed("Dave//stash"),
            get_account_id_from_seed("Eve//stash"),
            get_account_id_from_seed("Ferdie//stash"),
        ]
    });

    const ENDOWMENT: Balance = 100 * XRT;
    const STASH: Balance = 1_000 * XRT;

    GenesisConfig {
        system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        indices: Some(IndicesConfig {
            ids: endowed_accounts.iter().cloned()
                .chain(initial_authorities.iter().map(|x| x.0.clone()))
                .collect::<Vec<_>>(),
        }),
        balances: Some(BalancesConfig {
            balances: endowed_accounts.iter().cloned()
                .map(|k| (k, ENDOWMENT))
                .chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
                .collect(),
            vesting: vec![],
        }),
        session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| (x.1.clone(), session_keys(x.3.clone(), x.2.clone()))).collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            minimum_validator_count: 2,
            validator_count: 7,
            offline_slash: Perbill::from_millionths(1_000_000),
            offline_slash_grace: 4,
            stakers: initial_authorities.iter().map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)).collect(),
            invulnerables: initial_authorities.iter().map(|x| x.1.clone()).collect(),
        }),
        sudo: Some(SudoConfig {
            key: root_key,
        }),
        babe: Some(BabeConfig {
            authorities: initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
        }),
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities.iter().map(|x| (x.3.clone(), 1)).collect(),
        }),
		im_online: Some(ImOnlineConfig {
			gossip_at: 0,
			last_new_era_start: 0,
		}),
    }
}

/// XRT token properties.
fn xrt_props() -> Properties {
    json!({"tokenDecimals": 9, "tokenSymbol": "XRT"}).as_object().unwrap().clone()
}

/// Robonomics testnet config. 
fn robonomics_config_genesis() -> GenesisConfig {
    let aira_babe: BabeId       = hex!["3ae9a59c2c2a0bf06d9a58a47eaef86a6ce8eee356eb5d29574739d0d0c33d30"].unchecked_into();
    let aira_grandpa: GrandpaId = hex!["30d3114363ff180bb295099c34fb30060e3b2df89617f7d76078b37d4d351cca"].unchecked_into();
    let aira_stash: AccountId   = hex!["0ab623ec23b0346976d8fb4eaf012035fda269077490cbfb6aae15cb31d43777"].unchecked_into();
    let aira_control: AccountId = hex!["16e4b93d965a27e50de7e27b6e9b8471186b4a463bbad8e2b0a398007098504e"].unchecked_into();

    let akru_babe: BabeId       = hex!["0ae0b0afc9783c4e7493e56d9572fc45c024e12bbef7d3abf2534c2d07acb81b"].unchecked_into();
    let akru_grandpa: GrandpaId = hex!["4327b538c4d3fd84cb875328adeee97ee0754dc1491c5a453c07031a40215b0e"].unchecked_into();
    let akru_stash: AccountId   = hex!["a26253010447e4a0ec7ddce034034a4ebcfb1317440fd458c21c592ddf8d0337"].unchecked_into();
    let akru_control: AccountId = hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].unchecked_into();

    testnet_genesis(
        vec![
          (aira_stash, aira_control.clone(), aira_babe, aira_grandpa),
          (akru_stash, akru_control.clone(), akru_babe, akru_grandpa),
        ],
        akru_control.clone(),
        Some(vec![aira_control, akru_control]),
    )
}

/// Robonomics testnet config. 
pub fn robonomics_testnet_config() -> ChainSpec {
    ChainSpec::from_embedded(include_bytes!("../../res/robonomics_testnet.json")).unwrap()
}

/*
/// Robonomics testnet config.
pub fn robonomics_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        "/ip4/95.216.202.55/tcp/30333/p2p/QmcYrdpTWSGLTVHMCMj27xaGkuqab1sHqXF33YdUrhS9Fp".into(),
    ];
    ChainSpec::from_genesis(
        "Robonomics",
        "robonomics_testnet",
        robonomics_config_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])),
        None,
        None,
        Some(xrt_props())
    )
}
*/

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            get_authority_keys_from_seed("Alice"),
        ],
        get_account_id_from_seed("Alice").into(),
        None,
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        development_config_genesis,
        vec![],
        None,
        None,
        None,
        Some(xrt_props())
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            get_authority_keys_from_seed("Alice"),
            get_authority_keys_from_seed("Bob"),
        ],
        get_account_id_from_seed("Alice").into(),
        None,
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        local_testnet_genesis,
        vec![],
        None,
        None,
        None,
        Some(xrt_props())
    )
}
