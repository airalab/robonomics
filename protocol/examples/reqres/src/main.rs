/// Example of usage simple request response protocol from reqresp crate.
use libp2p::core::{
    identity,
    muxing::StreamMuxerBox,
    transport::{self, Transport},
    upgrade, Multiaddr, PeerId,
};
use libp2p::noise::{Keypair, NoiseConfig, X25519Spec};
use libp2p::request_response::*;
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::tcp::TcpConfig;
use libp2p::yamux::YamuxConfig;
use rust_base58::FromBase58;
use std::env;
use std::iter;
use std::process;
use std::{thread, time};

use robonomics_protocol::reqres::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    let ms = time::Duration::from_millis(100);

    //  server part

    let peer1 = async move {
        let protocols = iter::once((ReqRespProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let (peer1_id, trans) = mk_transport();
        let ping_proto1 = RequestResponse::new(ReqRespCodec(), protocols.clone(), cfg.clone());
        let mut swarm1 = Swarm::new(trans, ping_proto1, peer1_id);

        let addr_local = std::env::args().nth(1).unwrap(); // local i.e. "/ip4/192.168.1.10/tcp/61241"
        let addr: Multiaddr = addr_local.parse().unwrap();

        swarm1.listen_on(addr.clone()).unwrap();
        println!("Local peer 1 id: {:?}", peer1_id);

        loop {
            match swarm1.next_event().await {
                SwarmEvent::NewListenAddr(addr) => {
                    println!("Peer 1 listening on {}", addr.clone());
                }

                SwarmEvent::Behaviour(RequestResponseEvent::Message {
                    peer,
                    message:
                        RequestResponseMessage::Request {
                            request, channel, ..
                        },
                }) => {
                    let req = String::from_utf8_lossy(&request.0);
                    println!(" peer1 Req -> {:?} to {:?}", req.clone(), peer);

                    if req == "quit" {
                        thread::sleep(ms);
                        process::exit(0x0100);
                    }
                    let resp = Resp(format!("Hello {}", req).into_bytes());
                    println!(" peer1 Resp : {:?}", String::from_utf8_lossy(&resp.0));
                    swarm1
                        .behaviour_mut()
                        .send_response(channel, resp.clone())
                        .unwrap();
                }

                SwarmEvent::Behaviour(RequestResponseEvent::ResponseSent { peer, .. }) => {
                    println!("Response sent to peer ID {:?}", peer);
                }

                SwarmEvent::Behaviour(e) => panic!("Peer1: Unexpected event: {:?}", e),
                _ => {}
            }
        }
    };

    //  client part
    let peer2 = async move {
        let protocols = iter::once((ReqRespProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let peer_id = std::env::args().nth(2).unwrap();
        let remote_bytes = peer_id.from_base58().unwrap();
        let remote_peer = PeerId::from_bytes(&remote_bytes).unwrap();

        let (peer2_id, trans) = mk_transport();
        let ping_proto2 = RequestResponse::new(ReqRespCodec(), protocols, cfg);
        let mut swarm2 = Swarm::new(trans, ping_proto2, peer2_id.clone());
        println!("Local peer 2 id: {:?}", peer2_id);

        let addr_remote = std::env::args().nth(1).unwrap(); // remote  i.e. "/ip4/192.168.1.6/tcp/61241"
        let addr_r: Multiaddr = addr_remote.parse().unwrap();
        swarm2
            .behaviour_mut()
            .add_address(&remote_peer, addr_r.clone());

        let mut count = 0;
        let mut rq = Req(count.to_string().into_bytes());
        let mut req_id = swarm2
            .behaviour_mut()
            .send_request(&remote_peer, rq.clone());
        println!(
            " peer2 Req  ID {} -> {:?} : {}",
            req_id,
            remote_peer,
            String::from_utf8_lossy(&rq.0)
        );

        loop {
            match swarm2.next().await {
                RequestResponseEvent::Message {
                    peer,
                    message:
                        RequestResponseMessage::Response {
                            request_id,
                            response,
                        },
                } => {
                    count += 1;
                    println!(
                        " peer2 Resp ID {}  : {} to {:?}",
                        request_id,
                        String::from_utf8_lossy(&response.0),
                        peer
                    );
                    rq = Req(count.to_string().into_bytes());
                    req_id = swarm2
                        .behaviour_mut()
                        .send_request(&remote_peer, rq.clone());
                    println!(
                        " peer2 Req  ID {} -> {:?} : {}",
                        req_id,
                        remote_peer,
                        String::from_utf8_lossy(&rq.0)
                    );
                    thread::sleep(ms);
                }

                e => {
                    println!("Peer2 err: {:?}", e);
                    process::exit(0x0100)
                }
            }
        }
    };

    if args.len() < 3 {
        let _ = futures::executor::block_on(peer1);
    } else {
        let _ = futures::executor::block_on(peer2);
    }
}

fn mk_transport() -> (PeerId, transport::Boxed<(PeerId, StreamMuxerBox)>) {
    // if provided pk8 file with keys use it to have static PeerID
    // in other case PeerID  will be randomly generated
    let mut id_keys = identity::Keypair::generate_ed25519();
    let mut peer_id = id_keys.public().into_peer_id();

    let f = std::fs::read("private.pk8");
    let _ = match f {
        Ok(mut bytes) => {
            id_keys = identity::Keypair::rsa_from_pkcs8(&mut bytes).unwrap();
            peer_id = id_keys.public().into_peer_id();
            println!("try get peer ID from keypair at file");
        }
        Err(_e) => println!("try to use peer ID from random keypair"),
    };

    let noise_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&id_keys)
        .unwrap();
    (
        peer_id,
        TcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(YamuxConfig::default())
            .boxed(),
    )
}
