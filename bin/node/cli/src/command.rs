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
use crate::{Cli, Subcommand, service, load_spec};

/// Parse command line arguments into service configuration.
pub fn run<I, T>(args: I, version: VersionInfo) -> error::Result<()>
where
    I: Iterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args: Vec<_> = args.collect();
    let opt = sc_cli::from_iter::<Cli, _>(args.clone(), &version);

    let mut config = sc_service::Configuration::default();
    config.impl_name = "airalab-robonomics";

    match opt.subcommand {
        None => sc_cli::run(
            config,
            opt.run,
            service::new_light,
            service::new_full,
            load_spec,
            &version,
        ),
        Some(Subcommand::Base(subcommand)) => sc_cli::run_subcommand(
            config,
            subcommand,
            load_spec,
            |config: service::NodeConfiguration| Ok(new_full_start!(config).0),
            &version,
        ),
    }
}
