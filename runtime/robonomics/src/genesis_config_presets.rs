///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
//! Runtime genesis config presets

use crate::common::{currency::XRT, xcm_version, AccountId, AuraId};
use crate::*;

use alloc::{vec, vec::Vec};
use cumulus_primitives_core::ParaId;
use frame_support::build_struct_json_patch;
use sp_genesis_builder::PresetId;
use sp_keyring::Sr25519Keyring;
use xcm::latest::{
    prelude::{Location, NetworkId},
    WESTEND_GENESIS_HASH,
};

pub const ROBONOMICS_PARA_ID: ParaId = ParaId::new(2048);
pub const RELAY_ASSET_ID: u32 = u32::MAX - 1;

fn robonomics_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    endowment: Balance,
    id: ParaId,
    relay: Option<NetworkId>,
    links: Vec<(AssetId, Location)>,
) -> serde_json::Value {
    build_struct_json_patch!(RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, endowment))
                .collect(),
        },
        parachain_info: ParachainInfoConfig { parachain_id: id },
        collator_selection: CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: 32 * XRT,
        },
        session: SessionConfig {
            keys: invulnerables
                .into_iter()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),          // account id
                        acc,                  // validator id
                        SessionKeys { aura }, // session keys
                    )
                })
                .collect(),
        },
        polkadot_xcm: PolkadotXcmConfig {
            safe_xcm_version: Some(xcm_version::SAFE_XCM_VERSION)
        },
        xcm_info: XcmInfoConfig {
            relay: relay.unwrap_or(NetworkId::Kusama),
            links,
        },
        sudo: SudoConfig {
            key: Some(Sr25519Keyring::Alice.to_account_id())
        },
    })
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
    let patch = match id.as_ref() {
        sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => robonomics_genesis(
            // initial collators.
            vec![
                (
                    Sr25519Keyring::Alice.to_account_id(),
                    Sr25519Keyring::Alice.public().into(),
                ),
                (
                    Sr25519Keyring::Bob.to_account_id(),
                    Sr25519Keyring::Bob.public().into(),
                ),
            ],
            Sr25519Keyring::well_known()
                .map(|k| k.to_account_id())
                .collect(),
            1_000 * XRT,
            ROBONOMICS_PARA_ID,
            Some(NetworkId::ByGenesis(WESTEND_GENESIS_HASH)),
            vec![(RELAY_ASSET_ID, Location::parent())],
        ),
        sp_genesis_builder::DEV_RUNTIME_PRESET => robonomics_genesis(
            // initial collators.
            vec![(
                Sr25519Keyring::Alice.to_account_id(),
                Sr25519Keyring::Alice.public().into(),
            )],
            vec![
                Sr25519Keyring::Alice.to_account_id(),
                Sr25519Keyring::Bob.to_account_id(),
                Sr25519Keyring::AliceStash.to_account_id(),
                Sr25519Keyring::BobStash.to_account_id(),
            ],
            1_000 * XRT,
            ROBONOMICS_PARA_ID,
            None,
            vec![(RELAY_ASSET_ID, Location::parent())],
        ),
        _ => return None,
    };

    Some(
        serde_json::to_string(&patch)
            .expect("serialization to json is expected to work. qed.")
            .into_bytes(),
    )
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
    vec![
        PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
        PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
    ]
}
