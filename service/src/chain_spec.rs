// Copyright 2018 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use primitives::{AuthorityId, ed25519};
use node_primitives::AccountId;
use node_runtime::{GenesisConfig, ConsensusConfig, TimestampConfig, BalancesConfig};
use service::ChainSpec;

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

fn testnet_genesis(initial_authorities: Vec<AuthorityId>, endowed_accounts: Vec<AccountId>) -> GenesisConfig {
	GenesisConfig {
		consensus: Some(ConsensusConfig {
			code: include_bytes!("../../runtime/wasm/target/wasm32-unknown-unknown/release/node_runtime.compact.wasm").to_vec(),
			authorities: initial_authorities.clone(),
		}),
		system: None,
		timestamp: Some(TimestampConfig {
			period: 5,					// 5 second block time.
		}),
		balances: Some(BalancesConfig {
			transaction_base_fee: 1,
			transaction_byte_fee: 0,
			existential_deposit: 500,
			transfer_fee: 0,
			creation_fee: 0,
			reclaim_rebate: 0,
			balances: endowed_accounts.iter().map(|&k|(k, (1 << 60))).collect(),
		}),
	}
}

fn development_config_genesis() -> GenesisConfig {
	testnet_genesis(vec![
		ed25519::Pair::from_seed(b"Alice                           ").public().into(),
	], vec![
		ed25519::Pair::from_seed(b"Alice                           ").public().0.into(),
		ed25519::Pair::from_seed(b"Bob                             ").public().0.into(),
		ed25519::Pair::from_seed(b"Charlie                         ").public().0.into(),
		ed25519::Pair::from_seed(b"Dave                            ").public().0.into(),
		ed25519::Pair::from_seed(b"Eve                             ").public().0.into(),
		ed25519::Pair::from_seed(b"Ferdie                          ").public().0.into(),
	])
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec<GenesisConfig> {
	ChainSpec::from_genesis("Development", "development", development_config_genesis, vec![], None, None, None)
}

fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(vec![
		ed25519::Pair::from_seed(b"Alice                           ").public().into(),
		ed25519::Pair::from_seed(b"Bob                             ").public().into(),
	], vec![
		ed25519::Pair::from_seed(b"Alice                           ").public().0.into(),
		ed25519::Pair::from_seed(b"Bob                             ").public().0.into(),
		ed25519::Pair::from_seed(b"Charlie                         ").public().0.into(),
		ed25519::Pair::from_seed(b"Dave                            ").public().0.into(),
		ed25519::Pair::from_seed(b"Eve                             ").public().0.into(),
		ed25519::Pair::from_seed(b"Ferdie                          ").public().0.into(),
	])
}

fn local_testnet_genesis_instant() -> GenesisConfig {
	let mut genesis = local_testnet_genesis();
	genesis.timestamp = Some(TimestampConfig { period: 0 });
	genesis
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec<GenesisConfig> {
	ChainSpec::from_genesis("Local Testnet", "local_testnet", local_testnet_genesis, vec![], None, None, None)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn integration_test_config() -> ChainSpec<GenesisConfig> {
	ChainSpec::from_genesis("Integration Test", "test", local_testnet_genesis_instant, vec![], None, None, None)
}
