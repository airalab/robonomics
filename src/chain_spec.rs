use primitives::{AuthorityId, ed25519};
use template_node_runtime::{
	AccountId, GenesisConfig, ConsensusConfig, TimestampConfig, BalancesConfig, UpgradeKeyConfig,
	DemocracyConfig, GrandpaConfig, SessionConfig, StakingConfig, TreasuryConfig, Permill, Perbill
};
use substrate_service;

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialised `ChainSpec`. This is a specialisation of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Whatever the current runtime is, with just Alice as an auth.
	Development,
	/// Whatever the current runtime is, with simple Alice/Bob auths.
	LocalTestnet,
}

const MILLICENTS: u128 = 1_000_000_000;
const CENTS: u128 = 1_000 * MILLICENTS;    // assume this is worth about a cent.
const DOLLARS: u128 = 100 * CENTS;

const SECS_PER_BLOCK: u64 = 6;
const MINUTES: u64 = 60 / SECS_PER_BLOCK;
const HOURS: u64 = MINUTES * 60;
const DAYS: u64 = HOURS * 24;

impl Alternative {
	/// Get an actual chain config from one of the alternatives.
	pub(crate) fn load(self) -> Result<ChainSpec, String> {
		Ok(match self {
			Alternative::Development => ChainSpec::from_genesis(
				"Development",
				"dev",
				|| testnet_genesis(vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().into(),
				], vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into(),
				],
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into()
				),
				vec![],
				None,
				None,
				None,
				None
			),
			Alternative::LocalTestnet => ChainSpec::from_genesis(
				"Local Testnet",
				"local_testnet",
				|| testnet_genesis(vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().into(),
					ed25519::Pair::from_seed(b"Bob                             ").public().into(),
				], vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into(),
					ed25519::Pair::from_seed(b"Bob                             ").public().0.into(),
					ed25519::Pair::from_seed(b"Charlie                         ").public().0.into(),
					ed25519::Pair::from_seed(b"Dave                            ").public().0.into(),
					ed25519::Pair::from_seed(b"Eve                             ").public().0.into(),
					ed25519::Pair::from_seed(b"Ferdie                          ").public().0.into(),
				],
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into()
				),
				vec![],
				None,
				None,
				None,
				None
			),
		})
	}

	pub(crate) fn from(s: &str) -> Option<Self> {
		match s {
			"dev" => Some(Alternative::Development),
			"local" => Some(Alternative::LocalTestnet),
			_ => None,
		}
	}
}

fn testnet_genesis(initial_authorities: Vec<AuthorityId>, endowed_accounts: Vec<AccountId>, upgrade_key: AccountId) -> GenesisConfig {
	GenesisConfig {
		consensus: Some(ConsensusConfig {
			code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/template_node_runtime.compact.wasm").to_vec(),
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
		upgrade_key: Some(UpgradeKeyConfig {
			key: upgrade_key,
		}),
		democracy: Some(DemocracyConfig {
			launch_period: 10 * MINUTES,    // 1 day per public referendum
			voting_period: 10 * MINUTES,    // 3 days to discuss & vote on an active referendum
			minimum_deposit: 50 * DOLLARS,    // 12000 as the minimum deposit for a referendum
			public_delay: 10 * MINUTES,
			max_lock_periods: 6,
		}),
		grandpa: Some(GrandpaConfig {
			authorities: initial_authorities.clone().into_iter().map(|k| (k, 1)).collect(),
		}),
		session: Some(SessionConfig {
			validators: initial_authorities.iter().cloned().map(Into::into).collect(),
			session_length: 5 * MINUTES,
		}),
		staking: Some(StakingConfig {
			current_era: 0,
			intentions: initial_authorities.iter().cloned().map(Into::into).collect(),
			offline_slash: Perbill::from_billionths(1_000_000),
			session_reward: Perbill::from_billionths(2_065),
			current_offline_slash: 0,
			current_session_reward: 0,
			validator_count: 7,
			sessions_per_era: 12,
			bonding_duration: 60 * MINUTES,
			offline_slash_grace: 4,
			minimum_validator_count: 4,
		}),
		treasury: Some(TreasuryConfig {
			proposal_bond: Permill::from_percent(5),
			proposal_bond_minimum: 1 * DOLLARS,
			spend_period: 1 * DAYS,
			burn: Permill::from_percent(50),
		}),
	}
}
