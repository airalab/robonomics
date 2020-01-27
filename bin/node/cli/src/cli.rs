//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
pub use sc_cli::VersionInfo;
use sc_cli::{
    IntoExit, NoCustom, error,
    display_role, parse_and_prepare, ParseAndPrepare
};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use sc_service::{AbstractService, Roles as ServiceRoles, Configuration};
use futures::{channel::oneshot, future::{select, Either}};
use sc_executor::NativeExecutionDispatch;
use node_executor::Executor;
use crate::chain_spec::load_spec;
use crate::service;
use log::info;

/// Parse command line arguments into service configuration.
pub fn run<I, T, E>(args: I, exit: E, version: VersionInfo) -> error::Result<()> where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
    E: IntoExit,
{
    type Config<A, B> = Configuration<(), A, B>;

    match parse_and_prepare::<NoCustom, NoCustom, _>(&version, "airalab-robonomics", args) {
        ParseAndPrepare::Run(cmd) => cmd.run(load_spec, exit,
        |exit, _cli_args, _custom_args, mut config: Config<_, _>| {
            info!("{}", version.name);
            info!("  version {}", config.full_version());
            info!("  by {}, 2018-2020", version.author);
            info!("Chain specification: {}", config.chain_spec.name());
            info!("Native runtime: {}", Executor::native_version().runtime_version);
            info!("Node name: {}", config.name);
            info!("Roles: {}", display_role(&config));
            let runtime = RuntimeBuilder::new()
                .thread_name("main-tokio-")
                .threaded_scheduler()
                .enable_all()
                .build()
                .map_err(|e| format!("{:?}", e))?;
            config.tasks_executor = {
                let runtime_handle = runtime.handle().clone();
                Some(Box::new(move |fut| { runtime_handle.spawn(fut); }))
            };
            match config.roles {
                ServiceRoles::LIGHT => run_until_exit(
                    runtime,
                    service::new_light(config)?,
                    exit
                ),
                _ => run_until_exit(
                    runtime,
                    service::new_full(config)?,
                    exit
                ),
            }
        }),
        ParseAndPrepare::BuildSpec(cmd) => cmd.run::<NoCustom, _, _, _>(load_spec),
        ParseAndPrepare::ExportBlocks(cmd) => cmd.run_with_builder(|config: Config<_, _>|
            Ok(new_full_start!(config).0), load_spec, exit),
        ParseAndPrepare::ImportBlocks(cmd) => cmd.run_with_builder(|config: Config<_, _>|
            Ok(new_full_start!(config).0), load_spec, exit),
        ParseAndPrepare::CheckBlock(cmd) => cmd.run_with_builder(|config: Config<_, _>|
            Ok(new_full_start!(config).0), load_spec, exit),
        ParseAndPrepare::PurgeChain(cmd) => cmd.run(load_spec),
        ParseAndPrepare::RevertChain(cmd) => cmd.run_with_builder(|config: Config<_, _>|
            Ok(new_full_start!(config).0), load_spec),
        ParseAndPrepare::CustomCommand(_) => Ok(()),
    }
}

fn run_until_exit<T, E>(
    mut runtime: Runtime,
    service: T,
    e: E,
) -> error::Result<()>
    where
        T: AbstractService,
        E: IntoExit,
{
    let (exit_send, exit) = oneshot::channel();

    let informant = sc_cli::informant::build(&service);
    let handle = runtime.spawn(select(exit, informant));

    // we eagerly drop the service so that the internal exit future is fired,
    // but we need to keep holding a reference to the global telemetry guard
    let _telemetry = service.telemetry();

    let exit = e.into_exit();
    let service_res = runtime.block_on(select(service, exit));

    let _ = exit_send.send(());

    runtime.block_on(handle);

    match service_res {
        Either::Left((res, _)) => res.map_err(error::Error::Service),
        Either::Right((_, _)) => Ok(())
    }
}
