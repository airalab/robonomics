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
use std::io::Cursor;
use ipfs_api::IpfsClient;

#[tokio::main]
pub async fn add_file(input: Vec<u8>) -> Result<()> {
    let client = IpfsClient::default();
    let data = Cursor::new(input);

    match client.add(data).await {
        Ok(res) => {
            println!("{}", res.hash);
            Ok(())
        },
        Err(e) => {
            log::error!("error adding file: {}", e);
            Err(Error::Other(String::from("Error adding file")))
        }
    }
}
