/// Example of usage simple request response protocol from reqresp crate.
use bincode;
use chrono::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use libp2p::core::Multiaddr;
use libp2p::request_response::*;
use libp2p::swarm::{Swarm, SwarmEvent};
use std::fmt;
use std::iter;
use structopt::StructOpt;

use robonomics_protocol::reqres::*;

#[macro_use]
extern crate log;

#[derive(Debug, StructOpt)]
#[structopt(name = "reqres-cli-srv", about = "An request-response command line server.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    debug: bool,

    /// multiaddress of server, i.e. /ip4/192.168.0.102/tcp/61241
    #[structopt(value_name = "MULTIADDR")]
    address: String,
}


fn main() {
    env_logger::init();
    let opt = Opt::from_args();
    debug!("{:?}", opt);

    let peer1 = async move {
        let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let (peer1_id, trans) = mk_transport();
        let ping_proto1 = RequestResponse::new(RobonomicsCodec{is_ping: false}, protocols.clone(), cfg.clone());
        let mut swarm1 = Swarm::new(trans, ping_proto1, peer1_id);

        let addr_local = opt.address;
        let addr: Multiaddr = addr_local.parse().unwrap();

        swarm1.listen_on(addr.clone()).unwrap();
        let mut peer_id = String::new();
        fmt::write (&mut peer_id, format_args!("{:?}", peer1_id)).unwrap();
        debug!("Local peer 1 id: {}", peer_id.clone());
        let mut file = File::create("peerid.txt").unwrap();
        file.write_all(peer_id.as_bytes()).expect("Unable to write data");

        loop {
            match swarm1.next_event().await {
                SwarmEvent::NewListenAddr(addr) => {
                    debug!("Peer 1 listening on {}", addr.clone());
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
                            debug!(" peer1 Get '{}' from  {:?}", String::from_utf8_lossy(&decoded[..]), peer);

                            // send encoded response
                            //let resp_encoded: Vec<u8> = bincode::serialize(&format!("Hello {}", String::from_utf8_lossy(&decoded[..])).into_bytes()).unwrap();
                            let resp_encoded: Vec<u8> = bincode::serialize(&format!("{}", epoch()).into_bytes()).unwrap();
                            swarm1.behaviour_mut().send_response(channel, Response::Data(resp_encoded)).unwrap();
                        },

                        Request::Ping =>  {
                            debug!(" peer1 {:?} from {:?}", request, peer);
                            let resp: Response = Response::Pong;
                            debug!(" peer1 {:?} to   {:?}", resp, peer);
                            swarm1.behaviour_mut().send_response(channel, resp.clone()).unwrap();
                        },
                    }

                },

                SwarmEvent::Behaviour(RequestResponseEvent::ResponseSent {
                    peer, ..
                }) => {
                    debug!("Response sent to {:?}",  peer);
                }

                SwarmEvent::Behaviour(e) =>println!("Peer1: Unexpected event: {:?}", e),
                _ => {}
            }
        }
    };

    let () = async_std::task::block_on(peer1);
}

fn epoch () -> i64 {
   let ts =  Utc::now();
   ts.timestamp() * 1000 + ( ts.nanosecond() as i64 )/ 1000 / 1000
}
