///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life> 
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
//! Robonomics data blockchainization. 

use sp_core::{sr25519, crypto::{Pair, Ss58Codec}};
use robonomics_protocol::datalog;
use async_std::task;
use crate::error::Result;

/// Wrapper type for byte vector.
type Bytes = Vec<u8>;

/// Data blockchainization subsystem command.
#[derive(structopt::StructOpt, Clone, Debug)]
pub struct DatalogCmd {
    /// Substrate node WebSocket endpoint
    #[structopt(long, default_value = "ws://localhost:9944")]
    remote: String,
    /// Sender account seed URI
    #[structopt(short)]
    suri: String,
    /// Hex encoded data record to send (without 0x prefix)
    #[structopt(parse(try_from_str = hex::decode))]
    record: Option<Bytes>,
}

impl DatalogCmd {
    /// Send data record into blockchain.
    pub fn run(&self) -> Result<()> {
        let signer = sr25519::Pair::from_string(self.suri.as_str(), None)?;
        log::info!(
            target: "robonomics-cli",
            "Key loaded: {}", signer.public().to_ss58check(),
        );

        if let Some(record) = self.record.clone() {
            task::block_on(
                datalog::submit(
                    signer,
                    self.remote.as_str(),
                    record
                )
            ).map_err(Into::into)
        } else {
            let stdin = robonomics_io::Stdin::new()?;
            task::block_on(
        }
    }
}
