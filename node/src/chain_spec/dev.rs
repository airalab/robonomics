///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
//! DevNet chain specification.

use super::ChainSpec;
use sc_chain_spec::ChainType;

pub fn config() -> ChainSpec {
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "XRT".into());
    properties.insert("tokenDecimals".into(), 9.into());

    ChainSpec::builder(dev_runtime::wasm_binary_unwrap(), Default::default())
        .with_name("Development")
        .with_id("dev")
        .with_chain_type(ChainType::Development)
        .with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
        .with_properties(properties)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec() {
        let mut properties = sc_chain_spec::Properties::new();
        properties.insert("tokenSymbol".into(), "XRT".into());
        properties.insert("tokenDecimals".into(), 9.into());

        let spec = ChainSpec::builder(wasm_binary_unwrap(), Default::default())
            .with_name("Development")
            .with_id("dev")
            .with_chain_type(ChainType::Development)
            .with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
            .with_properties(properties)
            .build();

        let raw_chain_spec = spec.as_json(true);
        assert!(raw_chain_spec.is_ok());
    }
}
