use crate::{
    AuraConfig, AuraId, BalancesConfig, DemocracyConfig, GrandpaConfig, GrandpaId,
    RuntimeGenesisConfig, SudoConfig, SystemConfig,
};
use robonomics_primitives::{AccountId, Balance, CommunityAccount};

use scale_info::prelude::format;
use serde_json::Value;
use sp_core::{sr25519, Pair, Public};
use sp_genesis_builder::PresetId;
use sp_keyring::Sr25519Keyring;
use sp_runtime::traits::{IdentifyAccount, Verify};

extern crate alloc;
use alloc::{vec, vec::Vec};

/// General signer type.
type AccountPublic = <robonomics_primitives::Signature as Verify>::Signer;

fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> robonomics_primitives::AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

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
) -> Value {
    let config = RuntimeGenesisConfig {
        system: SystemConfig {
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
    };

    serde_json::to_value(config).expect("Could not build genesis config.")
}

pub fn genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    endowed_accounts: Option<Vec<AccountId>>,
    sudo_key: AccountId,
) -> Value {
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

    devnet_genesis(initial_authorities, endowed_accounts, sudo_key)
}

pub fn get_preset(id: &PresetId) -> Option<vec::Vec<u8>> {
    let patch = match id.as_ref() {
        // sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_testnet_genesis(),
        sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
        _ => return None,
    };
    Some(
        serde_json::to_string(&patch)
            .expect("serialization to json is expected to work. qed.")
            .into_bytes(),
    )
}

pub fn preset_names() -> Vec<PresetId> {
    vec![
        PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
        PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
    ]
}

fn development_config_genesis() -> Value {
    genesis(
        // initial collators.
        vec![
            get_authority_keys_from_seed("Alice"),
            get_authority_keys_from_seed("Bob"),
        ],
        None,
        Sr25519Keyring::Alice.to_account_id(),
    )
}
