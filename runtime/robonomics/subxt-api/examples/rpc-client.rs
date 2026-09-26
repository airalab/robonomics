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
use robonomics_runtime_subxt_api::{api, AccountId32, RobonomicsConfig};
use subxt::OnlineClient;
use subxt_signer::sr25519::dev;

#[tokio::main]
async fn main() {
    env_logger::init();
    if let Err(err) = run().await {
        eprintln!("{err}");
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client = OnlineClient::<RobonomicsConfig>::from_url("ws://127.0.0.1:9944").await?;

    // Create dev account
    let alice = dev::alice();
    let alice_id: AccountId32 = alice.public_key().into();

    // Create and submit a transaction
    let tx = api::transactions().system().remark(vec![1, 2, 3, 4]);
    let hash = client
        .tx()
        .await?
        .sign_and_submit_default(&tx, &alice)
        .await?;
    println!("Transaction hash: {:?}", hash);

    // Query account information
    let at_block = client.at_current_block().await?;
    let account = at_block
        .storage()
        .fetch(api::storage().system().account(), (alice_id,))
        .await?;
    println!("Account: {:?}", account.decode()?);

    Ok(())
}
