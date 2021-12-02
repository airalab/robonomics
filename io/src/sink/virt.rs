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

use async_std::{io, task};
use futures::{channel::mpsc, prelude::*, stream::StreamExt};
use ipfs_api::{IpfsClient, TryFromUri};
use robonomics_protocol::{
    pubsub::{self, Multiaddr, PubSub as _},
    subxt::{datalog, launch},
};
use sp_core::{crypto::Pair, sr25519};
use std::io::Cursor;
use std::time::Duration;

use bincode;
use chrono::prelude::*;
use libp2p::request_response::*;
use libp2p::swarm::{Swarm, SwarmEvent};
use robonomics_protocol::reqres::*;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::iter;

use crate::error::{Error, Result};

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

/// Listen for ping or get requests from clients
///
/// Returns response to clients pong or data
pub fn reqres(address: String) -> Result<impl Stream<Item = String>> {
    log::debug!(target: "robonomics-io", "reqres: bind address {}", address);

    let (sender, receiver) = mpsc::unbounded();
    // thread 'main' panicked at 'there is no reactor running, must be called from the context of a Tokio 1.x runtime', io/src/sink/virt.rs:183:5
    task::spawn(async move {
        let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let (peer1_id, trans) = mk_transport();
        let ping_proto1 = RequestResponse::new(
            RobonomicsCodec { is_ping: false },
            protocols.clone(),
            cfg.clone(),
        );
        let mut swarm1 = Swarm::new(trans, ping_proto1, peer1_id);

        let addr_local = address;
        let addr: Multiaddr = addr_local.parse().unwrap();

        swarm1.listen_on(addr.clone()).unwrap();
        let mut peer_id = String::new();
        fmt::write(&mut peer_id, format_args!("{:?}", peer1_id)).unwrap();
        log::debug!("Local peer 1 id: {}", peer_id.clone());
        let mut file = File::create("peerid.txt").unwrap();
        file.write_all(peer_id.as_bytes())
            .expect("Unable to write data");

        loop {
            match swarm1.next_event().await {
                SwarmEvent::NewListenAddr(addr) => {
                    log::debug!("Peer 1 listening on {}", addr.clone());
                }

                SwarmEvent::Behaviour(RequestResponseEvent::Message {
                    peer,
                    message:
                        RequestResponseMessage::Request {
                            request, channel, ..
                        },
                }) => {
                    // match type of request: Ping or Get and handle
                    match request {
                        Request::Get(data) => {
                            //decode received request
                            let decoded: Vec<u8> = bincode::deserialize(&data.to_vec()).unwrap();
                            log::debug!(
                                " peer1 Get '{}' from  {:?}",
                                String::from_utf8_lossy(&decoded[..]),
                                peer
                            );
                            let mut msg = String::new();
                            fmt::write(
                                &mut msg,
                                format_args!("{}", String::from_utf8_lossy(&decoded[..])),
                            )
                            .unwrap();
                            let _ = sender.unbounded_send(msg);
                            // send encoded response
                            let resp_encoded: Vec<u8> =
                                bincode::serialize(&format!("{}", epoch()).into_bytes()).unwrap();
                            swarm1
                                .behaviour_mut()
                                .send_response(channel, Response::Data(resp_encoded))
                                .unwrap();
                        }
                        Request::Ping => {
                            log::debug!(" peer1 {:?} from {:?}", request, peer);
                            let resp: Response = Response::Pong;
                            log::debug!(" peer1 {:?} to   {:?}", resp, peer);
                            swarm1
                                .behaviour_mut()
                                .send_response(channel, resp.clone())
                                .unwrap();
                        }
                    }
                }

                SwarmEvent::Behaviour(RequestResponseEvent::ResponseSent { peer, .. }) => {
                    log::debug!("Response sent to {:?}", peer);
                }

                SwarmEvent::Behaviour(e) => println!("Peer1: Unexpected event: {:?}", e),
                _ => {}
            }
        }
    });
    Ok(receiver)
}

fn epoch() -> i64 {
    let ts = Utc::now();
    ts.timestamp() * 1000 + (ts.nanosecond() as i64) / 1000 / 1000
}
