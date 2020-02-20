///////////////////////////////////////////////////////////////////////////////
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

use log::info;
use sc_cli::{VersionInfo, error};
use sc_service::Roles;
use crate::{
    Cli, Subcommand, IsIpci, load_spec,
    service::{
        RobonomicsExecutor, IpciExecutor, NativeExecutionDispatch,
        new_robonomics_full, new_robonomics_light,
        new_ipci_full, new_ipci_light,
        new_robonomics_chain_ops,
        new_ipci_chain_ops,
    },
};

/// Parse command line arguments into service configuration.
pub fn run(version: VersionInfo) -> error::Result<()> {
    let opt = sc_cli::from_args::<Cli>(&version);

    let mut config = sc_service::Configuration::new(&version);
    config.impl_name = "airalab-robonomics" ;

    match opt.subcommand {
        None => {
            sc_cli::init(&opt.run.shared_params, &version)?;
            sc_cli::init_config(&mut config, &opt.run.shared_params, &version, load_spec)?;
            sc_cli::update_config_for_running_node(&mut config, opt.run)?;

            info!("{}", version.name);
            info!("  version {}", config.full_version());
            info!("  by {}, {}~", version.author, version.copyright_start_year);
            info!("Chain specification: {}", config.expect_chain_spec().name());
            info!("Node name: {}", config.name);
            info!("Roles: {}", sc_cli::display_role(&config));

            let is_ipci = config.chain_spec.as_ref().map_or(false, |s| s.is_ipci());
            if is_ipci {
                info!("Native runtime: {}", IpciExecutor::native_version().runtime_version);
                match config.roles {
                    Roles::LIGHT => sc_cli::run_service_until_exit(config, new_ipci_light),
                    _            => sc_cli::run_service_until_exit(config, new_ipci_full),
                }
            } else {
                info!("Native runtime: {}", RobonomicsExecutor::native_version().runtime_version);
                match config.roles {
                    Roles::LIGHT => sc_cli::run_service_until_exit(config, new_robonomics_light),
                    _            => sc_cli::run_service_until_exit(config, new_robonomics_full),
                }
            }
        },
        Some(Subcommand::Base(cmd)) => {
            sc_cli::init(cmd.get_shared_params(), &version)?;
            sc_cli::init_config(&mut config, &cmd.get_shared_params(), &version, load_spec)?;

            let is_ipci = config.chain_spec.as_ref().map_or(false, |s| s.is_ipci());
            if is_ipci {
                cmd.run(config, new_ipci_chain_ops)
            } else {
                cmd.run(config, new_robonomics_chain_ops)
            }
        }
    }
}
