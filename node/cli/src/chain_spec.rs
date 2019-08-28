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
use im_online::sr25519::AuthorityId as ImOnlineId;
use primitives::{Pair, Public, crypto::UncheckedInto};
use node_runtime::{
    GenesisConfig, SystemConfig, SessionConfig, BabeConfig, StakingConfig,
    IndicesConfig, ImOnlineConfig, BalancesConfig, GrandpaConfig, SudoConfig,
    AuthorityDiscoveryConfig, SessionKeys, Perbill, StakerStatus, WASM_BINARY,
};
use node_runtime::constants::currency::*;
use node_runtime::types::{AccountId, Balance};
use substrate_service::{self, Properties};
use serde_json::json;
use hex_literal::hex;
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

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId) {
    (
        get_from_seed::<AccountId>(&format!("{}//stash", seed)),
        get_from_seed::<AccountId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
    )
}

fn session_keys(grandpa: GrandpaId, babe: BabeId, im_online: ImOnlineId) -> SessionKeys {
    SessionKeys { grandpa, babe, im_online, }
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId)>,
    endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
    let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
        vec![
            get_from_seed::<AccountId>("Alice"),
            get_from_seed::<AccountId>("Bob"),
            get_from_seed::<AccountId>("Charlie"),
            get_from_seed::<AccountId>("Dave"),
            get_from_seed::<AccountId>("Eve"),
            get_from_seed::<AccountId>("Ferdie"),
            get_from_seed::<AccountId>("Alice//stash"),
            get_from_seed::<AccountId>("Bob//stash"),
            get_from_seed::<AccountId>("Charlie//stash"),
            get_from_seed::<AccountId>("Dave//stash"),
            get_from_seed::<AccountId>("Eve//stash"),
            get_from_seed::<AccountId>("Ferdie//stash"),
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
            keys: initial_authorities.iter().map(|x| {
                (x.1.clone(), session_keys(x.2.clone(), x.3.clone(), x.4.clone()))
            }).collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            validator_count: 10,
            minimum_validator_count: 3,
            stakers: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
            }).collect(),
            invulnerables: initial_authorities.iter().map(|x| x.1.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            .. Default::default()
        }),
        sudo: Some(SudoConfig {
            key: endowed_accounts[0].clone(),
        }),
        babe: Some(BabeConfig {
            authorities: vec![], 
        }),
        grandpa: Some(GrandpaConfig {
            authorities: vec![], 
        }),
        im_online: Some(ImOnlineConfig {
            keys: vec![],
        }),
        authority_discovery: Some(AuthorityDiscoveryConfig{
            keys: vec![],
        }),
    }
}

/// XRT token properties.
fn xrt_props() -> Properties {
    json!({"tokenDecimals": 9, "tokenSymbol": "XRT"}).as_object().unwrap().clone()
}

/// Robonomics testnet config. 
fn robonomics_config_genesis() -> GenesisConfig {
    let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId)> = vec![(
        // airalab
        hex!["0ab623ec23b0346976d8fb4eaf012035fda269077490cbfb6aae15cb31d43777"].unchecked_into(),
        hex!["16e4b93d965a27e50de7e27b6e9b8471186b4a463bbad8e2b0a398007098504e"].unchecked_into(),
        hex!["174259e12de764a3d49c63c1b82494d1589f2a6d1547916dc3989bbfa96074ac"].unchecked_into(),
        hex!["389693262d286dc33a6dbd828d544eb72306e80646985a3c4b5b1f3bd93bc25a"].unchecked_into(),
        hex!["487faccb600b21904201a97ca8ba9723e850ff6fb45909bc330b030ac508dd4e"].unchecked_into(),
        ),(
        // akru
        hex!["a26253010447e4a0ec7ddce034034a4ebcfb1317440fd458c21c592ddf8d0337"].unchecked_into(),
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].unchecked_into(),
        hex!["ae29a6e24e3cfdee27ac1d40324d1497dd27839a301e9ce2ea5d93e4bdb49088"].unchecked_into(),
        hex!["643b13ab6205f0c373c566ad70775253db49287f87f2250539131f274598dd23"].unchecked_into(),
        hex!["8c131749823ccb3ebbe9f38564da0543ee3e4717a90a8edf67e93c07b5f5b513"].unchecked_into(),
        )];

    let endowed_accounts: Vec<AccountId> = vec![
        // akru
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].unchecked_into(),
        ];
    testnet_genesis(
        initial_authorities,
        Some(endowed_accounts),
    )
}

/*
/// Robonomics testnet config. 
pub fn robonomics_testnet_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../../res/robonomics_testnet.json")[..]).unwrap()
}
*/

/// Robonomics testnet config.
pub fn robonomics_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        "/ip4/95.216.202.55/tcp/30363/p2p/QmPrm3QaNv4Ls2DdAmsS1AoEbbYGrtqiyjxAVdc6mjEY5N".into()
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

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            get_authority_keys_from_seed("Alice"),
        ],
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
