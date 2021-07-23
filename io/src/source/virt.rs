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
use libp2p::core::PeerId;
use libp2p::request_response::*;
use libp2p::swarm::{Swarm};
use std::iter;
use std::process;
use rust_base58::FromBase58;
use robonomics_protocol::reqres::*;

/// Read line from standard console input.
pub fn stdin() -> impl Stream<Item = Result<String>> {
    let lines = io::BufReader::new(io::stdin()).lines();
    lines.map(|r| r.map_err(Into::into))
}

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
    let runtime = tokio::runtime::Runtime::new().expect("unable to start runtime");

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

/// Sends get or ping requests 
///
/// Returns response from server on get method
pub fn reqres( address: String, peerid: String, method : String,  in_value: Option<String>)
    -> Result<(
        impl Sink<Result <String>, Error = Error>,
        impl Stream<Item = Result<String>>,
)>  {
        let (sender, receiver) = mpsc::unbounded();
      
        task::spawn(async move {
        let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let peer_id = peerid;
        let remote_bytes = peer_id.from_base58().unwrap();
        let remote_peer = PeerId::from_bytes(&remote_bytes).unwrap();

        let (peer2_id, trans) = mk_transport();
        let ping_proto2 = RequestResponse::new(RobonomicsCodec {is_ping: false}, protocols, cfg);
        let mut swarm2 = Swarm::new(trans, ping_proto2, peer2_id.clone());
        log::debug!("Local peer 2 id: {:?}", peer2_id);

        let addr_remote = address;
        let addr_r : Multiaddr = addr_remote.parse().unwrap();
        swarm2.behaviour_mut().add_address(&remote_peer, addr_r.clone());

        let mut rq = Request::Ping;

        if method == "ping" {
            let req_id = swarm2.behaviour_mut().send_request(&remote_peer,rq);
            log::debug!(" peer2 Req{}: Ping  -> {:?}", req_id, remote_peer);
        } else if method == "get" {
            let value = in_value.unwrap();
            rq = Request::Get(value.clone().into_bytes());

            if let Request::Get(y) = rq {
                log::debug!(" peer2  Req: Get -> {:?} : '{}'", remote_peer, String::from_utf8_lossy(&y));
            }
            let req_encoded: Vec<u8> = bincode::serialize(&format!("{}", value).into_bytes()).unwrap();
            swarm2.behaviour_mut().send_request(&remote_peer, Request::Get(req_encoded));
        } else {
            println!("unsuported command {} ", method);
            process::exit(-1);
        }
            
        loop {
            match swarm2.next().await {
                RequestResponseEvent::Message {
                    peer,
                    message: RequestResponseMessage::Response { request_id, response }
                } => {
                    match response {
                        Response::Pong => {
                            log::debug!(" peer2 Resp{} {:?} from {:?}", request_id, &response, peer);
                            println!("{:?}", &response);
                            process::exit(0);
                        },
                        Response::Data (data) => {
                            // decode response 
                            let decoded : Vec<u8> = bincode::deserialize(&data.to_vec()).unwrap();
                            log::debug!(" peer2 Resp: Data '{}' from {:?}", String::from_utf8_lossy(&decoded[..]), remote_peer);
                            println!("{}", String::from_utf8_lossy(&decoded[..]));
                            process::exit(0);
                        }
                    }
                },

                e =>  {
                    println!("Peer2 err: {:?}", e);
                    process::exit(-2)
                }
            }
       }
    });
   Ok((sender.sink_err_into(),receiver))
}
