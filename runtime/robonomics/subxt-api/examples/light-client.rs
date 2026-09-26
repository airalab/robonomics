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
use robonomics_chain_spec::{POLKADOT_PARACHAIN_RAW, POLKADOT_RELAY_RAW};
use robonomics_runtime_subxt_api::{api, RobonomicsConfig};
use subxt::{lightclient::LightClient, OnlineClient};

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(err) = run().await {
        eprintln!("{err}");
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (light_client, _) = LightClient::relay_chain(POLKADOT_RELAY_RAW)?;
    let para_rpc = light_client.parachain(POLKADOT_PARACHAIN_RAW)?;
    let client: OnlineClient<RobonomicsConfig> = OnlineClient::from_rpc_client(para_rpc).await?;

    // Query CPS information
    let at_block = client.at_current_block().await?;
    let supply = at_block
        .storage()
        .fetch(api::storage().balances().total_issuance(), ())
        .await?;
    println!("Parachain XRT supply: {} Wn", supply.decode()?);

    Ok(())
}
