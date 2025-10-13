///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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
#![warn(unused_extern_crates)]

use polkadot_omni_node_lib::{
    chain_spec::{ChainSpec, Extensions, GenericChainSpec, LoadSpec},
    run,
    runtime::DefaultRuntimeResolver,
    CliConfig as CliConfigT, RunConfig, NODE_VERSION,
};

struct CliConfig;

impl CliConfigT for CliConfig {
    fn impl_version() -> String {
        let commit_hash = env!("SUBSTRATE_CLI_COMMIT_HASH");
        format!(
            "robonomics-{commit_hash} :: polkadot omni node v{}",
            NODE_VERSION
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/airalab/robonomics/issues/new".into()
    }

    fn copyright_start_year() -> u16 {
        2018
    }
}

fn robonomics_development_config() -> Result<GenericChainSpec, String> {
    let config = GenericChainSpec::builder(
        robonomics_runtime::WASM_BINARY.ok_or("wasm not available")?,
        Extensions {
            relay_chain: "westend-local".into(),
        },
    )
    .with_name("Robonomics Local Develoment")
    .with_id("robonomics-local-development")
    .with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
    .build();
    Ok(config)
}

/// OMNI chain spec loader with buildin robonomics chains.
struct RobonomicsChainSpecLoader;

impl LoadSpec for RobonomicsChainSpecLoader {
    fn load_spec(&self, path: &str) -> Result<Box<dyn ChainSpec>, String> {
        Ok(Box::new(match path {
            "" | "polkadot" => GenericChainSpec::from_json_bytes(
                &include_bytes!("../chains/polkadot-parachain.raw.json")[..],
            )?,
            "kusama" => GenericChainSpec::from_json_bytes(
                &include_bytes!("../chains/kusama-parachain.raw.json")[..],
            )?,
            "dev" => robonomics_development_config()?,
            path => GenericChainSpec::from_json_file(path.into())?,
        }))
    }
}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let config = RunConfig::new(
        Box::new(DefaultRuntimeResolver),
        Box::new(RobonomicsChainSpecLoader),
    );
    Ok(run::<CliConfig>(config)?)
}
