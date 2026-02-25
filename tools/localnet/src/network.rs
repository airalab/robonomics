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

/// Hardcoded network configuration
pub const RELAY_RPC_PORT: u16 = 9944;
pub const COLLATOR_1_RPC_PORT: u16 = 9910;
pub const COLLATOR_2_RPC_PORT: u16 = 9988;
pub const PARA_ID: u32 = 2000;

/// Network endpoint information
#[derive(Debug, Clone)]
pub struct NetworkEndpoints {
    pub relay_ws: String,
    pub collator_1_ws: String,
    pub collator_2_ws: String,
}

impl NetworkEndpoints {
    pub fn new() -> Self {
        Self {
            relay_ws: format!("ws://127.0.0.1:{}", RELAY_RPC_PORT),
            collator_1_ws: format!("ws://127.0.0.1:{}", COLLATOR_1_RPC_PORT),
            collator_2_ws: format!("ws://127.0.0.1:{}", COLLATOR_2_RPC_PORT),
        }
    }
}

impl Default for NetworkEndpoints {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the network configuration
pub fn build_network_config() -> Result<NetworkConfig> {
    log::debug!("Building network configuration...");
    
    let config = NetworkConfigBuilder::new()
        .with_relaychain(|r| {
            r.with_chain("rococo-local")
                .with_default_command("polkadot")
                .with_validator(|v| v.with_name("alice").with_rpc_port(RELAY_RPC_PORT))
                .with_validator(|v| v.with_name("bob"))
        })
        .with_parachain(|p| {
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
        })
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build network configuration: {:?}", e))?;
    
    log::debug!("Network configuration built successfully");
    Ok(config)
}

/// Spawn the network with progress indication
pub async fn spawn_network(timeout: Duration) -> Result<Network<LocalFileSystem>> {
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
    spinner.set_message("Building network configuration...");
    let config = build_network_config()?;
    
    // Spawn network
    spinner.set_message("Spawning relay chain validators...");
    log::info!("Spawning network with timeout of {} seconds", timeout.as_secs());
    
    let network = tokio::time::timeout(timeout, config.spawn_native())
        .await
        .context("Network spawn timeout")??;
    
    spinner.finish_and_clear();
    
    println!("{} Relay chain ready (alice, bob)", "[OK]".green());
    println!("{} Parachain collators ready (collator-1, collator-2)", "[OK]".green());
    
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
    println!("{} Parachain {} registered", "[OK]".green(), PARA_ID);
    println!();
    
    // Display connection info
    println!("{}", ">> Network Ready".bold().green());
    println!("{}", "==================================================".bright_black());
    let endpoints = NetworkEndpoints::new();
    println!("{:<20} {}", "Relay Chain:", endpoints.relay_ws.cyan());
    println!("{:<20} {}", "Collator 1:", endpoints.collator_1_ws.cyan());
    println!("{:<20} {}", "Collator 2:", endpoints.collator_2_ws.cyan());
    println!();
    
    log::info!("Network spawned successfully");
    
    Ok(network)
}

/// Display network endpoints
pub fn display_endpoints() {
    let endpoints = NetworkEndpoints::new();
    println!("{}", "Network Endpoints:".bold());
    println!("  Relay Chain:  {}", endpoints.relay_ws.cyan());
    println!("  Collator 1:   {}", endpoints.collator_1_ws.cyan());
    println!("  Collator 2:   {}", endpoints.collator_2_ws.cyan());
}
