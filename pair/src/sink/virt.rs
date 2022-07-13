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

extern crate structopt;

use async_std::{io};
use futures::{prelude::*};
use libp2p::Multiaddr;
use libp2p::PeerId;

use clap;
use clap::Parser;

use bincode;
use chrono::prelude::*;
use libp2p::request_response::*;
use libp2p::swarm::{Swarm, SwarmEvent};
use robonomics_protocol::reqres::*;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::iter;
use std::{thread, time};
use std::process;
use rust_base58::FromBase58;

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

// clap::Subcommand - only for emuns

#[derive(Debug, Parser)]
pub struct PairCmd {
   /// Pair operation to run.
   #[clap(subcommand)]
   pub subcommand: Option<PairSubCmds>, 
}

impl PairCmd {
    pub fn run (&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum PairSubCmds {
    Connect(ConnectCmd),
    Listen(ListenCmd),
}

#[derive(Debug, Parser)]   
pub struct ListenCmd { 

    #[clap(long, value_parser)]
    pub key: Option<String>,
    
    #[clap(long, value_parser)]
    pub addr: Option<String>,
}

impl ListenCmd {
    pub fn run (&self) -> Result<()> {

        let peer:String;

        match &self.key {
            Some(key) => {
                //println!("key {:?}", peer);
                peer = key.to_string();
            },
            _ => todo!(),
        }

        let address:String;

        if let Some(x) =  &self.addr {
            address = x.to_string();
        } else {
            address = "/ip4/127.0.0.1/tcp/61241".to_string();
        }

        let _ = reqres_server(address.to_string(), peer);
        Ok(())
    }
}
#[derive(Debug, Parser)]   
pub struct ConnectCmd { 

    #[clap(long, value_parser)]
    pub key: Option<String>,
    
    #[clap(long, value_parser)]
    pub addr: Option<String>,
}

impl ConnectCmd {
    pub fn run (&self) -> Result<()> {
        
        let peer:String;

        match &self.key {
            Some(key) => {
                //println!("key {:?}", peer);
                peer = key.to_string();
            },
            _ => todo!(),
        }

        let address:String;

        if let Some(x) =  &self.addr {
            address = x.to_string();
        } else {
            address = "/ip4/127.0.0.1/tcp/61241".to_string();
        }

        //let _ = reqres(address, peer, "connect".to_string());
        let _ = reqres_client(address, peer);
        
        Ok(())
    }
}

/// Listen for ping or get requests from clients
///
/// Returns response to clients pong or data
pub fn reqres_server(address: String, node: String) -> Result<String> {
    env_logger::init();

    log::debug!(target: "robonomics-io", "reqres: bind address {}", address);

    //let (sender, receiver) = mpsc::unbounded();

    // thread 'main' panicked at 'there is no reactor running, must be called from the context of a Tokio 1.x runtime', io/src/sink/virt.rs:183:5
    // task::spawn(
        let peer1 =     async move {
        let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let (peer1_id, trans) = mk_transport();
        let ping_proto1 = RequestResponse::new(
            RobonomicsCodec { is_ping: false },
            protocols.clone(),
            cfg.clone(),
        );
        let mut swarm1 = Swarm::new(trans, ping_proto1, peer1_id);

        let addr_local = address.clone();       
        let addr: Multiaddr = addr_local.parse().unwrap();

        swarm1.listen_on(addr.clone()).unwrap();
        let mut peer_id = String::new();
        fmt::write(&mut peer_id, format_args!("{:?}", peer1_id)).unwrap();
        log::debug!("Local peer 1 id: {}", peer_id.clone());
        let mut file = File::create("peerid.txt").unwrap();

        let peer_adr = peer_id.clone();
        let split_adr: Vec<&str> =  peer_adr.split(|c| c == '"' ).collect();
        println!("address: {} {}", address, split_adr[1].clone());

        file.write_all(split_adr[1].clone().as_bytes())
            .expect("Unable to write data");

        loop {
            //match swarm1.next_event().await {
            match swarm1.select_next_some().await {
                SwarmEvent::NewListenAddr{ address, .. } => {
                    log::debug!("Peer 1 listening on {}", address.clone());
                }

                SwarmEvent::ConnectionEstablished {peer_id, endpoint, ..} => {
                    log::debug!("incomming node id {} {}", peer_id, endpoint.get_remote_address());
                    if node != peer_id.to_string() {                            
                            println!("save node to pair {} {} ", peer_id, endpoint.get_remote_address());
                            // TODO store somewhere paired peerID 
                            swarm1.disconnect_peer_id(peer_id).unwrap();
                    } else {
                        println!("continue with expected node {}", peer_id.clone());
                    }
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
                            // let _ = sender.unbounded_send(msg);
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
    }
    //)
    ;
    //Ok(receiver);

    let _ = futures::executor::block_on(peer1);
    Ok("".to_string())
}

/// Send ping or get requests to client
pub fn reqres_client (address: String, node: String) -> Result<String> {
    env_logger::init();

    log::debug!(target: "robonomics-io", "reqres: bind address {}", address);

    //let (sender, receiver) = mpsc::unbounded();
    let ms = time::Duration::from_millis(100);

    // thread 'main' panicked at 'there is no reactor running, must be called from the context of a Tokio 1.x runtime', io/src/sink/virt.rs:183:5
    // task::spawn(
        let peer2 = async move {
            let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
            let cfg = RequestResponseConfig::default();
    
            //let peer_id = std::env::args().nth(2).unwrap();//
            let peer_id = node;
            let remote_bytes = peer_id.from_base58().unwrap();
            let remote_peer = PeerId::from_bytes(&remote_bytes).unwrap();
    
            let (peer2_id, trans) = mk_transport();
            let ping_proto2 = RequestResponse::new(RobonomicsCodec { is_ping: false }, protocols, cfg);
            let mut swarm2 = Swarm::new(trans, ping_proto2, peer2_id.clone());
            println!("Local peer 2 id: {:?}", peer2_id);
    
            // TODO discovery, now  server addres is fixed
            let addr_remote = "/ip4/127.0.0.1/tcp/61241";

            let addr_r: Multiaddr = addr_remote.parse().unwrap();
            swarm2
                .behaviour_mut()
                .add_address(&remote_peer, addr_r.clone());
    
            let mut count: i64 = 0;
            let mut rq = Request::Ping;
            let mut req_id = swarm2
                .behaviour_mut()
                .send_request(&remote_peer, rq.clone());
            println!(
                " peer2 Req{}: Ping  -> {:?} : {:?}",
                req_id, remote_peer, rq
            );
    
            loop {
                match swarm2.select_next_some().await {
                    SwarmEvent::Behaviour(event) => match event {
                        RequestResponseEvent::Message {
                            peer,
                            message:
                                RequestResponseMessage::Response {
                                    request_id,
                                    response,
                                },
                        } => {
                            match response {
                                Response::Pong => {
                                    println!(
                                        " peer2 Resp{} {:?} from {:?}",
                                        request_id, &response, peer
                                    );
                                }
                                Response::Data(data) => {
                                    // decode response
                                    let decoded: Vec<u8> =
                                        bincode::deserialize(&data.to_vec()).unwrap();
                                    println!(
                                        " peer2 Resp{}: Data '{}' from {:?}",
                                        req_id,
                                        String::from_utf8_lossy(&decoded[..]),
                                        remote_peer
                                    );
                                }
                            }
                            rq = Request::Get(count.to_string().into_bytes());
                            // send encoded request
                            if let Request::Get(y) = rq {
                                println!(
                                    " peer2  Req{}: Get -> {:?} : '{}'",
                                    req_id,
                                    remote_peer,
                                    String::from_utf8_lossy(&y)
                                );
                            }
                            let req_encoded: Vec<u8> =
                                bincode::serialize(&format!("{}", count).into_bytes()).unwrap();
                            req_id = swarm2
                                .behaviour_mut()
                                .send_request(&remote_peer, Request::Get(req_encoded));
                            count += 1;
                            thread::sleep(ms);
                        }
                        _ => {}
                    },
    
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("Peer2 connected with: {:?}", peer_id);
                    }

                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        println!("Peer2 disconnected from: {:?}", peer_id);
                        process::exit(0x0100)
                    } 
    
                    SwarmEvent::Dialing(peer_id) => {
                        println!("Peer2 dial to: {:?}", peer_id);
                    }
    
                    e => {
                        println!("Peer2 error: {:?}", e);
                        //process::exit(0x0100)
                    }
                }
            }
        };

    let _ = futures::executor::block_on(peer2);
    Ok("".to_string())
}


fn epoch() -> i64 {
    let ts = Utc::now();
    ts.timestamp() * 1000 + (ts.nanosecond() as i64) / 1000 / 1000
}
