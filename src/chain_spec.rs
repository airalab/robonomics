//! Chain specification and utils.

use primitives::{Ed25519AuthorityId, ed25519};
use robonomics_runtime::{
    GenesisConfig, ConsensusConfig, SessionConfig, StakingConfig, TimestampConfig,
    IndicesConfig, BalancesConfig, FeesConfig, GrandpaConfig, SudoConfig,
    AccountId, Perbill
};
use substrate_service;

/// Specialised `ChainSpec`. This is a specialisation of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// Robonomics testnet generator
pub fn robonomics_testnet_config() -> Result<ChainSpec, String> {
	ChainSpec::from_embedded(include_bytes!("../res/robonomics.json"))
}

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
    /// Whatever the current runtime is, with just Alice as an auth.
    Development,
    /// Robonomics public testnet
    Robonomics,
}

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
            Alternative::Robonomics => robonomics_testnet_config()?,
        })
    }

    pub(crate) fn from(s: &str) -> Option<Self> {
        match s {
            "dev" => Some(Alternative::Development),
            "" | "robonomics" => Some(Alternative::Robonomics),
            _ => None,
        }
    }
}

fn testnet_genesis(initial_authorities: Vec<Ed25519AuthorityId>, endowed_accounts: Vec<AccountId>, sudo_key: AccountId) -> GenesisConfig {
    const SECS_PER_BLOCK: u64 = 10;
    const MINUTES: u64 = 60 / SECS_PER_BLOCK;

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/robonomics_runtime.compact.wasm").to_vec(),
            authorities: initial_authorities.clone(),
        }),
        system: None,
        timestamp: Some(TimestampConfig {
            period: SECS_PER_BLOCK / 2,
        }),
        indices: Some(IndicesConfig {
            ids: endowed_accounts.clone(),
        }),
        balances: Some(BalancesConfig {
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            balances: endowed_accounts.iter().map(|&k|(k, (1 << 60))).collect(),
            vesting: vec![],
        }),
		fees: Some(FeesConfig {
			transaction_base_fee: 1000,
			transaction_byte_fee: 50,
		}),
        session: Some(SessionConfig {
            validators: initial_authorities.iter().cloned().map(Into::into).collect(),
            session_length: 5 * MINUTES,
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            intentions: initial_authorities.iter().cloned().map(Into::into).collect(),
            offline_slash: Perbill::from_billionths(1),
            session_reward: Perbill::from_billionths(5),
            current_offline_slash: 0,
            current_session_reward: 0,
            validator_count: 7,
            sessions_per_era: 12,
            bonding_duration: 60 * MINUTES,
            offline_slash_grace: 4,
            minimum_validator_count: 4,
            invulnerables: initial_authorities.iter().cloned().map(Into::into).collect(),
        }),
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities.clone().into_iter().map(|k| (k, 1)).collect(),
        }),
        sudo: Some(SudoConfig {
            key: sudo_key,
        }),
    }
}
