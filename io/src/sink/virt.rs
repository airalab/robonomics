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
use futures::channel::mpsc;
use futures::prelude::*;
use ipfs_api::{IpfsClient, TryFromUri};
use robonomics_protocol::{
    pubsub::{self, Multiaddr, PubSub as _},
    subxt::{datalog, launch},
};
use sp_core::{crypto::Pair, sr25519};
use std::io::Cursor;
use std::time::Duration;

use crate::error::{Error, Result};

use bincode;
use libp2p::core::{
    identity,
    muxing::StreamMuxerBox,
    transport::{self, Transport},
    upgrade, PeerId,
};
use libp2p::noise::{Keypair, NoiseConfig, X25519Spec};
use libp2p::request_response::*;
use libp2p::swarm::Swarm;
use libp2p::tcp::TcpConfig;
use rust_base58::FromBase58;
use std::iter;
use std::process;
use robonomics_protocol::reqres::*;

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
pub fn ipfs<T>(
    uri: &str,
) -> Result<(
    impl Sink<T, Error = Error>,
    impl Stream<Item = Result<String>>,
)>
where
    T: AsRef<[u8]> + Send + Sync + 'static,
{
    let client = IpfsClient::from_str(uri).expect("unvalid uri");
    let mut runtime = tokio::runtime::Runtime::new().expect("unable to start runtime");

    let (sender, receiver) = mpsc::unbounded();
    let hashes = receiver.map(move |msg: T| {
        runtime
            .block_on(client.add(Cursor::new(msg)))
            .map(|x| x.hash)
            .map_err(|e| e.to_string().into())
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

/// client what sends get or ping requests and expects response from server 
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
