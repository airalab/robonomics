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

use sc_cli::SubstrateCli;
use crate::{
    Cli, Subcommand, chain_spec,
    service::{
        new_robonomics_full, new_robonomics_light,
        new_ipci_full, new_ipci_light,
        new_robonomics_chain_ops,
        new_ipci_chain_ops,
    },
};

/// Can be called for a `Configuration` to check if it is a configuration for IPCI network.
pub trait IsIpci {
    fn is_ipci(&self) -> bool;
}

impl IsIpci for Box<dyn sc_chain_spec::ChainSpec> {
    fn is_ipci(&self) -> bool {
        self.id().starts_with("ipci")
    }
}

impl SubstrateCli for Cli {
    fn impl_name() -> &'static str {
        "Robonomics Node"
    }

    fn impl_version() -> &'static str {
        env!("ROBONOMICS_IMPL_VERSION")
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
            "dev" => Box::new(chain_spec::development_testnet_config()),
            "local" => Box::new(chain_spec::local_testnet_config()),
            "" | "robonomics_testnet" => Box::new(chain_spec::robonomics_testnet_config()),
            "ipci" => Box::new(chain_spec::ipci_config()),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
    sc_cli::reset_signal_pipe_handler()?;

    let cli = Cli::from_args();
    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&cli.run)?;

            if runner.config().chain_spec.is_ipci() {
                runner.run_node(new_ipci_light, new_ipci_full)
            } else {
                runner.run_node(new_robonomics_light, new_robonomics_full)
            }
        }
        Some(Subcommand::Base(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;

            if runner.config().chain_spec.is_ipci() {
                runner.run_subcommand(subcommand, |config| new_ipci_chain_ops(config))
            } else {
                runner.run_subcommand(subcommand, |config| new_robonomics_chain_ops(config))
            }
        }
        #[cfg(feature = "robonomics-protocol")]
        Some(Subcommand::PubSub(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;
            runner.sync_run(|_|
                subcommand.run().map_err(|e| sc_cli::Error::Other(e.to_string()))) 
        }
        #[cfg(feature = "benchmarking-cli")]
        Some(Subcommand::Benchmark(subcommand)) => {
            use crate::service::{RobonomicsExecutor, IpciExecutor};

            let runner = cli.create_runner(subcommand)?;
            if runner.config().chain_spec.is_ipci() {
                runner.sync_run(|config|
                    subcommand.run::<node_primitives::Block, IpciExecutor>(config))
            } else {
                runner.sync_run(|config|
                    subcommand.run::<node_primitives::Block, RobonomicsExecutor>(config))
            }
        }
    }
}
