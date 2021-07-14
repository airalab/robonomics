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
//! Virtual sensors collection.

use async_std::{io, task};
use futures::{channel::mpsc, prelude::*};
use ipfs_api::{IpfsClient, TryFromUri};
use robonomics_protocol::pubsub::{self, Multiaddr, PubSub as PubSubT};
use robonomics_protocol::subxt::{datalog, AccountId};
use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
use std::time::Duration;

use crate::error::{Error, Result};

use bincode;
use chrono::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use libp2p::core::{
    identity,
    muxing::StreamMuxerBox,
    transport::{self, Transport},
    upgrade, PeerId,
};
use libp2p::noise::{Keypair, NoiseConfig, X25519Spec};
use libp2p::request_response::*;
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::tcp::TcpConfig;
use std::fmt;
use std::iter;

use robonomics_protocol::reqres::*;

/// Read line from standard console input.
pub fn stdin() -> impl Stream<Item = Result<String>> {
    let lines = io::BufReader::new(io::stdin()).lines();
    lines.map(|r| r.map_err(Into::into))
}

/// Subscribe for data from PubSub topic.
pub fn pubsub(
    listen: Multiaddr,
    bootnodes: Vec<Multiaddr>,
    topic_name: String,
    heartbeat: Duration,
) -> Result<impl Stream<Item = Result<pubsub::Message>>> {
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

    // Subscribe to given topic
    Ok(pubsub.subscribe(&topic_name).map(|v| Ok(v)))
}

/// Read data records from blockchain.
///
/// Returns datalog data objects.
pub fn datalog(
    remote: String,
    address: String,
) -> Result<impl Stream<Item = Result<Vec<(u64, Vec<u8>)>>>> {
    let robot_account =
        AccountId::from_ss58check(address.as_str()).map_err(|_| Error::Ss58CodecError)?;

    let (mut sender, receiver) = mpsc::unbounded();
    task::spawn(async move {
        sender.send(robot_account).await.unwrap();
    });
    let data = receiver.then(move |robot_account: AccountId| {
        datalog::fetch(robot_account.clone(), remote.clone()).map(|r| r.map_err(Into::into))
    });
    Ok(data)
}

/// Download some data from IPFS network.
///
/// Returns IPFS data objects.
pub fn ipfs(
    uri: &str,
) -> Result<(
    impl Sink<String, Error = Error>,
    impl Stream<Item = Result<Vec<u8>>>,
)> {
    let client = IpfsClient::from_str(uri).expect("unvalid uri");
    let mut runtime = tokio::runtime::Runtime::new().expect("unable to start runtime");

    let (sender, receiver) = mpsc::unbounded();
    let datas = receiver.map(move |msg: String| {
        runtime
            .block_on(client.cat(msg.as_str()).map_ok(|c| c.to_vec()).try_concat())
            .map_err(|e| e.to_string().into())
    });
    Ok((sender.sink_err_into(), datas))
}

/// Listen for launch events on the blockchain.
///
/// Returns launch parameter, event sender account.
pub fn launch(
    remote: String,
    format: Ss58AddressFormat,
) -> impl Stream<Item = (String, String, bool)> {
    let (mut sender, receiver) = mpsc::unbounded();

    task::spawn(robonomics_protocol::subxt::launch::listen(
        remote,
        move |event| {
            let _ = sender.send((
                event.sender.to_ss58check_with_version(format),
                event.robot.to_ss58check_with_version(format),
                event.param,
            ));
        },
    ));

    receiver
}

#[cfg(feature = "ros")]
/// Subscribe for messages from ROS topic.
pub fn ros(
    topic: &str,
    queue_size: usize,
) -> Result<(impl Stream<Item = String>, rosrust::Subscriber)> {
    let _ = rosrust::try_init_with_options("robonomics", false);
    let (sender, receiver) = mpsc::unbounded();
    let safe_sender = std::sync::RwLock::new(sender);
    let subscriber = rosrust::subscribe(
        topic,
        queue_size,
        move |msg: substrate_ros_msgs::std_msgs::String| {
            let mut sender = safe_sender.write().unwrap();
            let _ = sender.send(msg.data);
        },
    )?;
    Ok((receiver, subscriber))
}

pub fn reqres(address: String) -> Result<impl Stream<Item = String>> { 
    log::debug!(target: "robonomics-io", "reqres: bind address {}", address);

    let (sender, receiver) = mpsc::unbounded();

    task::spawn(async move {
        let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let (peer1_id, trans) = mk_transport();
        let ping_proto1 = RequestResponse::new(RobonomicsCodec{is_ping: false}, protocols.clone(), cfg.clone());
        let mut swarm1 = Swarm::new(trans, ping_proto1, peer1_id);

        let addr_local = address;
        let addr: Multiaddr = addr_local.parse().unwrap();

        swarm1.listen_on(addr.clone()).unwrap();
        let mut peer_id = String::new();
        fmt::write (&mut peer_id, format_args!("{:?}", peer1_id)).unwrap();
        log::debug!("Local peer 1 id: {}", peer_id.clone());
        let mut file = File::create("peerid.txt").unwrap();
        file.write_all(peer_id.as_bytes()).expect("Unable to write data");

        loop {
            match swarm1.next_event().await {
                SwarmEvent::NewListenAddr(addr) => {
                    log::debug!("Peer 1 listening on {}", addr.clone());
                },

                SwarmEvent::Behaviour(RequestResponseEvent::Message {
                    peer,
                    message: RequestResponseMessage::Request { request, channel, .. }
                }) => {
                    // match type of request: Ping or Get and handle
                    match request {
                        Request::Get(data) =>  {
                            //decode received request
                            let decoded : Vec<u8> = bincode::deserialize(&data.to_vec()).unwrap();
                            log::debug!(" peer1 Get '{}' from  {:?}", String::from_utf8_lossy(&decoded[..]), peer);
                            let mut msg = String::new();
                            fmt::write (&mut msg, format_args!("{}", String::from_utf8_lossy(&decoded[..]))).unwrap();
                            let _ = sender.unbounded_send(msg);
                            // send encoded response
                            let resp_encoded: Vec<u8> = bincode::serialize(&format!("{}", epoch()).into_bytes()).unwrap();
                            swarm1.behaviour_mut().send_response(channel, Response::Data(resp_encoded)).unwrap();
                        },
                        Request::Ping =>  {
                            log::debug!(" peer1 {:?} from {:?}", request, peer);
                            let resp: Response = Response::Pong;
                            log::debug!(" peer1 {:?} to   {:?}", resp, peer);
                            swarm1.behaviour_mut().send_response(channel, resp.clone()).unwrap();
                        },
                    }
                },

                SwarmEvent::Behaviour(RequestResponseEvent::ResponseSent {
                    peer, ..
                }) => {
                    log::debug!("Response sent to {:?}",  peer);
                }

                SwarmEvent::Behaviour(e) =>println!("Peer1: Unexpected event: {:?}", e),
                _ => {}
            }
         };
    });
    Ok(receiver)
}

fn mk_transport() -> (PeerId, transport::Boxed<(PeerId, StreamMuxerBox)>) {
    // if provided pk8 file with keys use it to have static PeerID 
    // in other case PeerID  will be randomly generated
    let mut id_keys = identity::Keypair::generate_ed25519();
    let mut peer_id = id_keys.public().into_peer_id();

    let f = std::fs::read("private.pk8");
    let _ = match f {
        Ok(mut bytes) =>  {
        id_keys = identity::Keypair::rsa_from_pkcs8(&mut bytes).unwrap();
        peer_id = id_keys.public().into_peer_id();
        log::debug!("try get peer ID from keypair at file");
       },
        Err(_e) =>  log::debug!("try to use peer ID from random keypair"),
    };

    let noise_keys = Keypair::<X25519Spec>::new().into_authentic(&id_keys).unwrap();
    (peer_id, TcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(libp2p_yamux::YamuxConfig::default())
        .boxed())
}

fn epoch () -> i64 {
   let ts =  Utc::now();
   ts.timestamp() * 1000 + ( ts.nanosecond() as i64 )/ 1000 / 1000
}
