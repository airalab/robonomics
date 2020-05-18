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
//! Virtual sinkable devices.

use robonomics_protocol::{pubsub::{self, Multiaddr, PubSub as _}, datalog};
use ipfs_api::{IpfsClient, TryFromUri};
use sp_core::{sr25519, crypto::Pair};
use futures::channel::mpsc;
use async_std::{io, task};
use futures::prelude::*;
use std::time::Duration;
use std::io::Cursor;

use crate::error::{Result, Error};

/// Print on standard console output.
pub fn stdout() -> impl Sink<String, Error = Error> {
    io::BufWriter::new(io::stdout())
        .into_sink()
        .with(|s| {
            let line: Result<String> = Ok(format!("{}\n", s));
            futures::future::ready(line)
        })
        .sink_err_into()
}

/// Publish data into PubSub topic. 
pub fn pubsub<T: Into<Vec<u8>> + Send + 'static>(
    listen: Multiaddr,
    bootnodes: Vec<Multiaddr>,
    topic_name: String,
    heartbeat: Duration,
) -> Result<impl Sink<T, Error = Error>> {
    let (pubsub, worker) = pubsub::Gossipsub::new(heartbeat)?;

    // Listen address
    let _ = pubsub.listen(listen);

    // Connect to bootnodes
    for addr in bootnodes {
        let _ = pubsub.connect(addr);
    }

    // Spawn peer discovery
    task::spawn(pubsub::discovery::start(pubsub.clone()));

    // Spawn network worker
    task::spawn(worker);

    // Spawn message publisher task
    let (sender, receiver) = mpsc::unbounded();
    task::spawn(receiver.for_each(move |msg|
        future::ready(pubsub.publish(&topic_name, msg))
    ));

    Ok(sender.sink_err_into())
}

/// Submit signed data record into blockchain.
///
/// Returns hash of sended datalog extrinsic.
pub fn datalog<T: Into<Vec<u8>>>(
    remote: String,
    suri: String,
) -> Result<(impl Sink<T, Error = Error>, impl Stream<Item = Result<[u8; 32]>>)> {
    let pair = sr25519::Pair::from_string(suri.as_str(), None)?;

    let (sender, receiver) = mpsc::unbounded();
    let hashes = receiver.then(move |msg: T|
        datalog::submit(pair.clone(), remote.clone(), msg.into())
            .map(|r| r.map_err(Into::into))
    );
    Ok((sender.sink_err_into(), hashes))
}

/// Upload some data into IPFS network.
///
/// Returns IPFS hash of consumed data objects.
pub fn ipfs<T>(
    uri: &str,
) -> Result<(impl Sink<T, Error = Error>, impl Stream<Item = Result<String>>)> where
    T: AsRef<[u8]> + Send + Sync + 'static,
{
    let client = IpfsClient::from_str(uri).expect("unvalid uri");
    let mut runtime = tokio::runtime::Runtime::new().expect("unable to start runtime");

    let (sender, receiver) = mpsc::unbounded();
    let hashes = receiver.map(move |msg: T|
        runtime.block_on(client.add(Cursor::new(msg)))
            .map(|value| value.hash)
            .map_err(Into::into)
    );
    Ok((sender.sink_err_into(), hashes))
}
