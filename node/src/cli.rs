///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
//! Console line interface keys & commands.

const AFTER_HELP_ROBONOMICS: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</></>
   <bold>robonomics --chain main --sync warp -- --chain kusama --sync warp</>
           Launch a warp-syncing full node of the <italic>Robonomics</> parachain on the <italic>Kusama</> Relay Chain.
   <bold>robonomics --chain main --sync warp --relay-chain-rpc-url ws://rpc.example.com -- --chain kusama</>
           Launch a warp-syncing full node of the <italic>Robonomics</> parachain on the <italic>Kusama</> Relay Chain.
           Uses <italic>ws://rpc.example.com</> as remote relay chain node.
 "#
);

/// An overarching CLI command definition.
#[derive(Debug, clap::Parser)]
#[command(
    propagate_version = true,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
#[clap(after_help = AFTER_HELP_ROBONOMICS)]
pub struct Cli {
    /// Possible subcommand with parameters.
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub run: cumulus_client_cli::RunCmd,

    /// Disable automatic hardware benchmarks.
    ///
    /// By default these benchmarks are automatically ran at startup and measure
    /// the CPU speed, the memory bandwidth and the disk speed.
    ///
    /// The results are then printed out in the logs, and also sent as part of
    /// telemetry, if telemetry is enabled.
    #[arg(long)]
    pub no_hardware_benchmarks: bool,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub storage_monitor: sc_storage_monitor::StorageMonitorParams,

    /// An address assigned to collator. [default: off]
    /// Notice: If not set then collator rewards will go to treasury.
    #[clap(long)]
    pub lighthouse_account: Option<String>,

    /// Polkadot relaychain arguments.
    #[clap(raw = true)]
    pub relaychain_args: Vec<String>,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Key management cli utilities
    #[clap(subcommand)]
    Key(sc_cli::KeySubcommand),

    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
    Verify(sc_cli::VerifyCmd),

    /// Generate a seed that provides a vanity address.
    Vanity(sc_cli::VanityCmd),

    /// Sign a message, with a given (secret) key.
    Sign(sc_cli::SignCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(cumulus_client_cli::PurgeChainCmd),

    /// Export the genesis state of the parachain.
    #[clap(name = "export-genesis-state")]
    // ExportGenesisState(cumulus_client_cli::ExportGenesisStateCommand),
    ExportGenesisState(cumulus_client_cli::ExportGenesisHeadCommand),

    /// Export the genesis wasm of the parachain.
    #[clap(name = "export-genesis-wasm")]
    ExportGenesisWasm(cumulus_client_cli::ExportGenesisWasmCommand),

    /// Benchmarking runtime pallets.
    #[command(subcommand)]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}

#[derive(Debug)]
pub struct RelayChainCli {
    /// The actual relay chain cli object.
    pub base: polkadot_cli::RunCmd,

    /// Optional chain id that should be passed to the relay chain.
    pub chain_id: Option<String>,

    /// The base path that should be used by the relay chain.
    pub base_path: Option<std::path::PathBuf>,
}

impl RelayChainCli {
    /// Parse the relay chain CLI parameters using the para chain `Configuration`.
    pub fn new<'a>(
        para_config: &sc_service::Configuration,
        relay_chain_args: impl Iterator<Item = &'a String>,
    ) -> Self {
        let extension = crate::chain_spec::Extensions::try_get(&*para_config.chain_spec);
        let chain_id = extension.map(|e| e.relay_chain.clone());
        let base_path = para_config.base_path.path().join("polkadot");
        Self {
            base_path: Some(base_path),
            chain_id,
            base: clap::Parser::parse_from(relay_chain_args),
        }
    }
}
