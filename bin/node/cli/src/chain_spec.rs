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
use authority_discovery_primitives::AuthorityId as AuthorityDiscoveryId;
use sp_runtime::{Perbill, traits::{Verify, IdentifyAccount}};
use primitives::{Pair, Public, crypto::UncheckedInto, sr25519};
use node_runtime::{
    GenesisConfig, SystemConfig, SessionConfig, BabeConfig, StakingConfig,
    IndicesConfig, ImOnlineConfig, BalancesConfig, GrandpaConfig, SudoConfig,
    AuthorityDiscoveryConfig,
    SessionKeys, StakerStatus, WASM_BINARY,
};
use node_runtime::constants::currency::*;
use node_primitives::{AccountId, Balance, Signature};
use telemetry::TelemetryEndpoints;
use hex_literal::hex;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const ROBONOMICS_PROTOCOL_ID: &str = "xrt";
const ROBONOMICS_PROPERTIES: &str = r#"
    {
        "tokenDecimals": 9,
        "tokenSymbol": "XRT"
    }"#;


type AccountPublic = <Signature as Verify>::Signer;

/// Specialised `ChainSpec`. This is a specialisation of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
enum ChainOpt {
    /// Whatever the current runtime is, with just Alice as an auth.
    Development,
    /// Whatever the current runtime is, with simple Alice/Bob auths.
    LocalTestnet,
    /// Robonomics public testnet.
    RobonomicsTestnet,
}

impl ChainOpt {
    /// Get an actual chain config from one of the alternatives.
    pub(crate) fn load(self) -> Result<ChainSpec, String> {
        Ok(match self {
            ChainOpt::Development => development_config(),
            ChainOpt::LocalTestnet => local_testnet_config(),
            ChainOpt::RobonomicsTestnet => robonomics_testnet_config(),
        })
    }

    pub(crate) fn from(s: &str) -> Option<Self> {
        match s {
            "dev" => Some(ChainOpt::Development),
            "local" => Some(ChainOpt::LocalTestnet),
            "" | "robonomics" => Some(ChainOpt::RobonomicsTestnet),
            _ => None,
        }
    }
}

pub fn load_spec(id: &str) -> Result<Option<ChainSpec>, String> {
    Ok(match ChainOpt::from(id) {
        Some(spec) => Some(spec.load()?),
        None => None,
    })
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(
    seed: &str
) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
    ImOnlineId,
    AuthorityDiscoveryId
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}

fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys { grandpa, babe, im_online, authority_discovery, }
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)>,
    endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
    let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
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
                (x.0.clone(), session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()))
            }).collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            validator_count: 10,
            minimum_validator_count: 3,
            stakers: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
            }).collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
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
        authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![],
        }),
    }
}

/// Robonomics testnet config. 
pub fn robonomics_testnet_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/robonomics_testnet.json")[..]).unwrap()
}

/*
/// Robonomics testnet config. 
fn robonomics_config_genesis() -> GenesisConfig {
    let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
        hex!["58cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
        hex!["58cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
        hex!["daf0535a46d8187446471bf619ea9104bda443366c526bf6f2cd4e9a1fcf5dd7"].unchecked_into(),
        hex!["36cced69f5f1f07856ff0daac944c52e286e10184e52be76ca9377bd0406d90b"].unchecked_into(),
        hex!["80de51e4432ed5e37b6438f499f3ec017f9577a37e68cb32d6c6a07540c36909"].unchecked_into(),
        hex!["80de51e4432ed5e37b6438f499f3ec017f9577a37e68cb32d6c6a07540c36909"].unchecked_into(),
    )];

    let endowed_accounts: Vec<AccountId> = vec![
        // 5Cakru1BpXPiezeD2LRZh3pJamHcbX9yZ13KLBxuqdTpgnYF
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into(),
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
        Some(ROBONOMICS_PROTOCOL_ID),
        Some(serde_json::from_str(ROBONOMICS_PROPERTIES).unwrap()),
        Default::default(),
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
        Some(ROBONOMICS_PROTOCOL_ID),
        Some(serde_json::from_str(ROBONOMICS_PROPERTIES).unwrap()),
        Default::default(),
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
        Some(ROBONOMICS_PROTOCOL_ID),
        Some(serde_json::from_str(ROBONOMICS_PROPERTIES).unwrap()),
        Default::default(),
    )
}
