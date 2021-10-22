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
//! Ethereum compatible Robonomics Network types.

use jsonrpc_core::types::error::Error;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use sc_service::SpawnTaskHandle;
use std::str::FromStr;

#[derive(Clone)]
pub struct EthApi {
    task_handle: SpawnTaskHandle,
}

impl EthApi {
    pub fn new(task_handle: SpawnTaskHandle) -> Self {
        EthApi { task_handle }
    }
}

#[rpc(server)]
pub trait EthApiT {
    type Metadata;

    /// Test method
    #[rpc(name = "eth_balance")]
    fn eth_balance(&self, eth_account: String, eth_node: String) -> Result<String>;
}

impl EthApiT for EthApi {
    type Metadata = sc_rpc_api::Metadata;

    fn eth_balance(&self, eth_account: String, eth_node: String) -> Result<String> {
        let account = web3::types::Address::from_str(&eth_account).unwrap();
        let transport = web3::transports::Http::new(&eth_node).unwrap();
        let web3 = web3::Web3::new(transport);

        self.task_handle.spawn("eth_balance", async move {
            let bs = web3.eth().balance(account, None).await;
            println!("Balance: {:?}", bs);
        });

        Ok("".to_string())
    }
}
