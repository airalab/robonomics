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
//! Network configuration and spawning logic.

use anyhow::{Context, Result};
use serde_json::json;
use std::time::Duration;
use zombienet_sdk::{
    LocalFileSystem, Network, NetworkConfig, NetworkConfigBuilder, NetworkConfigExt,
};

use crate::cli::NetworkTopology;

/// Hardcoded network configuration
pub const RELAY_RPC_PORT: u16 = 9944;
pub const ASSET_HUB_RPC_PORT: u16 = 9910;
pub const COLLATOR_RPC_PORT: u16 = 9988;
pub const PARA_ID: u32 = 2000;
pub const ASSET_HUB_PARA_ID: u32 = 1000;

/// Hardcoded Genesis Parameters
pub const INITIAL_BALANCE: u128 = 1_000_000_000_000_000_000_000;
// Converted from ParaId=2000 as child on https://www.shawntabrizi.com/substrate-js-utilities/
pub const PARA_ACCOUNT: &str = "5Ec4AhPUwPeyTFyuhGuBbD224mY85LKLMSqSSo33JYWCazU4";
// Converted from ParaId=2000 as sibling on https://www.shawntabrizi.com/substrate-js-utilities/
pub const PARA_SIB_ACCOUNT: &str = "5Eg2fntJ27qsari4FGrGhrMqKFDRnkNSR6UshkZYBGXmSuC8";

/// Network endpoint information
#[derive(Debug, Clone)]
pub struct NetworkEndpoints {
    pub relay_ws: String,
    pub collator_ws: String,
    pub assethub_ws: Option<String>,
}

impl From<&NetworkTopology> for NetworkEndpoints {
    fn from(value: &NetworkTopology) -> Self {
        match value {
            NetworkTopology::Simple => NetworkEndpoints::simple(),
            NetworkTopology::AssetHub => NetworkEndpoints::assethub(),
        }
    }
}

impl NetworkEndpoints {
    pub fn simple() -> Self {
        Self {
            relay_ws: format!("ws://127.0.0.1:{}", RELAY_RPC_PORT),
            collator_ws: format!("ws://127.0.0.1:{}", COLLATOR_RPC_PORT),
            assethub_ws: None,
        }
    }

    pub fn assethub() -> Self {
        Self {
            relay_ws: format!("ws://127.0.0.1:{}", RELAY_RPC_PORT),
            collator_ws: format!("ws://127.0.0.1:{}", COLLATOR_RPC_PORT),
            assethub_ws: Some(format!("ws://127.0.0.1:{}", ASSET_HUB_RPC_PORT)),
        }
    }
}

/// Build the network configuration based on topology
pub fn build_network_config(topology: &NetworkTopology) -> Result<NetworkConfig> {
    let assethub_balances = json!([
        [PARA_SIB_ACCOUNT, 1000000000000000000u128],
        [
            "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
            1000000000000000000u128
        ],
        [
            "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy",
            1000000000000000000u128
        ],
        [
            "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
            1000000000000000000u128
        ],
        [
            "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw",
            1000000000000000000u128
        ],
        [
            "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL",
            1000000000000000000u128
        ],
        [
            "5Ck5SLSHYac6WFt5UZRSsdJjwmpSZq85fd5TRNAdZQVzEAPT",
            1000000000000000000u128
        ],
        [
            "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
            1000000000000000000u128
        ],
        [
            "5HKPmK9GYtE1PSLsS1qiYU9xQ9Si1NcEhdeCq9sw5bqu4ns8",
            1000000000000000000u128
        ],
        [
            "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
            1000000000000000000u128
        ],
        [
            "5FCfAonRZgTFrTd9HREEyeJjDpT397KMzizE6T3DvebLFE7n",
            1000000000000000000u128
        ],
        [
            "5CRmqmsiNFExV6VbdmPJViVxrWmkaXXvBrSX8oqBT8R9vmWk",
            1000000000000000000u128
        ],
        [
            "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc",
            1000000000000000000u128
        ]
    ]);

    let relay_balances = json!([
        [PARA_ACCOUNT, 1000000000000000000u128],
        [
            "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
            1000000000000000000u128
        ],
        [
            "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy",
            1000000000000000000u128
        ],
        [
            "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
            1000000000000000000u128
        ],
        [
            "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw",
            1000000000000000000u128
        ],
        [
            "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL",
            1000000000000000000u128
        ],
        [
            "5Ck5SLSHYac6WFt5UZRSsdJjwmpSZq85fd5TRNAdZQVzEAPT",
            1000000000000000000u128
        ],
        [
            "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
            1000000000000000000u128
        ],
        [
            "5HKPmK9GYtE1PSLsS1qiYU9xQ9Si1NcEhdeCq9sw5bqu4ns8",
            1000000000000000000u128
        ],
        [
            "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
            1000000000000000000u128
        ],
        [
            "5FCfAonRZgTFrTd9HREEyeJjDpT397KMzizE6T3DvebLFE7n",
            1000000000000000000u128
        ],
        [
            "5CRmqmsiNFExV6VbdmPJViVxrWmkaXXvBrSX8oqBT8R9vmWk",
            1000000000000000000u128
        ],
        [
            "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc",
            1000000000000000000u128
        ]
    ]);

    log::debug!(
        "Building network configuration for topology: {:?}",
        topology
    );

    let mut builder = NetworkConfigBuilder::new().with_relaychain(|r| {
        r.with_chain("rococo-local")
            .with_genesis_overrides(json!({
                "patch": { "balances": { "balances": relay_balances } }
            }))
            .with_default_command("polkadot")
            .with_default_args(vec!["-lxcm=trace".into()])
            .with_validator(|v| v.with_name("alice").with_rpc_port(RELAY_RPC_PORT))
            .with_validator(|v| v.with_name("bob"))
    });

    match topology {
        NetworkTopology::Simple => {
            // Simple: Robonomics parachain with 1 collator
            builder = builder.with_parachain(|p| {
                p.with_id(PARA_ID)
                    .with_initial_balance(1_000_000_000_000_000_000_000u128)
                    .with_chain("local")
                    .with_collator(|c| {
                        c.with_name("robonomics-collator")
                            .with_command("robonomics")
                            .with_rpc_port(COLLATOR_RPC_PORT)
                    })
            });
        }
        NetworkTopology::AssetHub => {
            // With AssetHub: AssetHub + Robonomics + HRMP channels
            builder = builder
                .with_parachain(|p| {
                    p.with_id(ASSET_HUB_PARA_ID)
                        .with_initial_balance(1_000_000_000_000_000_000_000u128)
                        .with_chain("asset-hub-rococo-local")
                        .with_default_args(vec!["-lxcm=trace".into()])
                        .with_genesis_overrides(json!({
                            "patch": { "balances": { "balances": assethub_balances } }
                        }))
                        .with_collator(|c| {
                            c.with_name("asset-hub-collator")
                                .with_command("polkadot-parachain")
                                .with_rpc_port(ASSET_HUB_RPC_PORT)
                        })
                })
                .with_parachain(|p| {
                    p.with_id(PARA_ID)
                        .with_chain("local")
                        .with_default_args(vec!["-lxcm=trace".into()])
                        .with_collator(|c| {
                            c.with_name("robonomics-collator")
                                .with_command("robonomics")
                                .with_rpc_port(COLLATOR_RPC_PORT)
                        })
                })
        }
    }

    let config = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build network configuration: {:?}", e))?;

    log::debug!("Network configuration built successfully");
    Ok(config)
}

/// Spawn the network with progress indication
pub async fn spawn_network(
    topology: &NetworkTopology,
    timeout: Duration,
) -> Result<Network<LocalFileSystem>> {
    log::info!(">> Starting Robonomics Local Network");

    // Build configuration
    let config = build_network_config(topology)?;

    // Spawn network
    log::info!(
        "Spawning network with timeout of {} seconds",
        timeout.as_secs()
    );
    let network = tokio::time::timeout(timeout, config.spawn_native())
        .await
        .context("Network spawn timeout")??;
    log::info!(">> Network Ready");

    let endpoints = NetworkEndpoints::from(topology);
    log::info!("Relay Chain: {}", endpoints.relay_ws);
    if let Some(assethub) = endpoints.assethub_ws {
        log::info!("AssetHub: {}", assethub);
    }
    log::info!("Robonomics: {}", endpoints.collator_ws);

    Ok(network)
}
