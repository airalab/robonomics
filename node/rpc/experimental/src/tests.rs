///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
#[cfg(test)]

    use super::*;
    use bincode;
    use futures::StreamExt;
    use libp2p::core::Multiaddr;
    use libp2p::request_response::*;
    use libp2p::swarm::{SwarmBuilder, SwarmEvent};
    use robonomics_protocol::reqres::*;
    use std::iter;
    use tokio;
    use tokio::runtime::Handle;

    #[test]
    // test spilt multiaddress on transport address and peerID
    fn test_get_addr() {
        assert_eq!(
            reqresrpc::get_addr(
                "/ip4/192.168.0.103/tcp/61241/QmZDuvm3dEjSgD9nq6a7d1b1kccfFjBdcHSMzCB9ULAcoH"
                    .to_string()
            ),
            (
                "/ip4/192.168.0.103/tcp/61241".to_string(),
                "QmZDuvm3dEjSgD9nq6a7d1b1kccfFjBdcHSMzCB9ULAcoH".to_string()
            )
        );
    }

    #[tokio::main]
    #[test]
    // test p2p_ping and p2p_get
    // Steps:
    // - create peerID,
    // - run node with it,
    // - test Ping to this peerID on OK condition,
    // - test Ping to other peerID on NOK condition,
    // - test Get with check of echo message.

    async fn test_p2p_ping_and_get() {
        let (peer1_id, trans) = mk_transport();

        let peer1 = async move {
            let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
            let cfg = RequestResponseConfig::default();

            let ping_proto1 = RequestResponse::new(
                RobonomicsCodec { is_ping: false },
                protocols.clone(),
                cfg.clone(),
            );

            let mut swarm1 = {
                SwarmBuilder::new(trans, ping_proto1, peer1_id)
                    .executor(Box::new(|fut| {
                        tokio::spawn(fut);
                    }))
                    .build()
            };

            let addr_local = "/ip4/127.0.0.1/tcp/61241";
            let addr: Multiaddr = addr_local.parse().unwrap();

            swarm1.listen_on(addr.clone()).unwrap();
            println!("Local peer 1 id: {:?}", peer1_id);

            loop {
                match swarm1.select_next_some().await {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Peer 1 listening on {}", address.clone());
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
                                let decoded: Vec<u8> =
                                    bincode::deserialize(&data.to_vec()).unwrap();
                                println!(
                                    " peer1 Get '{}' from  {:?}",
                                    String::from_utf8_lossy(&decoded[..]),
                                    peer
                                );
                                let resp: Response = Response::Data(
                                    format!("Hello {}", String::from_utf8_lossy(&decoded[..]))
                                        .into_bytes(),
                                );
                                if let Response::Data(y) = resp.clone() {
                                    println!(
                                        " peer1 Resp::Data '{}' to {:?}",
                                        String::from_utf8_lossy(&y),
                                        peer
                                    );
                                }
                                // send encoded response
                                let resp_encoded: Vec<u8> = bincode::serialize(
                                    &format!("Hello {}", String::from_utf8_lossy(&decoded[..]))
                                        .into_bytes(),
                                )
                                .unwrap();
                                swarm1
                                    .behaviour_mut()
                                    .send_response(channel, Response::Data(resp_encoded))
                                    .unwrap();
                            }

                            Request::Ping => {
                                println!(" peer1 {:?} from {:?}", request, peer);
                                let resp: Response = Response::Pong;
                                println!(" peer1 {:?} to   {:?}", resp, peer);
                                swarm1
                                    .behaviour_mut()
                                    .send_response(channel, resp.clone())
                                    .unwrap();
                            }
                        }
                    }

                    SwarmEvent::Behaviour(RequestResponseEvent::ResponseSent { peer, .. }) => {
                        println!("Response sent to {:?}", peer);
                    }

                    SwarmEvent::Behaviour(e) => println!("Peer1: Unexpected event: {:?}", e),
                    _ => {}
                }
            }
        };

        let handle = Handle::current();
        let _ = handle.enter();

        std::thread::spawn(move || {
            handle.block_on(peer1);
        });

        let peer_addr = format!("/ip4/127.0.0.1/tcp/61241/{:?}", peer1_id.clone());
        let peer_addr1 = peer_addr.replace("PeerId(\"", "");
        let peer_addr2 = peer_addr1.replace("\")", "");
        let peer_addr3 = peer_addr2.clone();

        // RPC Ping method test to running node
        println!(" peer to ping {} ", peer_addr2.clone());

        let handle1 = Handle::current();
        let _ = handle1.enter();

        let ping_res = tokio::task::spawn_blocking(move || {
            handle1.block_on(ReqRespRpc::p2p_ping(&ReqRespRpc, peer_addr2.clone()))
        })
        .await
        .unwrap();

        let fres = ping_res.unwrap();
        assert_eq!(true, fres.contains("Pong from PeerId"));

        // RPC Ping method test with DialFailure to dummy peer ID
        let handle2 = Handle::current();
        let _ = handle2.enter();

        let ping_res = tokio::task::spawn_blocking(move || {
            handle2.block_on(ReqRespRpc::p2p_ping(
                &ReqRespRpc,
                "/ip4/127.7.0.1/tcp/61241/QmZDuvm3dEjSgD9nq6a7d1b1kccfFjBdcHSMzCB9ULAcoH"
                    .to_string(),
            ))
        })
        .await
        .unwrap();

        let fres = ping_res.unwrap();
        assert_eq!(true, fres.contains("error: DialFailure"));

        // RPC Get method test to running node
        let handle3 = Handle::current();
        let _ = handle3.enter();

        let test_msg = "message 42".to_string();
        let get_msg = test_msg.clone();

        let ping_res = tokio::task::spawn_blocking(move || {
            handle3.block_on(ReqRespRpc::p2p_get(
                &ReqRespRpc,
                peer_addr3,
                get_msg.clone(),
            ))
        })
        .await
        .unwrap();

        let fres = ping_res.unwrap();
        assert_eq!(true, fres.contains(&test_msg));
    }
