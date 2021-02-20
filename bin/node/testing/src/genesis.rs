// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Genesis Configuration.

use crate::keyring::*;
use sp_keyring::{Ed25519Keyring, Sr25519Keyring};
use node_runtime::{
	GenesisConfig, BabeConfig, BalancesConfig, SystemConfig, GrandpaConfig,
    wasm_binary_unwrap,
    constants::currency::*,
};
use node_primitives::AccountId;
use sp_core::ChangesTrieConfiguration;

/// Create genesis runtime configuration for tests.
pub fn config(support_changes_trie: bool, code: Option<&[u8]>) -> GenesisConfig {
	config_endowed(support_changes_trie, code, Default::default())
}

/// Create genesis runtime configuration for tests with some extra
/// endowed accounts.
pub fn config_endowed(
	support_changes_trie: bool,
	code: Option<&[u8]>,
	extra_endowed: Vec<AccountId>,
) -> GenesisConfig {

	let mut endowed = vec![
		(alice(), 111 * XRT),
		(bob(), 100 * XRT),
		(charlie(), 100_000_000 * XRT),
		(dave(), 111 * XRT),
		(eve(), 101 * XRT),
		(ferdie(), 100 * XRT),
	];

	endowed.extend(
		extra_endowed.into_iter().map(|endowed| (endowed, 100*XRT))
	);

    let keys = vec![
		to_session_keys(&Ed25519Keyring::Alice, &Sr25519Keyring::Alice),
		to_session_keys(&Ed25519Keyring::Bob, &Sr25519Keyring::Bob),
		to_session_keys(&Ed25519Keyring::Charlie, &Sr25519Keyring::Charlie),
    ];

	GenesisConfig {
		frame_system: Some(SystemConfig {
			changes_trie_config: if support_changes_trie { Some(ChangesTrieConfiguration {
				digest_interval: 2,
				digest_levels: 2,
			}) } else { None },
			code: code.map(|x| x.to_vec()).unwrap_or_else(|| wasm_binary_unwrap().to_vec()),
		}),
		pallet_balances: Some(BalancesConfig {
			balances: endowed,
		}),
		pallet_babe: Some(BabeConfig {
            authorities: keys 
                .iter()
                .map(|x| (x.babe.clone(), 1))
                .collect(),
        }),
		pallet_grandpa: Some(GrandpaConfig {
            authorities: keys 
                .iter()
                .map(|x| (x.grandpa.clone(), 1))
                .collect(),
		}),
		pallet_sudo: Some(Default::default()),
	}
}
