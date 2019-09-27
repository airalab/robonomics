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

/// Robonomics testnet config. 
pub fn robonomics_testnet_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../../res/robonomics_testnet.json")[..]).unwrap()
}

/// XRT token properties.
fn xrt_props() -> Properties {
    json!({"tokenDecimals": 9, "tokenSymbol": "XRT"}).as_object().unwrap().clone()
}

/*
/// Robonomics testnet config. 
fn robonomics_config_genesis() -> GenesisConfig {
    let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId)> = vec![(
        // validator-01 
        hex!["847204aef88ce7693048264d17e6a78656dc14d812a5979abb81742a639f8461"].unchecked_into(),
        hex!["d61a40b1d11183243afcbaf2b74dfdd5faa8858f95ec401d9ee102db136a9c19"].unchecked_into(),
        hex!["ae29a6e24e3cfdee27ac1d40324d1497dd27839a301e9ce2ea5d93e4bdb49088"].unchecked_into(),
        hex!["643b13ab6205f0c373c566ad70775253db49287f87f2250539131f274598dd23"].unchecked_into(),
        hex!["8c131749823ccb3ebbe9f38564da0543ee3e4717a90a8edf67e93c07b5f5b513"].unchecked_into(),
        ),(
        // validator-02
        hex!["e237342b0088a0dc5103b7985e636c45eb61cf9aca578f74194209eb3c333e10"].unchecked_into(),
        hex!["426ce030cd1794c92d019b571088126e26e183ed07508c3ab70e231367fc4165"].unchecked_into(),
        hex!["4bce118897776da9e99bf271d44172ea77bb48fc7e73226d2963302077e0e5f2"].unchecked_into(),
        hex!["96680f0e3446720e605ea1338ddfca56c1bd0412dd0ff705d24bd243a49f5b2d"].unchecked_into(),
        hex!["00a8a49ce6ac03a1f08d109f6d61b782efad71f526efa1d02b66bf940963c65e"].unchecked_into(),
        ),(
        // validator-03
        hex!["fc9d09d51b60eba639a694b1b7aacdc96175a1ede2abb38d33104ed7a658282c"].unchecked_into(),
        hex!["2ce954ca2837694f46c7ffb77ddd58afe5d8e038c8224de3769cdbe4cf0ed41a"].unchecked_into(),
        hex!["7d2233debe742ad6b57826b39be45eb098875fc02854bd52cda3a061b922c613"].unchecked_into(),
        hex!["34b9a5af40d2ee4e0b3f2b49f8d8fb7024dfa2670720eed83942e5d36848cc3c"].unchecked_into(),
        hex!["489859d83afdf7418f8c00540003ae4b9768df30faae731b24808b966c154f37"].unchecked_into(),
        )];

    let endowed_accounts: Vec<AccountId> = vec![
        // 5Cakru1BpXPiezeD2LRZh3pJamHcbX9yZ13KLBxuqdTpgnYF  
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].unchecked_into(),
        ];
    testnet_genesis(
        initial_authorities,
        Some(endowed_accounts),
    )
}

/// Robonomics testnet config.
pub fn robonomics_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        // validator-01
        "/ip4/51.15.132.76/tcp/30363/p2p/QmRg7aTH3ZBbcxmXfMn4CgEEBcnJzeC6UewFco7Dxh2M84".into(),
        // validator-02
        "/ip4/188.127.249.219/tcp/30363/p2p/QmYp26uKLyDesPzCS5Y3w44NUKZmDz87F3ywJkhHhh9SUf".into(),
        // validator-03
        "/ip4/167.71.148.38/tcp/30363/p2p/Qmep2VYsMfiBQnTMHVk6AddygMysiK379VP48hKZCoWtWT".into(),
        // akru
        "/ip4/95.216.202.55/tcp/30363/p2p/QmPrm3QaNv4Ls2DdAmsS1AoEbbYGrtqiyjxAVdc6mjEY5N".into(),
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
