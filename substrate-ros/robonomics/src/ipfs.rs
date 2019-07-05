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

use lazy_static::lazy_static;
use futures::{Stream, Future};
use ipfs_api::IpfsClient;
use std::sync::Mutex;
use std::{io::Write, fs::File};

lazy_static! {
    static ref IPFS: Mutex<Option<IpfsClient>> = Mutex::new(None);
}

macro_rules! ipfs {
    () => {
        IPFS.lock().unwrap().as_mut().unwrap()
    };
}

#[inline]
pub fn init() {
    let client = Some(IpfsClient::default());
    let mut ipfs = IPFS.lock().unwrap();
    *ipfs = client;
}

#[inline]
pub fn read_file(ipfs_path: &str) -> impl Future<Item=(),Error=()> {
    let mut f = File::create(ipfs_path).expect("could not create file");

    ipfs!().cat(ipfs_path)
        .for_each(move |chunk| f.write_all(&chunk).map_err(From::from))
        .map_err(|e| eprintln!("{}", e))
}
