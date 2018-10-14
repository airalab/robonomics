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
use node_runtime::{GenesisConfig, ConsensusConfig, CouncilSeatsConfig, CouncilVotingConfig, DemocracyConfig,
	SessionConfig, StakingConfig, TimestampConfig, BalancesConfig, TreasuryConfig,
	ContractConfig, Permill, Perbill};
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
		balances: Some(BalancesConfig {
			transaction_base_fee: 1,
			transaction_byte_fee: 0,
			existential_deposit: 500,
			transfer_fee: 0,
			creation_fee: 0,
			reclaim_rebate: 0,
			balances: endowed_accounts.iter().map(|&k|(k, (1 << 60))).collect(),
		}),
		session: Some(SessionConfig {
			validators: initial_authorities.iter().cloned().map(Into::into).collect(),
			session_length: 10,
		}),
		staking: Some(StakingConfig {
			current_era: 0,
			intentions: initial_authorities.iter().cloned().map(Into::into).collect(),
			minimum_validator_count: 1,
			validator_count: 2,
			sessions_per_era: 5,
			bonding_duration: 2 * 60 * 12,
			offline_slash: Perbill::zero(),
			session_reward: Perbill::zero(),
			current_offline_slash: 0,
			current_session_reward: 0,
			offline_slash_grace: 0,
		}),
		democracy: Some(DemocracyConfig {
			launch_period: 9,
			voting_period: 18,
			minimum_deposit: 10,
		}),
		council_seats: Some(CouncilSeatsConfig {
			active_council: endowed_accounts.iter()
				.filter(|a| initial_authorities.iter().find(|&b| a.0 == b.0).is_none())
				.map(|a| (a.clone(), 1000000)).collect(),
			candidacy_bond: 10,
			voter_bond: 2,
			present_slash_per_voter: 1,
			carry_count: 4,
			presentation_duration: 10,
			approval_voting_period: 20,
			term_duration: 1000000,
			desired_seats: (endowed_accounts.len() - initial_authorities.len()) as u32,
			inactive_grace_period: 1,
		}),
		council_voting: Some(CouncilVotingConfig {
			cooloff_period: 75,
			voting_period: 20,
		}),
		timestamp: Some(TimestampConfig {
			period: 5,					// 5 second block time.
		}),
		treasury: Some(TreasuryConfig {
			proposal_bond: Permill::from_percent(5),
			proposal_bond_minimum: 1_000_000,
			spend_period: 12 * 60 * 24,
			burn: Permill::from_percent(50),
		}),
		contract: Some(ContractConfig {
			contract_fee: 21,
			call_base_fee: 135,
			create_base_fee: 175,
			gas_price: 1,
			max_depth: 1024,
			block_gas_limit: 10_000_000,
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
	ChainSpec::from_genesis("Development", "development", development_config_genesis, vec![], None, None)
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
	ChainSpec::from_genesis("Local Testnet", "local_testnet", local_testnet_genesis, vec![], None, None)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn integration_test_config() -> ChainSpec<GenesisConfig> {
	ChainSpec::from_genesis("Integration Test", "test", local_testnet_genesis_instant, vec![], None, None)
}
