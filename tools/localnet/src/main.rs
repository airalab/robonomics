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
//! Robonomics local network spawner and integration test tool.

use anyhow::Result;
use clap::Parser;
use std::time::Duration;

mod cli;
mod health;
mod logging;
mod network;
mod tests;

use cli::{Cli, Commands, OutputFormat};

/// Exit codes
const EXIT_SUCCESS: i32 = 0;
const EXIT_TESTS_FAILED: i32 = 1;
const EXIT_NETWORK_SPAWN_FAILED: i32 = 2;
const EXIT_TIMEOUT: i32 = 3;

#[tokio::main]
async fn main() {
    let exit_code = run().await.unwrap_or_else(|e| {
        eprintln!("Fatal error: {}", e);
        EXIT_NETWORK_SPAWN_FAILED
    });
    
    std::process::exit(exit_code);
}

async fn run() -> Result<i32> {
    let cli = Cli::parse();
    
    // Initialize logging
    match cli.format {
        OutputFormat::Text => logging::init_logger(cli.verbose),
        OutputFormat::Json => logging::init_json_logger(),
    }
    
    // Execute command
    let exit_code = match cli.command.unwrap_or_default() {
        Commands::Spawn { persist, timeout } => {
            cmd_spawn(persist, timeout).await?
        }
        Commands::Test { fail_fast, tests, timeout, no_spawn } => {
            cmd_test(fail_fast, tests, timeout, no_spawn, &cli.format).await?
        }
    };
    
    Ok(exit_code)
}

/// Spawn command handler
async fn cmd_spawn(persist: bool, timeout: u64) -> Result<i32> {
    let timeout_duration = Duration::from_secs(timeout);
    
    match network::spawn_network(timeout_duration).await {
        Ok(network) => {
            if persist {
                use colored::Colorize;
                println!("{}", "Network will remain running. Press Ctrl+C to stop.".bright_black());
                println!();
                
                tokio::signal::ctrl_c().await?;
                
                println!();
                println!("{}", "Shutting down network...".yellow());
                drop(network);
                println!("{}", "Network stopped.".green());
            } else {
                drop(network);
            }
            Ok(EXIT_SUCCESS)
        }
        Err(e) => {
            log::error!("Failed to spawn network: {}", e);
            Err(e)
        }
    }
}

/// Test command handler
async fn cmd_test(fail_fast: bool, tests: Vec<String>, timeout: u64, no_spawn: bool, format: &OutputFormat) -> Result<i32> {
    let network = if no_spawn {
        log::info!("Skipping network spawn (--no-spawn specified)");
        None
    } else {
        // Spawn the network
        log::info!("Spawning network for testing...");
        let timeout_duration = Duration::from_secs(timeout);
        
        match network::spawn_network(timeout_duration).await {
            Ok(n) => {
                // Wait a bit for network to stabilize
                tokio::time::sleep(Duration::from_secs(5)).await;
                Some(n)
            }
            Err(e) => {
                log::error!("Failed to spawn network: {}", e);
                return Ok(EXIT_NETWORK_SPAWN_FAILED);
            }
        }
    };
    
    // Run tests
    let test_filter = if tests.is_empty() {
        None
    } else {
        Some(tests)
    };
    
    let results = tests::run_integration_tests(fail_fast, test_filter, matches!(format, OutputFormat::Json)).await?;
    
    // Clean up network
    drop(network);
    
    // Return appropriate exit code
    if results.is_success() {
        Ok(EXIT_SUCCESS)
    } else {
        Ok(EXIT_TESTS_FAILED)
    }
}


