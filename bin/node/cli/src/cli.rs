///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life>
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

use sc_cli::{KeySubcommand, SignCmd, VanityCmd, VerifyCmd};
use structopt::StructOpt;

/// An overarching CLI command definition.
#[derive(Debug, StructOpt)]
#[structopt(settings = &[
    structopt::clap::AppSettings::GlobalVersion,
    structopt::clap::AppSettings::ArgsNegateSubcommands,
    structopt::clap::AppSettings::SubcommandsNegateReqs,
])]
pub struct Cli {
    /// Possible subcommand with parameters.
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub run: RunCmd,

    /// Polkadot relaychain arguments.
    #[structopt(raw = true)]
    pub relaychain_args: Vec<String>,
}

#[derive(Debug, StructOpt)]
pub struct RunCmd {
    #[structopt(flatten)]
    pub base: sc_cli::RunCmd,

    /// Id of the parachain this collator collates for.
    #[structopt(long)]
    pub parachain_id: Option<u32>,
}

impl std::ops::Deref for RunCmd {
    type Target = sc_cli::RunCmd;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

/// Possible subcommands of the main binary.
#[derive(Debug, StructOpt)]
pub enum Subcommand {
    /// Key management cli utilities
    Key(KeySubcommand),

    /// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
    Verify(VerifyCmd),

    /// Generate a seed that provides a vanity address.
    Vanity(VanityCmd),

    /// Sign a message, with a given (secret) key.
    Sign(SignCmd),

    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Robonomics Framework I/O operations.
    #[cfg(feature = "robonomics-cli")]
    Io(robonomics_cli::IoCmd),

    /// Benchmarking runtime pallets.
    #[cfg(feature = "frame-benchmarking-cli")]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),

    /// Export the genesis state of the parachain.
    #[structopt(name = "export-genesis-state")]
    #[cfg(feature = "parachain")]
    ExportGenesisState(super::parachain::cli::ExportGenesisStateCommand),

    /// Export the genesis wasm of the parachain.
    #[structopt(name = "export-genesis-wasm")]
    #[cfg(feature = "parachain")]
    ExportGenesisWasm(super::parachain::cli::ExportGenesisWasmCommand),
}
