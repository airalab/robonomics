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
//! Simple Req-Resp Protocol

use bincode;
use libp2p::core::Multiaddr;
use libp2p::request_response::*;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use rust_base58::FromBase58;
use std::iter;

use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite, FutureExt};
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed};
use libp2p::core::ProtocolName;
use libp2p::core::{
    identity,
    muxing::StreamMuxerBox,
    transport::{self, Transport},
    upgrade, PeerId,
};
use libp2p::noise::{Keypair, NoiseConfig, X25519Spec};
use libp2p::request_response::RequestResponseCodec;
use libp2p::tcp::{GenTcpConfig, TokioTcpTransport};
use libp2p::yamux::YamuxConfig;
use std::io;

use futures::StreamExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Ping,
    Get(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Pong,
    Data(Vec<u8>),
}
impl ProtocolName for RobonomicsProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/robonomics/1".as_bytes()
    }
}

#[derive(Clone)]
pub struct RobonomicsCodec {
    pub is_ping: bool,
}

#[derive(Debug, Clone)]
pub struct RobonomicsProtocol();

#[async_trait]
impl RequestResponseCodec for RobonomicsCodec {
    type Protocol = RobonomicsProtocol;
    type Request = Request;
    type Response = Response;

    async fn read_request<T>(
        &mut self,
        _: &RobonomicsProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_length_prefixed(io, 1024)
            .map(|res| match res {
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                Ok(vec) if vec.is_empty() => {
                    self.is_ping = true; // set Ping flag; expected to reset with Pong response
                    Ok(Request::Ping)
                }
                Ok(vec) => Ok(Request::Get(vec)),
            })
            .await
    }

    async fn read_response<T>(
        &mut self,
        _: &RobonomicsProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_length_prefixed(io, 1024)
            .map(|res| match res {
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                Ok(vec) if vec.is_empty() => Ok(Response::Pong),
                Ok(vec) => Ok(Response::Data(vec)),
            })
            .await
    }

    async fn write_request<T>(
        &mut self,
        _: &RobonomicsProtocol,
        io: &mut T,
        req: Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        match req {
            Request::Ping => write_length_prefixed(io, "".as_bytes()).await,
            Request::Get(data) => write_length_prefixed(io, data).await,
        }
    }

    async fn write_response<T>(
        &mut self,
        _: &RobonomicsProtocol,
        io: &mut T,
        resp: Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        match resp {
            Response::Pong => {
                self.is_ping = false; // reset Ping flag
                write_length_prefixed(io, "".as_bytes()).await
            }
            // send Pong if somebody try in app logic to obfuscate Ping by sending Data instead of Pong
            Response::Data(data) => {
                if !self.is_ping {
                    write_length_prefixed(io, data).await
                } else {
                    write_length_prefixed(io, "".as_bytes()).await
                }
            }
        }
    }
}

pub fn mk_transport() -> (PeerId, transport::Boxed<(PeerId, StreamMuxerBox)>) {
    // if provided pk8 file with keys use it to have static PeerID
    // in other case PeerID  will be randomly generated
    let mut id_keys = identity::Keypair::generate_ed25519();
    let mut peer_id = id_keys.public().to_peer_id();

    let f = std::fs::read("private.pk8");
    match f {
        Ok(mut bytes) => {
            id_keys = identity::Keypair::rsa_from_pkcs8(&mut bytes).unwrap();
            peer_id = id_keys.public().to_peer_id();
            log::debug!("try get peer ID from keypair at file");
        }
        Err(_e) => log::debug!("try to use peer ID from random keypair"),
    };

    let transport = TokioTcpTransport::new(GenTcpConfig::default().nodelay(true));

    let noise_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&id_keys)
        .unwrap();
    (
        peer_id,
        transport
            .upgrade(upgrade::Version::V1)
            .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(YamuxConfig::default())
            .boxed(),
    )
}

/// Request Response client API
/// Sends get or ping requests
///
/// Returns response from server on get method
#[tokio::main]
pub async fn reqres(
    address: String,
    peerid: String,
    method: String,
    in_value: Option<String>,
) -> Result<String, String> {
    let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
    let cfg = RequestResponseConfig::default();

    let peer_id = peerid;
    let remote_bytes = peer_id.from_base58().unwrap();
    let remote_peer = PeerId::from_bytes(&remote_bytes).unwrap();

    let (peer2_id, trans) = mk_transport();
    let ping_proto2 = RequestResponse::new(RobonomicsCodec { is_ping: false }, protocols, cfg);

    let mut swarm2 = {
        SwarmBuilder::new(trans, ping_proto2, peer2_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build()
    };

    log::debug!("Local peer 2 id: {:?}", peer2_id);

    let addr_remote = address;
    let addr_r: Multiaddr = addr_remote.parse().unwrap();
    swarm2
        .behaviour_mut()
        .add_address(&remote_peer, addr_r.clone());

    let mut rq = Request::Ping;

    if method == "ping" {
        let req_id = swarm2.behaviour_mut().send_request(&remote_peer, rq);
        log::debug!(" peer2 Req{}: Ping  -> {:?}", req_id, remote_peer);
    } else if method == "get" {
        let value = in_value.unwrap();
        rq = Request::Get(value.clone().into_bytes());

        if let Request::Get(y) = rq {
            log::debug!(
                " peer2  Req: Get -> {:?} : '{}'",
                remote_peer,
                String::from_utf8_lossy(&y)
            );
        }
        let req_encoded: Vec<u8> = bincode::serialize(&value.into_bytes()).unwrap();
        swarm2
            .behaviour_mut()
            .send_request(&remote_peer, Request::Get(req_encoded));
    } else {
        println!("unsuported command {method} ");
    }

    let received = loop {
        match swarm2.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                log::debug!("Peer2 connected: {:?}", peer_id);
            }

            SwarmEvent::Dialing(peer_id) => {
                log::debug!("Peer2 dial: {:?}", peer_id);
            }

            SwarmEvent::Behaviour(event) => match event {
                RequestResponseEvent::Message {
                    peer,
                    message:
                        RequestResponseMessage::Response {
                            request_id,
                            response,
                        },
                } => match response {
                    Response::Pong => {
                        let pong_msg =
                            format!(" peer2 Resp{} {:?} from {:?}", request_id, &response, peer);
                        log::debug!("{}", pong_msg);
                        break pong_msg;
                    }
                    Response::Data(data) => {
                        let decoded: Vec<u8> = bincode::deserialize(&data.to_vec()).unwrap();
                        log::debug!(
                            " peer2 Resp: Data '{}' from {:?}",
                            String::from_utf8_lossy(&decoded[..]),
                            remote_peer
                        );
                        let data_msg = format!("{}", String::from_utf8_lossy(&decoded[..]));
                        log::debug!("{}", data_msg);
                        break data_msg;
                    }
                },

                e => {
                    let err_msg = format!("Peer2 err: {e:?}");
                    println!("{err_msg}");
                    break err_msg;
                }
            },

            _ => {}
        };
    };
    Ok(received)
}
