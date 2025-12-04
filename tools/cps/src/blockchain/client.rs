use anyhow::{anyhow, Result};
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::{sr25519::Keypair, SecretUri};

#[derive(Clone)]
pub struct Config {
    pub ws_url: String,
    pub suri: Option<String>,
}

pub struct Client {
    pub api: OnlineClient<PolkadotConfig>,
    pub keypair: Option<Keypair>,
}

impl Client {
    pub async fn new(config: &Config) -> Result<Self> {
        // Connect to the blockchain
        let api = OnlineClient::<PolkadotConfig>::from_url(&config.ws_url)
            .await
            .map_err(|e| anyhow!("Failed to connect to {}: {}", config.ws_url, e))?;

        // Parse keypair if SURI provided
        let keypair = if let Some(suri) = &config.suri {
            let uri: SecretUri = suri
                .parse()
                .map_err(|e| anyhow!("Failed to parse SURI: {e}"))?;
            Some(Keypair::from_uri(&uri).map_err(|e| anyhow!("Failed to create keypair: {e}"))?)
        } else {
            None
        };

        Ok(Self { api, keypair })
    }

    pub fn require_keypair(&self) -> Result<&Keypair> {
        self.keypair
            .as_ref()
            .ok_or_else(|| anyhow!("This operation requires an account. Please provide --suri or set ROBONOMICS_SURI environment variable."))
    }
}
