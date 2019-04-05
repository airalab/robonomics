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

use primitives::{ed25519, sr25519, crypto::Pair};
use robonomics_runtime::{
    GenesisConfig, ConsensusConfig, SessionConfig, StakingConfig, TimestampConfig,
    IndicesConfig, BalancesConfig, GrandpaConfig, SudoConfig,
    AccountId, AuthorityId, Perbill, StakerStatus
};
use substrate_service::{self, Properties};
use serde_json::json;

/*
use hex_literal::{hex, hex_impl};
use primitives::crypto::UncheckedInto;
use telemetry::TelemetryEndpoints;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
*/

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

/// Helper function to generate AuthorityId from seed
pub fn get_session_key_from_seed(seed: &str) -> AuthorityId {
    ed25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, AuthorityId) {
    (
        get_account_id_from_seed(&format!("{}//stash", seed)),
        get_account_id_from_seed(seed),
        get_session_key_from_seed(seed)
    )
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, AuthorityId)>,
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
        ]
    });

    const COASE: u128 = 1_000;
    const GLUSHKOV: u128 = 1_000 * COASE;    // assume this is worth about a cent.
    const XRT: u128 = 1_000 * GLUSHKOV;

    const SECS_PER_BLOCK: u64 = 4;
    const MINUTES: u64 = 60 / SECS_PER_BLOCK;

    const ENDOWMENT: u128 = 100 * XRT;
    const STASH: u128 = 1_000 * XRT;

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!("../../runtime/wasm/target/wasm32-unknown-unknown/release/robonomics_runtime.compact.wasm").to_vec(),
            authorities: initial_authorities
                .iter()
                .map(|x| x.2.clone())
                .collect(),
        }),
        system: None,
        indices: Some(IndicesConfig {
            ids: endowed_accounts.clone(),
        }),
        balances: Some(BalancesConfig {
            existential_deposit: 1 * COASE,
            transaction_base_fee: 1 * GLUSHKOV,
            transaction_byte_fee: 50 * COASE,
            transfer_fee: 0,
            creation_fee: 0,
            balances: endowed_accounts
                .iter()
                .map(|k| (k.clone(), ENDOWMENT))
                .chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
                .collect(),
            vesting: vec![],
        }),
        session: Some(SessionConfig {
            validators: initial_authorities
                .iter()
                .map(|x| x.1.clone())
                .collect(),
            session_length: 15,
            keys: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), x.2.clone()))
                .collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            minimum_validator_count: 2,
            validator_count: 7,
            sessions_per_era: 10,
            bonding_duration: 10 * MINUTES,
            current_session_reward: 0,
            session_reward: Perbill::from_millionths(200_000),
            offline_slash: Perbill::from_millionths(1_000_000),
            offline_slash_grace: 4,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities
                .iter()
                .map(|x| x.1.clone())
                .collect(),
        }),
        timestamp: Some(TimestampConfig {
            minimum_period: SECS_PER_BLOCK / 2,
        }),
        sudo: Some(SudoConfig {
            key: root_key,
        }),
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.2.clone(), 1))
                .collect(),
        }),
    }
}

/// XRT token properties.
fn xrt_props() -> Properties {
    json!({"tokenDecimals": 9, "tokenSymbol": "XRT"}).as_object().unwrap().clone()
}

/// Robonomics testnet config. 
pub fn robonomics_testnet_config() -> ChainSpec {
    ChainSpec::from_embedded(include_bytes!("../../res/robonomics_testnet.json")).unwrap()
}

/*
/// Robonomics testnet config. 
fn robonomics_config_genesis() -> GenesisConfig {
    let aira_auth: AuthorityId  = hex!["30d3114363ff180bb295099c34fb30060e3b2df89617f7d76078b37d4d351cca"].unchecked_into();
    let aira_stash: AccountId   = hex!["0ab623ec23b0346976d8fb4eaf012035fda269077490cbfb6aae15cb31d43777"].unchecked_into();
    let aira_control: AccountId = hex!["16e4b93d965a27e50de7e27b6e9b8471186b4a463bbad8e2b0a398007098504e"].unchecked_into();

    let akru_auth: AuthorityId  = hex!["4327b538c4d3fd84cb875328adeee97ee0754dc1491c5a453c07031a40215b0e"].unchecked_into();
    let akru_stash: AccountId   = hex!["a26253010447e4a0ec7ddce034034a4ebcfb1317440fd458c21c592ddf8d0337"].unchecked_into();
    let akru_control: AccountId = hex!["16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"].unchecked_into();

    testnet_genesis(
        vec![
          (aira_stash, aira_control.clone(), aira_auth),
          (akru_stash, akru_control.clone(), akru_auth),
        ],
        akru_control.clone(),
        Some(vec![aira_control, akru_control]),
    )
}

/// Robonomics testnet config.
pub fn robonomics_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        "/ip4/164.132.111.49/tcp/30333/p2p/QmbPgV4iTsWHhrZDTPU5g1YtxJ11PcGC3f9oMTaNLUvJ6m".into(),
        "/ip4/54.38.53.77/tcp/30333/p2p/QmPVJKr8TkLkDF98BYyySxe2bVJ2BY9epXvmdCkExwtp2Q".into(),
        "/ip4/139.162.132.141/tcp/30333/p2p/QmUQhKfBKfb5jMstpQ5kUER5HzVsLLJysyewnFDHEveHkh".into(),
        "/ip6/2001:41d0:401:3100::34e6/tcp/30333/p2p/QmbPgV4iTsWHhrZDTPU5g1YtxJ11PcGC3f9oMTaNLUvJ6m".into(),
        "/ip6/fc6c:99a2:171a:f36a:8cd0:cc6b:efb7:8bb4/tcp/30333/p2p/QmbPgV4iTsWHhrZDTPU5g1YtxJ11PcGC3f9oMTaNLUvJ6m".into(),
        "/ip6/fcaa:9c13:6ea4:4b92:8b9b:9:2390:52c1/tcp/30333/p2p/QmduvgCG1Tfj2P1oLRDjGzrhto4PKmSUtCGCNKoKEvNHxL".into(),
        "/ip6/fc59:cb90:5852:7fe3:a759:57d9:f546:a3a8/tcp/30333/p2p/QmUQhKfBKfb5jMstpQ5kUER5HzVsLLJysyewnFDHEveHkh".into(),
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
