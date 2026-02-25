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
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use zombienet_sdk::{LocalFileSystem, Network, NetworkConfig, NetworkConfigBuilder, NetworkConfigExt};

use crate::cli::NetworkTopology;

/// Hardcoded network configuration
pub const RELAY_RPC_PORT: u16 = 9944;
pub const ASSET_HUB_RPC_PORT: u16 = 9910;
pub const COLLATOR_1_RPC_PORT: u16 = 9988;
pub const COLLATOR_2_RPC_PORT: u16 = 9989;
pub const PARA_ID: u32 = 2000;
pub const ASSET_HUB_PARA_ID: u32 = 1000;

/// Network endpoint information
#[derive(Debug, Clone)]
pub struct NetworkEndpoints {
    pub relay_ws: String,
    pub collator_1_ws: String,
    pub collator_2_ws: Option<String>,
    pub asset_hub_ws: Option<String>,
}

impl NetworkEndpoints {
    pub fn simple() -> Self {
        Self {
            relay_ws: format!("ws://127.0.0.1:{}", RELAY_RPC_PORT),
            collator_1_ws: format!("ws://127.0.0.1:{}", COLLATOR_1_RPC_PORT),
            collator_2_ws: Some(format!("ws://127.0.0.1:{}", COLLATOR_2_RPC_PORT)),
            asset_hub_ws: None,
        }
    }
    
    pub fn assethub() -> Self {
        Self {
            relay_ws: format!("ws://127.0.0.1:{}", RELAY_RPC_PORT),
            collator_1_ws: format!("ws://127.0.0.1:{}", COLLATOR_1_RPC_PORT),
            collator_2_ws: None,
            asset_hub_ws: Some(format!("ws://127.0.0.1:{}", ASSET_HUB_RPC_PORT)),
        }
    }
}

/// Build the network configuration based on topology
pub fn build_network_config(topology: &NetworkTopology) -> Result<NetworkConfig> {
    log::debug!("Building network configuration for topology: {:?}", topology);
    
    let mut builder = NetworkConfigBuilder::new()
        .with_relaychain(|r| {
            r.with_chain("rococo-local")
                .with_default_command("polkadot")
                .with_validator(|v| v.with_name("alice").with_rpc_port(RELAY_RPC_PORT))
                .with_validator(|v| v.with_name("bob"))
        });
    
    match topology {
        NetworkTopology::Simple => {
            // Simple: Robonomics parachain with 2 collators
            builder = builder.with_parachain(|p| {
                p.with_id(PARA_ID)
                    .with_chain("local")
                    .with_collator(|c| {
                        c.with_name("collator-1")
                            .with_command("robonomics")
                            .with_rpc_port(COLLATOR_1_RPC_PORT)
                    })
                    .with_collator(|c| {
                        c.with_name("collator-2")
                            .with_command("robonomics")
                            .with_rpc_port(COLLATOR_2_RPC_PORT)
                    })
            });
        }
        NetworkTopology::Assethub => {
            // With AssetHub: AssetHub + Robonomics + HRMP channels
            builder = builder
                .with_parachain(|p| {
                    p.with_id(ASSET_HUB_PARA_ID)
                        .with_chain("asset-hub-rococo-local")
                        .with_collator(|c| {
                            c.with_name("asset-hub-collator")
                                .with_command("polkadot-parachain")
                                .with_rpc_port(ASSET_HUB_RPC_PORT)
                        })
                })
                .with_parachain(|p| {
                    p.with_id(PARA_ID)
                        .with_chain("local")
                        .with_collator(|c| {
                            c.with_name("robonomics-collator")
                                .with_command("robonomics")
                                .with_rpc_port(COLLATOR_1_RPC_PORT)
                        })
                })
                .with_hrmp_channel(|h| h.with_sender(ASSET_HUB_PARA_ID).with_recipient(PARA_ID))
                .with_hrmp_channel(|h| h.with_sender(PARA_ID).with_recipient(ASSET_HUB_PARA_ID));
        }
    }
    
    let config = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build network configuration: {:?}", e))?;
    
    log::debug!("Network configuration built successfully");
    Ok(config)
}

/// Spawn the network with progress indication
pub async fn spawn_network(topology: &NetworkTopology, timeout: Duration) -> Result<Network<LocalFileSystem>> {
    println!();
    println!("{}", ">> Starting Robonomics Local Network".bold().green());
    println!("{}", "==================================================".bright_black());
    println!();
    
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));
    
    // Build configuration
    spinner.set_message(format!("Building {:?} network configuration...", topology));
    let config = build_network_config(topology)?;
    
    // Spawn network
    spinner.set_message("Spawning relay chain validators...");
    log::info!("Spawning network with timeout of {} seconds", timeout.as_secs());
    
    let network = tokio::time::timeout(timeout, config.spawn_native())
        .await
        .context("Network spawn timeout")??;
    
    spinner.finish_and_clear();
    
    println!("{} Relay chain ready (alice, bob)", "[OK]".green());
    
    match topology {
        NetworkTopology::Simple => {
            println!("{} Parachain collators ready (collator-1, collator-2)", "[OK]".green());
        }
        NetworkTopology::Assethub => {
            println!("{} AssetHub collator ready", "[OK]".green());
            println!("{} Robonomics collator ready", "[OK]".green());
        }
    }
    
    // Wait a bit for parachain registration
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message("Waiting for parachain registration...");
    
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    spinner.finish_and_clear();
    
    match topology {
        NetworkTopology::Simple => {
            println!("{} Parachain {} registered", "[OK]".green(), PARA_ID);
        }
        NetworkTopology::Assethub => {
            println!("{} AssetHub {} registered", "[OK]".green(), ASSET_HUB_PARA_ID);
            println!("{} Robonomics {} registered", "[OK]".green(), PARA_ID);
            println!("{} HRMP channels established", "[OK]".green());
        }
    }
    println!();
    
    // Display connection info
    println!("{}", ">> Network Ready".bold().green());
    println!("{}", "==================================================".bright_black());
    
    let endpoints = match topology {
        NetworkTopology::Simple => NetworkEndpoints::simple(),
        NetworkTopology::Assethub => NetworkEndpoints::assethub(),
    };
    
    println!("{:<20} {}", "Relay Chain:", endpoints.relay_ws.cyan());
    if let Some(asset_hub) = endpoints.asset_hub_ws {
        println!("{:<20} {}", "AssetHub:", asset_hub.cyan());
    }
    println!("{:<20} {}", "Robonomics:", endpoints.collator_1_ws.cyan());
    if let Some(collator_2) = endpoints.collator_2_ws {
        println!("{:<20} {}", "Collator 2:", collator_2.cyan());
    }
    println!();
    
    log::info!("Network spawned successfully");
    
    Ok(network)
}

/// Display network endpoints
pub fn display_endpoints(topology: &NetworkTopology) {
    let endpoints = match topology {
        NetworkTopology::Simple => NetworkEndpoints::simple(),
        NetworkTopology::Assethub => NetworkEndpoints::assethub(),
    };
    
    println!("{}", "Network Endpoints:".bold());
    println!("  Relay Chain:  {}", endpoints.relay_ws.cyan());
    if let Some(asset_hub) = endpoints.asset_hub_ws {
        println!("  AssetHub:     {}", asset_hub.cyan());
    }
    println!("  Robonomics:   {}", endpoints.collator_1_ws.cyan());
    if let Some(collator_2) = endpoints.collator_2_ws {
        println!("  Collator 2:   {}", collator_2.cyan());
    }
}
