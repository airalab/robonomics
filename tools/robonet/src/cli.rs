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
#[command(name = "robonet")]
#[command(about = "Robonomics Network Testbed: local networks and integration testing.", long_about = None)]
#[command(version)]
#[command(before_help = r#"
╔═══════════════════════════════════════════════════════════════════╗
║                                                                   ║
║   ██████╗  ██████╗ ██████╗  ██████╗ ███╗   ██╗███████╗████████╗   ║
║   ██╔══██╗██╔═══██╗██╔══██╗██╔═══██╗████╗  ██║██╔════╝╚══██╔══╝   ║
║   ██████╔╝██║   ██║██████╔╝██║   ██║██╔██╗ ██║█████╗     ██║      ║
║   ██╔══██╗██║   ██║██╔══██╗██║   ██║██║╚██╗██║██╔══╝     ██║      ║
║   ██║  ██║╚██████╔╝██████╔╝╚██████╔╝██║ ╚████║███████╗   ██║      ║
║   ╚═╝  ╚═╝ ╚═════╝ ╚═════╝  ╚═════╝ ╚═╝  ╚═══╝╚══════╝   ╚═╝      ║
║                                                                   ║
║          Local Networks Testbed - Robonomics Network              ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
"#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Verbose output (-v, -vv, -vvv for increasing verbosity)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Output format (text or json)
    #[arg(
        short,
        long,
        value_name = "FORMAT",
        default_value = "text",
        global = true
    )]
    pub output: OutputFormat,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Spawn local network with relay chain and parachain
    Spawn {
        /// Network topology to spawn
        #[arg(long, value_name = "TOPOLOGY", default_value = "simple")]
        topology: NetworkTopology,

        /// Keep network running after spawn (don't wait for Ctrl+C)
        #[arg(long)]
        persist: bool,

        /// Network spawn timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
    },

    /// Run integration tests on the spawned network
    Test {
        /// Network topology to spawn (ignored if --no-spawn is used)
        #[arg(long, value_name = "TOPOLOGY", default_value = "simple")]
        topology: NetworkTopology,

        /// Stop on first test failure
        #[arg(long)]
        fail_fast: bool,

        /// Network spawn timeout in seconds
        #[arg(long, default_value = "60")]
        timeout: u64,

        /// Skip network spawning (assume network is already running)
        #[arg(long)]
        no_spawn: bool,

        /// Specific tests to run (if not set then all tests will run)
        tests: Vec<String>,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum NetworkTopology {
    /// Simple topology: relay chain + robonomics parachain only
    Simple,
    /// AssetHub topology: relay chain + robonomics + asset-hub for XCM tests
    AssetHub,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

impl Default for Commands {
    fn default() -> Self {
        Commands::Spawn {
            topology: NetworkTopology::Simple,
            persist: true,
            timeout: 300,
        }
    }
}
