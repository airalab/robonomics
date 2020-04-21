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
pub struct Cli {
    /// Possible subcommand with parameters.
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub run: sc_cli::RunCmd,
}

/// Possible subcommands of the main binary.
#[derive(Clone, Debug, StructOpt)]
pub enum Subcommand {
    /// A set of base subcommands handled by `sc_cli`.
    #[structopt(flatten)]
    Base(sc_cli::Subcommand),
    /// This subcommand runs node in message router mode.
    #[cfg(feature = "robonomics-protocol")]
    #[structopt(
        name = "pubsub",
        about = "Run node in pubsub(gossipsub) router mode."
    )]
    PubSub(robonomics_protocol::cli::PubSubCmd),
    /// This subcommand store hex-encoded data to Datalog pallet.
    #[cfg(feature = "robonomics-protocol")]
    #[structopt(
        name = "datalog",
        about = "Store hex-encoded data on blockchain."
    )]
    Datalog(robonomics_protocol::cli::DatalogCmd),
    #[cfg(feature = "robonomics-sensors")]
    #[structopt(
        name = "sensors",
        about = "Reads data from sensor."
    )]
    Sensor(robonomics_sensors::cli::SensorCmd),
    /// The custom benchmark subcommmand benchmarking runtime pallets.
    #[cfg(feature = "benchmarking-cli")]
    #[structopt(
        name = "benchmark",
        about = "Benchmark runtime pallets."
    )]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}
