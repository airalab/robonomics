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
    service::{ipci, robonomics},
    Cli, Subcommand,
};
use codec::Encode;
use sc_cli::{ChainSpec, Role, RuntimeVersion, SubstrateCli};
use sp_api::BlockT;
use sp_core::hexdisplay::HexDisplay;
use std::io::Write;

#[cfg(feature = "parachain")]
use crate::parachain;
#[cfg(feature = "parachain")]
use cumulus_primitives::genesis::generate_genesis_block;

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
            path => parachain::load_spec(path, self.run.parachain_id.unwrap_or(3000).into())?,
            #[cfg(not(feature = "parachain"))]
            path => Err("Unknown spec")?,
        })
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        match chain_spec.family() {
            RobonomicsFamily::DaoIpci => &ipci_runtime::VERSION,
            RobonomicsFamily::Development => &robonomics_runtime::VERSION,
            #[cfg(feature = "parachain")]
            RobonomicsFamily::Parachain => &robonomics_parachain_runtime::VERSION,
            #[cfg(not(feature = "parachain"))]
            RobonomicsFamily::Parachain => &robonomics_runtime::VERSION,
        }
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&*cli.run)?;
            match runner.config().chain_spec.family() {
                RobonomicsFamily::DaoIpci => runner.run_node_until_exit(|config| async move {
                    match config.role {
                        Role::Light => ipci::new_light(config).map(|r| r.0),
                        _ => ipci::new_full(config),
                    }
                }),

                RobonomicsFamily::Development => runner.run_node_until_exit(|config| async move {
                    match config.role {
                        Role::Light => robonomics::new_light(config).map(|r| r.0),
                        _ => robonomics::new_full(config),
                    }
                }),

                RobonomicsFamily::Parachain => runner.run_node_until_exit(|config| async move {
                    if matches!(config.role, Role::Light) {
                        return Err("Light client not supporter!".into());
                    }

                    #[cfg(not(feature = "parachain"))]
                    {
                        return Err("Parachain feature isn't enabled".into());
                    }

                    #[cfg(feature = "parachain")]
                    parachain::command::run(
                        config,
                        &cli.relaychain_args,
                        cli.run.parachain_id,
                        cli.run.validator,
                    )
                    .await
                }),
            }
            .map_err(Into::into)
        }
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::Sign(cmd)) => cmd.run(),
        Some(Subcommand::Verify(cmd)) => cmd.run(),
        Some(Subcommand::Vanity(cmd)) => cmd.run(),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        #[cfg(feature = "robonomics-cli")]
        Some(Subcommand::Io(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;
            runner.sync_run(|_| subcommand.run().map_err(|e| e.to_string().into()))
        }
        #[cfg(feature = "frame-benchmarking-cli")]
        Some(Subcommand::Benchmark(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;
            match runner.config().chain_spec.family() {
                RobonomicsFamily::DaoIpci => runner.sync_run(|config| {
                    subcommand.run::<node_primitives::Block, ipci::Executor>(config)
                }),
                RobonomicsFamily::Development => runner.sync_run(|config| {
                    subcommand.run::<node_primitives::Block, robonomics::Executor>(config)
                }),
                _ => Err("Unknown chain")?,
            }
        }
        #[cfg(feature = "parachain")]
        Some(Subcommand::ExportGenesisState(params)) => {
            let mut builder = sc_cli::GlobalLoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let block: node_primitives::Block = generate_genesis_block(&parachain::load_spec(
                &params.chain.clone().unwrap_or_default(),
                params.parachain_id.into(),
            )?)?;
            let raw_header = block.header().encode();
            let output_buf = if params.raw {
                raw_header
            } else {
                format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
            };

            if let Some(output) = &params.output {
                std::fs::write(output, output_buf)?;
            } else {
                std::io::stdout().write_all(&output_buf)?;
            }

            Ok(())
        }
        #[cfg(feature = "parachain")]
        Some(Subcommand::ExportGenesisWasm(params)) => {
            let mut builder = sc_cli::GlobalLoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let raw_wasm_blob = parachain::extract_genesis_wasm(
                &cli.load_spec(&params.chain.clone().unwrap_or_default())?,
            )?;

            let output_buf = if params.raw {
                raw_wasm_blob
            } else {
                format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
            };

            if let Some(output) = &params.output {
                std::fs::write(output, output_buf)?;
            } else {
                std::io::stdout().write_all(&output_buf)?;
            }

            Ok(())
        }
    }
}
