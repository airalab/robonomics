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

#[cfg(feature = "parachain")]
use crate::parachain;
use crate::{
    chain_spec::*,
    service::{self, ipci, robonomics},
    Cli, Subcommand,
};
use sc_cli::{ChainSpec, Role, RuntimeVersion, SubstrateCli};
use sc_service::PartialComponents;

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "airalab-robonomics".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/airalab/robonomics/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2018
    }

    fn executable_name() -> String {
        "robonomics".into()
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(development_config()),
            "ipci" => Box::new(ipci_config()),
            #[cfg(feature = "parachain")]
            "" | "parachain" => Box::new(parachain::chain_spec::robonomics_parachain_config()),
            path => Box::new(crate::chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        match chain_spec.family() {
            RobonomicsFamily::DaoIpci => &ipci_runtime::VERSION,
            RobonomicsFamily::Development => &robonomics_runtime::VERSION,
            #[cfg(feature = "parachain")]
            RobonomicsFamily::Parachain => &robonomics_parachain_runtime::VERSION,
            RobonomicsFamily::Unknown => panic!("Unknown runtime"),
        }
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
                    runner.run_node_until_exit(|config| match config.role {
                        Role::Light => ipci::new_light(config),
                        _ => ipci::new_full(config),
                    })
                }

                RobonomicsFamily::Development => {
                    runner.run_node_until_exit(|config| match config.role {
                        Role::Light => robonomics::new_light(config),
                        _ => robonomics::new_full(config),
                    })
                }

                #[cfg(feature = "parachain")]
                RobonomicsFamily::Parachain => runner.run_node_until_exit(|config| {
                    if matches!(config.role, Role::Light) {
                        return Err("Light client not supporter!".into());
                    }

                    parachain::command::run(
                        config,
                        cli.parachain_id,
                        &cli.relaychain_args,
                        cli.run.validator,
                    )
                }),

                _ => Err(format!(
                    "unsupported chain spec: {}",
                    runner.config().chain_spec.id()
                ))?,
            }
        }
        Some(Subcommand::Base(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;
            match runner.config().chain_spec.family() {
                RobonomicsFamily::DaoIpci => runner.run_subcommand(subcommand, |config| {
                    let PartialComponents { client, backend, task_manager, import_queue, ..}
                        = service::new_partial::<
                            ipci_runtime::RuntimeApi,
                            ipci::Executor
                        >(&config)?;
                    Ok((client, backend, import_queue, task_manager))
                }),

                RobonomicsFamily::Development => runner.run_subcommand(subcommand, |config| {
                    let PartialComponents { client, backend, task_manager, import_queue, ..}
                        = service::new_partial::<
                            robonomics_runtime::RuntimeApi,
                            robonomics::Executor
                        >(&config)?;
                    Ok((client, backend, import_queue, task_manager))
                }),

                #[cfg(feature = "parachain")]
                RobonomicsFamily::Parachain => runner.run_subcommand(subcommand, |mut config| {
                    let PartialComponents { client, backend, task_manager, import_queue, ..}
                        = parachain::new_partial(&mut config)?;
                    Ok((client, backend, import_queue, task_manager))
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
                    subcommand.run::<node_primitives::Block, ipci::Executor>(config)
                })
            } else {
                runner.sync_run(|config| {
                    subcommand.run::<node_primitives::Block, robonomics::Executor>(config)
                })
            }
        }
    }
}
