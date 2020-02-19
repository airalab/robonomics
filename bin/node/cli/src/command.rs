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
use sc_cli::{VersionInfo, error};
use crate::{
    Cli, Subcommand, IsIpci, load_spec,
    service::{
        new_robonomics_full, new_robonomics_light,
        new_ipci_full, new_ipci_light,
        new_robonomics_chain_ops,
        new_ipci_chain_ops,
    },
};

/// Parse command line arguments into service configuration.
pub fn run(version: VersionInfo) -> error::Result<()> {
    let opt = sc_cli::from_args::<Cli>(&version);

    let mut config = sc_service::Configuration::default();
    let is_ipci = config.chain_spec.as_ref().map_or(false, |s| s.is_ipci());
    config.impl_name = if is_ipci { "airalab-ipci" } else { "airalab-robonomics" };

    match opt.subcommand {
        None => if is_ipci {
            sc_cli::run(
                config, opt.run, new_ipci_full, new_ipci_light, load_spec, &version
            )
        } else {
            sc_cli::run(
                config, opt.run, new_robonomics_full, new_robonomics_light, load_spec, &version
            )
        },
        Some(Subcommand::Base(cmd)) => if is_ipci {
            sc_cli::run_subcommand(config, cmd, load_spec, new_ipci_chain_ops, &version)
        } else {
            sc_cli::run_subcommand(config, cmd, load_spec, new_robonomics_chain_ops, &version)
        }
    }
}
