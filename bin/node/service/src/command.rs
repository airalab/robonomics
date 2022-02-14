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

use crate::cli::{Cli, Subcommand};
#[cfg(feature = "full")]
use crate::{chain_spec::*, service::robonomics};
use sc_cli::{ChainSpec, RuntimeVersion, SubstrateCli};

#[cfg(feature = "parachain")]
use crate::parachain;

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

    #[cfg(feature = "full")]
    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(development_config()),
            #[cfg(feature = "parachain")]
            path => parachain::load_spec(path, self.run.parachain_id.unwrap_or(2048).into())?,
            #[cfg(not(feature = "parachain"))]
            path => Box::new(crate::chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }

    #[cfg(not(feature = "full"))]
    fn load_spec(&self, _id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Err("Chain spec isn't supported for zero build")?
    }

    #[cfg(feature = "full")]
    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        match chain_spec.family() {
            RobonomicsFamily::Development => &local_runtime::VERSION,
            #[cfg(feature = "parachain")]
            RobonomicsFamily::Alpha => &alpha_runtime::VERSION,
            #[cfg(feature = "kusama")]
            RobonomicsFamily::Main => &main_runtime::VERSION,
            #[cfg(feature = "ipci")]
            RobonomicsFamily::Ipci => &ipci_runtime::VERSION,
        }
    }

    #[cfg(not(feature = "full"))]
    fn native_runtime_version(_chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        unimplemented!()
    }
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        #[cfg(not(feature = "full"))]
        None => Ok(()),
        #[cfg(feature = "full")]
        None => {
            let runner = cli.create_runner(&*cli.run)?;
            // Default interval 1 sec
            let heartbeat_interval = cli.run.heartbeat_interval.unwrap_or_else(|| 1000);

            match runner.config().chain_spec.family() {
                RobonomicsFamily::Development => runner.run_node_until_exit(|config| async move {
                    if matches!(config.role, sc_cli::Role::Light) {
                        return Err("Light client not supported!".into());
                    }

                    robonomics::new(config, heartbeat_interval)
                }),

                #[cfg(feature = "parachain")]
                RobonomicsFamily::Alpha => runner.run_node_until_exit(|config| async move {
                    if matches!(config.role, sc_cli::Role::Light) {
                        return Err("Light client not supported!".into());
                    }

                    if cli.run.validator && cli.run.lighthouse_account.is_none() {
                        return Err(
                            "Option --lighthouse-account should be set for validator".into()
                        );
                    }

                    let params = parachain::command::parse_args(
                        config,
                        &cli.relaychain_args,
                        cli.run.parachain_id,
                        cli.run.lighthouse_account,
                    )?;

                    parachain::alpha::start_node(
                        params.0,
                        params.1,
                        params.2,
                        params.3,
                        heartbeat_interval,
                    )
                    .await
                }),

                #[cfg(feature = "kusama")]
                RobonomicsFamily::Main => runner.run_node_until_exit(|config| async move {
                    if matches!(config.role, sc_cli::Role::Light) {
                        return Err("Light client not supported!".into());
                    }

                    if cli.run.validator && cli.run.lighthouse_account.is_none() {
                        return Err(
                            "Option --lighthouse-account should be set for validator".into()
                        );
                    }

                    let params = parachain::command::parse_args(
                        config,
                        &cli.relaychain_args,
                        cli.run.parachain_id,
                        cli.run.lighthouse_account,
                    )?;

                    parachain::main::start_node(
                        params.0,
                        params.1,
                        params.2,
                        params.3,
                        heartbeat_interval,
                    )
                    .await
                }),

                #[cfg(feature = "ipci")]
                RobonomicsFamily::Ipci => runner.run_node_until_exit(|config| async move {
                    if matches!(config.role, sc_cli::Role::Light) {
                        return Err("Light client not supported!".into());
                    }

                    let params = parachain::command::parse_args(
                        config,
                        &cli.relaychain_args,
                        cli.run.parachain_id,
                        cli.run.lighthouse_account,
                    )?;

                    parachain::ipci::start_node(
                        params.0,
                        params.1,
                        params.2,
                        params.3,
                        heartbeat_interval,
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
        #[cfg(feature = "full")]
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        #[cfg(feature = "full")]
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        #[cfg(feature = "robonomics-cli")]
        Some(Subcommand::Io(cmd)) => {
            let runner = cli.create_runner(&*cli.run)?;
            runner.sync_run(|_| cmd.run().map_err(|e| e.to_string().into()))
        }
        #[cfg(feature = "frame-benchmarking-cli")]
        Some(Subcommand::Benchmark(subcommand)) => {
            let runner = cli.create_runner(subcommand)?;
            match runner.config().chain_spec.family() {
                RobonomicsFamily::Development => runner.sync_run(|config| {
                    subcommand.run::<robonomics_primitives::Block, robonomics::Executor>(config)
                }),
                _ => Err("Unknown chain")?,
            }
        }
        #[cfg(feature = "parachain")]
        Some(Subcommand::ExportGenesisState(params)) => {
            use codec::Encode;
            use sp_api::BlockT;
            use sp_core::hexdisplay::HexDisplay;
            use std::io::Write;

            let mut builder = sc_cli::LoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let spec = parachain::load_spec(
                &params.chain.clone().unwrap_or_default(),
                params.parachain_id.into(),
            )?;
            let state_version = Cli::native_runtime_version(&spec).state_version();

            let block: robonomics_primitives::Block =
                parachain::generate_genesis_block(&spec, state_version)?;
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
            use sp_core::hexdisplay::HexDisplay;
            use std::io::Write;

            let mut builder = sc_cli::LoggerBuilder::new("");
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
