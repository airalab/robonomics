///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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

use clap::Parser;
use robonomics_pair;
use sc_cli::{KeySubcommand, SignCmd, VanityCmd, VerifyCmd};

/// An overarching CLI command definition.
#[derive(Debug, Parser)]
#[clap(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
pub struct Cli {
    /// Possible subcommand with parameters.
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[allow(missing_docs)]
    #[clap(flatten)]
    #[cfg(feature = "full")]
    pub run: cumulus_client_cli::RunCmd,

    /// Id of the parachain this collator collates for.
    #[clap(long)]
    #[cfg(feature = "parachain")]
    pub parachain_id: Option<u32>,

    /// An address assigned to collator. [default: off]
    /// Notice: If not set then collator rewards will go to treasury.
    #[clap(long)]
    #[cfg(feature = "parachain")]
    pub lighthouse_account: Option<String>,

    /// Local key.
    #[clap(long)]
    pub local_key_file: Option<String>,

    /// PubSub heartbeat interval.
    #[clap(long)]
    pub heartbeat_interval: Option<u64>,

    /// Nodes for connect.
    #[clap(long)]
    pub robonomics_bootnodes: Vec<String>,

    /// Disable mDNS.
    #[clap(long)]
    pub disable_mdns: bool,

    /// Disable Kademlia.
    #[clap(long)]
    pub disable_kad: bool,

    /// Polkadot relaychain arguments.
    #[clap(raw = true, conflicts_with = "relay-chain-rpc-url")]
    #[cfg(feature = "parachain")]
    pub relaychain_args: Vec<String>,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Key management cli utilities
    #[clap(subcommand)]
    Key(KeySubcommand),

    /// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
    Verify(VerifyCmd),

    /// Generate a seed that provides a vanity address.
    Vanity(VanityCmd),

    /// Sign a message, with a given (secret) key.
    Sign(SignCmd),

    /// Build a chain specification.
    #[cfg(feature = "full")]
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Remove the whole chain.
    #[cfg(feature = "full")]
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Robonomics Framework I/O operations.
    #[cfg(feature = "robonomics-cli")]
    Io(robonomics_cli::IoCmd),

    /// Pair by peerId operatins
    /// robonomics pair listen --key ...
    /// robonomics pair connect --addr ...
    Pair(robonomics_pair::sink::virt::PairCmd),

    /// Benchmarking runtime pallets.
    #[cfg(feature = "frame-benchmarking-cli")]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),

    /// Export the genesis state of the parachain.
    #[clap(name = "export-genesis-state")]
    #[cfg(feature = "parachain")]
    ExportGenesisState(cumulus_client_cli::ExportGenesisStateCommand),

    /// Export the genesis wasm of the parachain.
    #[clap(name = "export-genesis-wasm")]
    #[cfg(feature = "parachain")]
    ExportGenesisWasm(cumulus_client_cli::ExportGenesisWasmCommand),
}
