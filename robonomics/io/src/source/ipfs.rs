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
use crate::error::{Result, Error};
use futures::TryStreamExt;
use std::io::{self, Write};
use ipfs_api::IpfsClient;

#[tokio::main]
pub async fn read_file(_input: String) -> Result<()> {
    log::debug!("read_file");
    let client = IpfsClient::default();

    match client.cat("QmcAdHc6DDRHHsBxi2iccCMzZ5ihtwuWUKDzeW6MtUN24Y")
        .map_ok(|chunk| chunk.to_vec()).try_concat()
        .await {
        Ok(res) => {
            let out = io::stdout();
            let mut out = out.lock();

            out.write_all(&res).unwrap();
            Ok(())
        }
        Err(e) => {
            log::error!("error getting file: {}", e);
            Err(Error::Other(String::from("Error getting file")))
        }
    }
}
