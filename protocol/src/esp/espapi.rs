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
//! Robonomics Network protocol.

use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

#[rpc]
pub trait ESPApi {
    #[rpc(name = "esp_init")]
    fn init(&self) -> Result<u64>;

    #[rpc(name = "esp_send")]
    fn send(&self) -> Result<u64>;
}

pub struct ESP;

impl ESPApi for ESP {
    fn init(&self) -> Result<u64> {
        Ok(8)
    }

    fn send(&self) -> Result<u64> {
        Ok(42)
    }
}
