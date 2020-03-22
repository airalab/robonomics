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

use serde::{Serialize, Deserialize};
use sc_chain_spec::ChainSpecExtension;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_consensus_babe::AuthorityId as BabeId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_runtime::{Perbill, traits::{Verify, IdentifyAccount}};
use sp_core::{Pair, Public, crypto::UncheckedInto, sr25519};
use robonomics_runtime::{
    GenesisConfig, SystemConfig, SessionConfig, BabeConfig, StakingConfig,
    IndicesConfig, ImOnlineConfig, BalancesConfig, GrandpaConfig, SudoConfig,
    AuthorityDiscoveryConfig, SessionKeys, StakerStatus,
};
use node_primitives::{AccountId, Balance, Signature, Block};
use sc_telemetry::TelemetryEndpoints;
use hex_literal::hex;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

const ROBONOMICS_PROTOCOL_ID: &str = "xrt";
const ROBONOMICS_PROPERTIES: &str = r#"
    {
        "tokenDecimals": 9,
        "tokenSymbol": "XRT"
    }"#;

const IPCI_PROTOCOL_ID: &str = "mito";
const IPCI_PROPERTIES: &str = r#"
    {
        "tokenSymbol": "MITO"
    }"#;

type AccountPublic = <Signature as Verify>::Signer;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<
    GenesisConfig,
    Extensions,
>;

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
    BabeId,
    GrandpaId,
    ImOnlineId,
    AuthorityDiscoveryId
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}

fn session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys { babe, grandpa, im_online, authority_discovery, }
}

pub fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId, AuthorityDiscoveryId)>,
    endowed_accounts: Option<Vec<AccountId>>,
    sudo_key: AccountId,
) -> GenesisConfig {
    const ENDOWMENT: Balance = 1_000_000_000_000_000_000;

    let endowed_accounts: Vec<(AccountId, Balance)> = endowed_accounts.unwrap_or_else(|| {
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
    }).iter().cloned().map(|acc| (acc, ENDOWMENT)).collect();

    make_genesis(
        initial_authorities,
        endowed_accounts,
        sudo_key,
        robonomics_runtime::WASM_BINARY.to_vec(),
    )
}

/// Helper function to create GenesisConfig
pub fn make_genesis(
    initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId, AuthorityDiscoveryId)>,
    endowed_accounts: Vec<(AccountId, Balance)>,
    sudo_key: AccountId,
    code: Vec<u8>,
) -> GenesisConfig {
    const STASH: Balance = 1_000_000;
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code,
            changes_trie_config: Default::default(),
        }),
        pallet_indices: Some(IndicesConfig {
            indices: vec![],
        }),
        pallet_balances: Some(BalancesConfig {
            balances: endowed_accounts.iter().cloned()
                .chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
                .collect(),
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.0.clone(), session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()))
            }).collect::<Vec<_>>(),
        }),
        pallet_staking: Some(StakingConfig {
            validator_count: 10,
            minimum_validator_count: 3,
            stakers: initial_authorities.iter().map(|x| {
                (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
            }).collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            .. Default::default()
        }),
        pallet_babe: Some(BabeConfig {
            authorities: vec![], 
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![], 
        }),
        pallet_im_online: Some(ImOnlineConfig {
            keys: vec![],
        }),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![],
        }),
        pallet_sudo: Some(SudoConfig {
            key: sudo_key,
        }),
    }
}
/*

/// Robonomics testnet config. 
pub fn robonomics_testnet_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/robonomics_testnet.json")[..]).unwrap()
}
*/

/// Robonomics testnet genesis. 
fn robonomics_testnet_genesis() -> GenesisConfig {
    let initial_authorities = vec![(
//     5FyXbH8PgKRrveqXJYjSLnLSeHiK6pov8kQKaUVGN5XHy4MA
        hex!["acfe268b8276a4ed8924aef1441eb05334522f6c6c7487c12d71b0fb2ab28d37"].into(),
//     5C7d35rhGpKP8UzXFwgdRzFnu6e5AiPx765XBytu7xakaHor
        hex!["0239825db490fce751ee12d6cf67a59e1278f52fd82d5a9033f773924a709348"].into(),

        hex!["304d073f2c918bff832e6f61949e1b7ac38fb8d57da1157f30d10e1406cc9137"].unchecked_into(),
        hex!["85ddf5a932937c65694146577b50375668055ff435400310ca5344edf0f50b83"].unchecked_into(),
        hex!["64063c2394c0a8250e5dc03286ead10e2efcda342467fbcbdf5f03d0e39aae19"].unchecked_into(),
        hex!["926165922b8174c8446503a9bdc6581f4a658393169ea890c291fa2ad6b0b345"].unchecked_into(),
    )];

    let sudo_key: AccountId =
//     5FvJfouVa2y2LFMSG5sRPzxrTQ2Vd8NhzovsoUKYG9n2hQtK
        hex!["aa88ea58465ffbcf716c3d57fab7c29b6d7c7243133b024e61556b92512a4765"].into();

    make_genesis(
        initial_authorities,
        robonomics_runtime::constants::currency::STAKE_HOLDERS.to_vec(),
        sudo_key,
        robonomics_runtime::WASM_BINARY.to_vec(),
    )
}

/// Robonomics testnet config.
pub fn robonomics_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        // validator-01
        "/ip4/51.15.132.76/tcp/30363/p2p/QmRg7aTH3ZBbcxmXfMn4CgEEBcnJzeC6UewFco7Dxh2M84".into(),
        // validator-02
        "/ip4/188.127.249.219/tcp/30363/p2p/QmepKcvG1henJm4SxnRfr17Uq2A2t5mYDZZDRYBqHohSND".into(),
        // validator-03
        "/ip4/167.71.148.38/tcp/30363/p2p/Qmep2VYsMfiBQnTMHVk6AddygMysiK379VP48hKZCoWtWT".into(),
        // akru
        "/ip4/95.216.202.55/tcp/30363/p2p/QmPrm3QaNv4Ls2DdAmsS1AoEbbYGrtqiyjxAVdc6mjEY5N".into(),
    ];
    ChainSpec::from_genesis(
        "Robonomics",
        "robonomics_testnet",
        robonomics_testnet_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])),
        Some(ROBONOMICS_PROTOCOL_ID),
        Some(serde_json::from_str(ROBONOMICS_PROPERTIES).unwrap()),
        Default::default(),
    )
}

/*
/// IPCI blockchain config. 
pub fn ipci_config() -> ChainSpec {
    ChainSpec::from_json_bytes(&include_bytes!("../res/ipci.json")[..]).unwrap()
}
*/

/// IPCI blockchain genesis. 
fn ipci_genesis() -> GenesisConfig {
    let initial_authorities = vec![(
        // akru.me
        hex!["58cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
        hex!["58cdc7ef880c80e8475170f206381d2cb13a87c209452fc6d8a1e14186d61b28"].into(),
        hex!["36cced69f5f1f07856ff0daac944c52e286e10184e52be76ca9377bd0406d90b"].unchecked_into(),
        hex!["daf0535a46d8187446471bf619ea9104bda443366c526bf6f2cd4e9a1fcf5dd7"].unchecked_into(),
        hex!["80de51e4432ed5e37b6438f499f3ec017f9577a37e68cb32d6c6a07540c36909"].unchecked_into(),
        hex!["80de51e4432ed5e37b6438f499f3ec017f9577a37e68cb32d6c6a07540c36909"].unchecked_into(),
    )];

    let sudo_key: AccountId =
        // 5Cakru1BpXPiezeD2LRZh3pJamHcbX9yZ13KLBxuqdTpgnYF
        hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].into();

    make_genesis(
        initial_authorities,
        ipci_runtime::constants::currency::STAKE_HOLDERS.to_vec(),
        sudo_key,
        ipci_runtime::WASM_BINARY.to_vec(),
    )
}

/// IPCI config.
pub fn ipci_config() -> ChainSpec {
    let boot_nodes = vec![
        // akru
        "/ip4/95.216.202.55/tcp/30363/p2p/QmPrm3QaNv4Ls2DdAmsS1AoEbbYGrtqiyjxAVdc6mjEY5N".into(),
    ];
    ChainSpec::from_genesis(
        "IPCI",
        "ipci",
        ipci_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])),
        Some(IPCI_PROTOCOL_ID),
        Some(serde_json::from_str(IPCI_PROPERTIES).unwrap()),
        Default::default(),
    )
}

fn development_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![get_authority_keys_from_seed("Alice")],
        None,
        get_account_id_from_seed::<sr25519::Public>("Alice"),
    )
}

/// Development config (single validator Alice)
pub fn development_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        development_testnet_genesis,
        vec![],
        None,
        None,
        None,
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
        get_account_id_from_seed::<sr25519::Public>("Alice"),
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
        Default::default(),
    )
}
