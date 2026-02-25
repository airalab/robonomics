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
//! CLI argument parsing and command definitions.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "localnet")]
#[command(about = "Robonomics local network spawner and integration test tool", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Verbose output (-v, -vv, -vvv for increasing verbosity)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Output format (text or json)
    #[arg(short, long, value_name = "FORMAT", default_value = "text", global = true)]
    pub format: OutputFormat,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Spawn local network with relay chain and parachain
    Spawn {
        /// Keep network running after spawn (don't wait for Ctrl+C)
        #[arg(long)]
        persist: bool,

        /// Network spawn timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
    },

    /// Run integration tests on the spawned network
    Test {
        /// Stop on first test failure
        #[arg(long)]
        fail_fast: bool,

        /// Test filter pattern (run only matching tests)
        #[arg(short, long)]
        filter: Option<String>,

        /// Network spawn timeout in seconds
        #[arg(long, default_value = "60")]
        timeout: u64,
    },

    /// Check network health and connectivity
    Health {
        /// Show detailed health information
        #[arg(long)]
        detailed: bool,
    },

    /// Clean up network resources and processes
    Clean {
        /// Force cleanup without confirmation
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Spawn {
            persist: true,
            timeout: 300,
        }
    }
}
