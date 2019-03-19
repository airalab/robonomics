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

use primitives::{Ed25519AuthorityId as AuthorityId, ed25519};
use robonomics_runtime::{
    GenesisConfig, ConsensusConfig, SessionConfig, StakingConfig, TimestampConfig,
    IndicesConfig, BalancesConfig, FeesConfig, GrandpaConfig, SudoConfig,
    AccountId, Perbill
};
use substrate_service::{self, Properties};
use serde_json::json;

use substrate_keystore::pad_seed;
use substrate_telemetry::TelemetryEndpoints;

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

/// Helper function to generate AuthorityID from seed
pub fn get_account_id_from_seed(seed: &str) -> AccountId {
    let padded_seed = pad_seed(seed);
    // NOTE from ed25519 impl:
    // prefer pkcs#8 unless security doesn't matter -- this is used primarily for tests.
    ed25519::Pair::from_seed(&padded_seed).public().0.into()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, AuthorityId) {
    let padded_seed = pad_seed(seed);
    // NOTE from ed25519 impl:
    // prefer pkcs#8 unless security doesn't matter -- this is used primarily for tests.
    (
        get_account_id_from_seed(&format!("{}-stash", seed)),
        get_account_id_from_seed(seed),
        ed25519::Pair::from_seed(&padded_seed).public().0.into()
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

    const ENDOWMENT: u128 = 10_000_000 * XRT;
    const STASH: u128 = 100 * XRT;

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/robonomics_runtime.compact.wasm").to_vec(),
            authorities: initial_authorities.iter().map(|x| x.2.clone()).collect(),
        }),
        system: None,
        indices: Some(IndicesConfig {
            ids: endowed_accounts.clone(),
        }),
        balances: Some(BalancesConfig {
            existential_deposit: 1 * COASE,
            transfer_fee: 0,
            creation_fee: 0,
            balances: endowed_accounts.iter()
                .map(|&k| (k, ENDOWMENT))
                .chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
                .collect(),
            vesting: vec![],
        }),
        session: Some(SessionConfig {
            validators: initial_authorities.iter().map(|x| x.1.into()).collect(),
            session_length: 15,
            keys: initial_authorities.iter().map(|x| (x.1.clone(), x.2.clone())).collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            minimum_validator_count: 2,
            validator_count: 7,
            sessions_per_era: 60,
            bonding_duration: 60 * MINUTES,
            session_reward: Perbill::from_millionths(200_000),
            offline_slash: Perbill::from_millionths(1_000_000),
            current_offline_slash: 0,
            current_session_reward: 0,
            offline_slash_grace: 4,
            stakers: initial_authorities.iter().map(|x| (x.0.into(), x.1.into(), STASH)).collect(),
            invulnerables: initial_authorities.iter().map(|x| x.1.into()).collect(),
        }),
        timestamp: Some(TimestampConfig {
            period: SECS_PER_BLOCK / 2,
        }),
        sudo: Some(SudoConfig {
            key: root_key,
        }),
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
        }),
        fees: Some(FeesConfig {
            transaction_base_fee: 1 * GLUSHKOV,
            transaction_byte_fee: 50 * COASE,
        }),
    }
}

/// XRT token properties.
fn xrt_props() -> Properties {
    json!({"tokenDecimals": 9, "tokenSymbol": "XRT"}).as_object().unwrap().clone()
}

/// Robonomics testnet config. 
fn robonomics_config_genesis() -> GenesisConfig {
    let aira_stash   = ed25519::Public::from_ss58check("5HairafECXZHE4VBPwz2r5BVEYjK2MjGE5bRDL451BSCsfED").unwrap().0;
    let aira_control = ed25519::Public::from_ss58check("5DAirazvxYkdNUCQJY8uGz2KdSUJnEYzPRDb78ss1S8ewFQv").unwrap().0;
    let akru_stash   = ed25519::Public::from_ss58check("5HakruKnWQWa36am44tKu9hwDjkYCzaravUqjkerfpY6pQHi").unwrap().0;
    let akru_control = ed25519::Public::from_ss58check("5Dakru9P3kCScVXgoU2pN8dQU3US178msVxUatD122affWFt").unwrap().0;
    let khssnv_stash   = ed25519::Public::from_ss58check("5DKhSUPRtrPuNcH3a4WRkWpeWBhKrNpRzWTykmCELHV6dhRs").unwrap().0;
    let khssnv_control = ed25519::Public::from_ss58check("5CKhj6NsXY9exr2oCwK2t3v1cfXyWqg9qmAQ2LTM1psyZ6VR").unwrap().0;
    testnet_genesis(
        vec![
          (aira_stash.into(), aira_control.into(), aira_control.into()),
          (akru_stash.into(), akru_control.into(), akru_control.into()),
          (khssnv_stash.into(), khssnv_control.into(), khssnv_control.into()),
        ],
        akru_control.into(),
        Some(vec![aira_control.into(), akru_control.into(), khssnv_control.into()]),
    )
}

/*
/// Robonomics testnet config. 
pub fn robonomics_testnet_config() -> Result<ChainSpec, String> {
    ChainSpec::from_embedded(include_bytes!("../res/robonomics.json"))
}
*/

/// Robonomics testnet config.
pub fn robonomics_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        "/ip4/164.132.111.49/tcp/30333/p2p/QmbPgV4iTsWHhrZDTPU5g1YtxJ11PcGC3f9oMTaNLUvJ6m".into(),
        "/ip4/54.38.53.77/tcp/30333/p2p/QmPVJKr8TkLkDF98BYyySxe2bVJ2BY9epXvmdCkExwtp2Q".into(),
        "/ip4/139.162.132.141/tcp/30333/p2p/QmUQhKfBKfb5jMstpQ5kUER5HzVsLLJysyewnFDHEveHkh".into(),
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
