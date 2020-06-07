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

use structopt::StructOpt;

/// An overarching CLI command definition.
#[derive(Clone, Debug, StructOpt)]
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
    pub run: sc_cli::RunCmd,

    /// Polkadot relaychain arguments.
    #[cfg(feature = "parachain")]
    #[structopt(raw = true)]
    pub relaychain_args: Vec<String>,
}

/// Possible subcommands of the main binary.
#[derive(Clone, Debug, StructOpt)]
pub enum Subcommand {
    /// A set of base subcommands handled by `sc_cli`.
    #[structopt(flatten)]
    Base(sc_cli::Subcommand),
    /// Robonomics Framework I/O operations.
    #[cfg(feature = "robonomics-cli")]
    Io(robonomics_cli::IoCmd),
    /// Benchmarking runtime pallets.
    #[cfg(feature = "benchmarking-cli")]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),
    /// Print hex-encoded genesis state of the parachain.
    #[cfg(feature = "parachain")]
    ExportGenesisState(ExportGenesisState),
}

#[derive(Clone, Debug, StructOpt)]
pub struct ExportGenesisState {
    /// Genesis state path
    pub head_file: Option<std::path::PathBuf>,
}

#[derive(Clone, Debug, StructOpt)]
pub struct ExportGenesisState {
    /// Genesis state path
    pub head_file: Option<std::path::PathBuf>,
}
