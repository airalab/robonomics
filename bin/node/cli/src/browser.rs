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
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use sc_service::Configuration;
use browser_utils::{
    Transport, Client,
    browser_configuration, set_console_error_panic_hook, init_console_log,
};
use crate::{IsIpci, chain_spec::ChainSpec};

/// Starts the client.
#[wasm_bindgen]
pub async fn start_client(chain_spec: String, log_level: String, wasm_ext: Transport) -> Result<Client, JsValue> {
    start_inner(chain_spec, log_level, wasm_ext)
        .await
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

async fn start_inner(chain_spec: String, log_level: String, wasm_ext: Transport) -> Result<Client, Box<dyn std::error::Error>> {
    set_console_error_panic_hook();
    init_console_log(log::Level::from_str(&log_level)?)?;
    let chain_spec = ChainSpec::from_json_bytes(chain_spec.as_bytes().to_vec())
        .map_err(|e| format!("{:?}", e))?;

    let config: Configuration<_, _> = browser_configuration(wasm_ext, chain_spec)
        .await?;

    info!("Robonomics Web Node");
    info!("  version {}", config.full_version());
    info!("  by Airalab <research@aira.life>, 2018-2020");
    info!("Chain specification: {}", config.expect_chain_spec().name());
    info!("Node name: {}", config.name);
    info!("Roles: {:?}", config.roles);

    let is_ipci = config.chain_spec.as_ref().map_or(false, |s| s.is_ipci());
    // Create the service. This is the most heavy initialization step.
    if is_ipci {
        let service = crate::service::new_ipci_light(config)
            .map_err(|e| format!("{:?}", e))?;
        Ok(browser_utils::start_client(service))
    } else {
        let service = crate::service::new_robonomics_light(config)
            .map_err(|e| format!("{:?}", e))?;
        Ok(browser_utils::start_client(service))
    }
}
