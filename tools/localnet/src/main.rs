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
//! ZombieNet SDK based Robonomics local network.

use anyhow::Result;
use zombienet_sdk::{NetworkConfigBuilder, NetworkConfigExt};

const RELAY_RPC_PORT: u16 = 9944;
const ASSET_HUB_RPC_PORT: u16 = 9911;
const ROBONOMICS_RPC_PORT: u16 = 9988;

#[tokio::main]
async fn main() -> Result<()> {
    let network = NetworkConfigBuilder::new()
        .with_relaychain(|r| {
            r.with_chain("rococo-local")
                .with_default_command("polkadot")
                .with_validator(|v| v.with_name("alice").with_rpc_port(RELAY_RPC_PORT))
                .with_validator(|v| v.with_name("bob"))
        })
        .with_parachain(|p| {
            p.with_id(1000)
                .with_chain("asset-hub-rococo-local")
                .with_collator(|c| {
                    c.with_name("collator-1000")
                        .with_command("polkadot-parachain")
                        .with_rpc_port(ASSET_HUB_RPC_PORT)
                })
        })
        .with_parachain(|p| {
            p.with_id(2000).with_chain("local").with_collator(|c| {
                c.with_name("collator-2000")
                    .with_command("robonomics")
                    .with_rpc_port(ROBONOMICS_RPC_PORT)
            })
        })
        .with_hrmp_channel(|h| h.with_sender(1000).with_recipient(2000))
        .with_hrmp_channel(|h| h.with_sender(2000).with_recipient(1000))
        .build()
        .unwrap()
        .spawn_native()
        .await?;

    println!("Alice WS: {}", network.get_node("alice")?.ws_uri());
    println!(
        "ParaId 1000 WS: {}",
        network.get_node("collator-1000")?.ws_uri()
    );
    println!(
        "ParaId 2000 WS: {}",
        network.get_node("collator-2000")?.ws_uri()
    );
    tokio::signal::ctrl_c().await?;
    Ok(())
}
