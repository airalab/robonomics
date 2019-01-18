use primitives::{Ed25519AuthorityId as AuthorityId, ed25519};
use robonomics_node_runtime::{
    AccountId, GenesisConfig, ConsensusConfig, TimestampConfig, IndicesConfig, BalancesConfig,
    UpgradeKeyConfig, AuthorityKeyConfig
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

fn testnet_genesis(initial_authorities: Vec<AuthorityId>, endowed_accounts: Vec<AccountId>, upgrade_key: AccountId) -> GenesisConfig {
    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/robonomics_node_runtime.compact.wasm").to_vec(),
            authorities: initial_authorities.clone(),
        }),
        system: None,
        timestamp: Some(TimestampConfig {
            period: 5,                    // 5 second block time.
        }),
        indices: Some(IndicesConfig {
            ids: endowed_accounts.clone(),
        }),
        balances: Some(BalancesConfig {
            transaction_base_fee: 1,
            transaction_byte_fee: 0,
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            balances: endowed_accounts.iter().map(|&k|(k, (1 << 60))).collect(),
        }),
        upgrade_key: Some(UpgradeKeyConfig {
            key: upgrade_key,
        }),
        authority_key: Some(AuthorityKeyConfig {
            key: upgrade_key,
        }),
        robonomics: None
    }
}
