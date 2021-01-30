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

use node_primitives::{AccountId, Balance, Block, Signature};
use node_runtime::{
    wasm_binary_unwrap, BabeConfig, BalancesConfig, GenesisConfig, GrandpaConfig, SudoConfig,
    SystemConfig,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Robonomics runtime family chains.
pub enum RobonomicsFamily {
    /// Development chain (used for local tests only).
    Development,
    /// Robonomics Network parachain (https://telemetry.polkadot.io/#list/Robonomics).
    Parachain,
}

/// Robonomics family chains idetify.
pub trait RobonomicsChain {
    fn family(&self) -> RobonomicsFamily;
}

impl RobonomicsChain for Box<dyn sc_chain_spec::ChainSpec> {
    fn family(&self) -> RobonomicsFamily {
        if self.id() == "dev" {
            return RobonomicsFamily::Development;
        }

        RobonomicsFamily::Parachain
    }
}

type AccountPublic = <Signature as Verify>::Signer;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// Helper function to generate a crypto pair from seed
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
fn get_authority_keys_from_seed(seed: &str) -> (AccountId, BabeId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<GrandpaId>(seed),
    )
}

fn development_genesis(
    initial_authorities: Vec<(AccountId, BabeId, GrandpaId)>,
    endowed_accounts: Option<Vec<AccountId>>,
    sudo_key: AccountId,
) -> GenesisConfig {
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
            ]
        })
        .iter()
        .cloned()
        .map(|acc| (acc, ENDOWMENT))
        .collect();

    mk_genesis(
        initial_authorities,
        endowed_accounts,
        sudo_key,
        wasm_binary_unwrap().to_vec(),
    )
}

/// Helper function to create GenesisConfig
fn mk_genesis(
    initial_authorities: Vec<(AccountId, BabeId, GrandpaId)>,
    balances: Vec<(AccountId, Balance)>,
    sudo_key: AccountId,
    code: Vec<u8>,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code,
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig { balances }),
        pallet_babe: Some(BabeConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.2.clone(), 1))
                .collect(),
        }),
        pallet_sudo: Some(SudoConfig { key: sudo_key }),
    }
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    let genesis = || {
        development_genesis(
            vec![get_authority_keys_from_seed("Alice")],
            None,
            get_account_id_from_seed::<sr25519::Public>("Alice"),
        )
    };
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        genesis,
        vec![],
        None,
        None,
        None,
        Default::default(),
    )
}
