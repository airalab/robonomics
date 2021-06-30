/// Example of usage simple request response protocol from reqresp crate.
use bincode;
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
        let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let (peer1_id, trans) = mk_transport();
        let ping_proto1 = RequestResponse::new(RobonomicsCodec{is_ping: false}, protocols.clone(), cfg.clone());
        let mut swarm1 = Swarm::new(trans, ping_proto1, peer1_id);

        let addr_local = std::env::args().nth(1).unwrap(); // local i.e. "/ip4/192.168.1.10/tcp/61241"
        let addr: Multiaddr = addr_local.parse().unwrap();

        swarm1.listen_on(addr.clone()).unwrap();
        println!("Local peer 1 id: {:?}", peer1_id);

        loop {
            match swarm1.next_event().await {
                SwarmEvent::NewListenAddr(addr) => {
                    println!("Peer 1 listening on {}", addr.clone());
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
                            println!(" peer1 Get '{}' from  {:?}", String::from_utf8_lossy(&decoded[..]), peer);
                            let resp : Response = Response::Data(format!("Hello {}", String::from_utf8_lossy(&decoded[..])).into_bytes());
                            if let Response::Data(y) = resp.clone() {
                                println!(" peer1 Resp::Data '{}' to {:?}", String::from_utf8_lossy(&y), peer);
                            }
                            // send encoded response
                            let resp_encoded: Vec<u8> = bincode::serialize(&format!("Hello {}", String::from_utf8_lossy(&decoded[..])).into_bytes()).unwrap();
                            swarm1.behaviour_mut().send_response(channel, Response::Data(resp_encoded)).unwrap();
                        },

                        Request::Ping =>  {
                            println!(" peer1 {:?} from {:?}", request, peer);
                            let resp: Response = Response::Pong;
                            println!(" peer1 {:?} to   {:?}", resp, peer);
                            swarm1.behaviour_mut().send_response(channel, resp.clone()).unwrap();
                        },
                    }

                },

                SwarmEvent::Behaviour(RequestResponseEvent::ResponseSent {
                    peer, ..
                }) => {
                    println!("Response sent to {:?}",  peer);
                }

                SwarmEvent::Behaviour(e) =>println!("Peer1: Unexpected event: {:?}", e),
                _ => {}
            }
        }
    };

    //  client part 
    let peer2 = async move {

        let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let peer_id = std::env::args().nth(2).unwrap();
        let remote_bytes = peer_id.from_base58().unwrap(); 
        let remote_peer = PeerId::from_bytes(&remote_bytes).unwrap();

        let (peer2_id, trans) = mk_transport();
        let ping_proto2 = RequestResponse::new(RobonomicsCodec {is_ping: false}, protocols, cfg);
        let mut swarm2 = Swarm::new(trans, ping_proto2, peer2_id.clone());
        println!("Local peer 2 id: {:?}", peer2_id);
 
        let addr_remote = std::env::args().nth(1).unwrap(); // remote  i.e. "/ip4/192.168.1.6/tcp/61241"
        let addr_r : Multiaddr = addr_remote.parse().unwrap();
        swarm2.behaviour_mut().add_address(&remote_peer, addr_r.clone());

        let mut count : i64 = 0;
        let mut rq = Request::Ping;
        let mut req_id = swarm2.behaviour_mut().send_request(&remote_peer, rq.clone());
        println!(" peer2 Req{}: Ping  -> {:?} : {:?}", req_id, remote_peer, rq);

        loop {
            match swarm2.next().await {
                RequestResponseEvent::Message {
                    peer,
                    message: RequestResponseMessage::Response { request_id, response }
                } => {
                    match response {
                        Response::Pong => {
                            println!(" peer2 Resp{} {:?} from {:?}", request_id, &response, peer);
                        },
                        Response::Data (data) => {
                            // decode response 
                            let decoded : Vec<u8> = bincode::deserialize(&data.to_vec()).unwrap();
                            println!(" peer2 Resp{}: Data '{}' from {:?}", req_id, String::from_utf8_lossy(&decoded[..]), remote_peer);
                        }
                    }
                    rq = Request::Get(count.to_string().into_bytes());
                    // send encoded request 
                    if let Request::Get(y) = rq {
                        println!(" peer2  Req{}: Get -> {:?} : '{}'", req_id, remote_peer, String::from_utf8_lossy(&y));
                    }
                    let req_encoded: Vec<u8> = bincode::serialize(&format!("{}", count).into_bytes()).unwrap();
                    req_id = swarm2.behaviour_mut().send_request(&remote_peer, Request::Get(req_encoded));
                    count += 1;
                    thread::sleep(ms);
                },

                e =>  {
                    println!("Peer2 err: {:?}", e);
                    process::exit(0x0100)
                }
            }
        }
    };

    if  args.len() < 3 {
        let () = async_std::task::block_on(peer1);
    } else {
        let () = async_std::task::block_on(peer2);
    }
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
        println!("try get peer ID from keypair at file");
       },
        Err(_e) =>  println!("try to use peer ID from random keypair"),
    };

    let noise_keys = Keypair::<X25519Spec>::new().into_authentic(&id_keys).unwrap();
    (peer_id, TcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(libp2p_yamux::YamuxConfig::default())
        .boxed())
}
