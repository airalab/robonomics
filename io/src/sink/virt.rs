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
//! Virtual sinkable devices.

use async_compat::CompatExt;
use futures::{channel::mpsc, io::BufWriter, prelude::*, stream::StreamExt};
use ipfs_api::{IpfsClient, TryFromUri};
use robonomics_protocol::{
    pubsub::{self, Multiaddr, PubSub as _},
    subxt::{datalog, launch},
};
use sp_core::{crypto::Pair, sr25519};
use std::io::Cursor;
use std::time::Duration;
use tokio::task;

use crate::error::{Error, Result};

/// Print on standard console output.
pub fn stdout() -> impl Sink<String, Error = Error> {
    BufWriter::new(tokio::io::stdout().compat())
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
    task::spawn(receiver.for_each(move |msg| future::ready(pubsub.publish(&topic_name, msg))));

    Ok(sender.sink_err_into())
}

/// Submit signed data record into blockchain.
///
/// Returns hash of sended datalog extrinsic.
pub fn datalog<T: Into<Vec<u8>>>(
    remote: String,
    suri: String,
    rws: Option<String>,
) -> Result<(
    impl Sink<T, Error = Error>,
    impl Stream<Item = Result<[u8; 32]>>,
)> {
    let pair = sr25519::Pair::from_string(suri.as_str(), None)?;

    let (sender, receiver) = mpsc::unbounded();
    let hashes = receiver.then(move |msg: T| {
        datalog::submit(pair.clone(), remote.clone(), msg.into(), rws.clone())
            .map(|r| r.map_err(Into::into))
    });
    Ok((sender.sink_err_into(), hashes))
}

/// Upload some data into IPFS network.
///
/// Returns IPFS hash of consumed data objects.
pub fn ipfs<'a, T>(
    uri: &'a str,
) -> Result<(
    impl Sink<T, Error = Error>,
    impl Stream<Item = Result<String>> + 'a,
)>
where
    T: AsRef<[u8]> + Send + Sync + 'static,
{
    let (sender, receiver) = mpsc::unbounded();
    let hashes = receiver.then(move |msg: T| async move {
        let client = IpfsClient::from_str(uri).expect("unvalid uri");
        client
            .add(Cursor::new(msg))
            .map_ok(|x| x.hash)
            .map_err(|e| e.to_string().into())
            .await
    });
    Ok((sender.sink_err_into(), hashes))
}

/// Submit signed launch request into blockchain.
///
/// Returns hash of sended launch extrinsic.
pub fn launch(
    remote: String,
    suri: String,
    robot: String,
    rws: Option<String>,
) -> Result<(
    impl Sink<bool, Error = Error>,
    impl Stream<Item = Result<[u8; 32]>>,
)> {
    let pair = sr25519::Pair::from_string(suri.as_str(), None)?;

    let (sender, receiver) = mpsc::unbounded();
    let hashes = receiver.then(move |signal: bool| {
        launch::submit(
            pair.clone(),
            remote.clone(),
            robot.clone(),
            signal,
            rws.clone(),
        )
        .map(|r| r.map_err(Into::into))
    });
    Ok((sender.sink_err_into(), hashes))
}

#[cfg(feature = "ros")]
/// Publish message to ROS topic.
pub fn ros(topic: &str, queue_size: usize) -> Result<impl Sink<String, Error = Error>> {
    let _ = rosrust::try_init_with_options("robonomics", false);
    let publisher = rosrust::publish(topic, queue_size)?;

    let (sender, receiver) = mpsc::unbounded();
    task::spawn(receiver.for_each(move |data| {
        let mut msg = substrate_ros_msgs::std_msgs::String::default();
        msg.data = data;
        let _ = publisher.send(msg);
        future::ready(())
    }));

    Ok(sender.sink_err_into())
}
