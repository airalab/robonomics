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

use crate::{
    chain_spec::*,
    service::{executor, ipci, robonomics},
    Cli, Subcommand,
};
use sc_cli::SubstrateCli;

impl SubstrateCli for Cli {
    fn impl_name() -> &'static str {
        "airalab-robonomics"
    }

    fn impl_version() -> &'static str {
        env!("SUBSTRATE_CLI_IMPL_VERSION")
    }

    fn description() -> &'static str {
        env!("CARGO_PKG_DESCRIPTION")
    }

    fn author() -> &'static str {
        env!("CARGO_PKG_AUTHORS")
    }

    fn support_url() -> &'static str {
        "https://github.com/airalab/robonomics/issues/new"
    }

    fn copyright_start_year() -> i32 {
        2018
    }

    fn executable_name() -> &'static str {
        "robonomics"
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(development_testnet_config()),
            "local" => Box::new(local_testnet_config()),
            "ipci" => Box::new(ipci_config()),
            "" | "robonomics_testnet" => Box::new(robonomics_testnet_config()),
            path => Box::new(ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
        })
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&cli.run)?;
            match runner.config().chain_spec.family() {
                RobonomicsFamily::DaoIpci => {
                    runner.run_node(ipci::new_light, ipci::new_full, ipci_runtime::VERSION)
                }

                RobonomicsFamily::Testnet => runner.run_node(
                    robonomics::new_light,
                    robonomics::new_full,
                    robonomics_runtime::VERSION,
                ),

                _ => Err(format!(
                    "unsupported chain spec: {}",
                    runner.config().chain_spec.id()
                ))?,
            }
        }
        Some(Subcommand::Base(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;
            match runner.config().chain_spec.family() {
                RobonomicsFamily::DaoIpci => runner.run_subcommand(subcommand, |mut config| {
                    config.keystore = sc_service::config::KeystoreConfig::InMemory;
                    Ok(new_full_start!(config, ipci_runtime::RuntimeApi, executor::Ipci).0)
                }),

                RobonomicsFamily::Testnet => runner.run_subcommand(subcommand, |mut config| {
                    config.keystore = sc_service::config::KeystoreConfig::InMemory;
                    Ok(new_full_start!(
                        config,
                        robonomics_runtime::RuntimeApi,
                        executor::Robonomics
                    )
                    .0)
                }),

                _ => Err(format!(
                    "unsupported chain spec: {}",
                    runner.config().chain_spec.id()
                ))?,
            }
        }
        #[cfg(feature = "robonomics-cli")]
        Some(Subcommand::Io(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;
            runner.sync_run(|_| subcommand.run().map_err(|e| e.to_string().into()))
        }
        #[cfg(feature = "benchmarking-cli")]
        Some(Subcommand::Benchmark(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;
            if runner.config().chain_spec.is_ipci() {
                runner.sync_run(|config| {
                    subcommand.run::<node_primitives::Block, executor::Ipci>(config)
                })
            } else {
                runner.sync_run(|config| {
                    subcommand.run::<node_primitives::Block, executor::Robonomics>(config)
                })
            }
        }
    }
}
