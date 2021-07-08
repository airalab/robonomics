/// Example of usage simple request response protocol
use bincode;
use libp2p::core::{
    identity,
    muxing::StreamMuxerBox,
    transport::{self, Transport},
    upgrade, Multiaddr, PeerId,
};
use libp2p::noise::{Keypair, NoiseConfig, X25519Spec};
use libp2p::request_response::*;
use libp2p::swarm::Swarm;
use libp2p::tcp::TcpConfig;
use rust_base58::FromBase58;
use std::iter;
use std::process;
use structopt::StructOpt;
use robonomics_protocol::reqres::*;

#[macro_use]
extern crate log;

#[derive(Debug, StructOpt)]
#[structopt(name = "reqres-cli", about = "An request-response command line client.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    debug: bool,

    // TODO timeout if needed

    /// multiaddress of server, i.e. /ip4/192.168.0.102/tcp/61241
    #[structopt(value_name = "MULTIADDR")]
    address: String,

    /// server peer ID, i.e. 12D3KooWHdqJNpszJR4na6pheUwSMNQCuGXU6sFTGDQMyQWEsszS
    #[structopt(value_name = "PEER_ID")]
    peerid: String,

    /// request type: `ping` or `get`
    #[structopt(value_name = "METHOD")]
    method: String,

    /// value: only required when `method` is `get`
    #[structopt(name = "VALUE", required_if("method", "get"))]
    value:  Option<String>,
}

    //  CLI client 
fn main() {
    env_logger::init();

    let opt = Opt::from_args();
    debug!("{:?}", opt);

    let peer2 = async move {

        let protocols = iter::once((RobonomicsProtocol(), ProtocolSupport::Full));
        let cfg = RequestResponseConfig::default();

        let peer_id = opt.peerid;
        let remote_bytes = peer_id.from_base58().unwrap();
        let remote_peer = PeerId::from_bytes(&remote_bytes).unwrap();

        let (peer2_id, trans) = mk_transport();
        let ping_proto2 = RequestResponse::new(RobonomicsCodec {is_ping: false}, protocols, cfg);
        let mut swarm2 = Swarm::new(trans, ping_proto2, peer2_id.clone());
        debug!("Local peer 2 id: {:?}", peer2_id);

        let addr_remote = opt.address;
        let addr_r : Multiaddr = addr_remote.parse().unwrap();
        swarm2.behaviour_mut().add_address(&remote_peer, addr_r.clone());

        let mut rq = Request::Ping;

        if opt.method == "ping" {
            let req_id = swarm2.behaviour_mut().send_request(&remote_peer,rq);
            debug!(" peer2 Req{}: Ping  -> {:?}", req_id, remote_peer);
        } else if opt.method == "get" {
            let value = opt.value.unwrap();
            rq = Request::Get(value.clone().into_bytes());

            if let Request::Get(y) = rq {
                debug!(" peer2  Req: Get -> {:?} : '{}'", remote_peer, String::from_utf8_lossy(&y));
            }
            let req_encoded: Vec<u8> = bincode::serialize(&format!("{}", value).into_bytes()).unwrap();
            swarm2.behaviour_mut().send_request(&remote_peer, Request::Get(req_encoded));
        } else {
            println!("unsuported command {} ", opt.method);
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
                            debug!(" peer2 Resp{} {:?} from {:?}", request_id, &response, peer);
                            println!("{:?}", &response);
                            process::exit(0);
                        },
                        Response::Data (data) => {
                            // decode response 
                            let decoded : Vec<u8> = bincode::deserialize(&data.to_vec()).unwrap();
                            debug!(" peer2 Resp: Data '{}' from {:?}", String::from_utf8_lossy(&decoded[..]), remote_peer);
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
    };

    let () = async_std::task::block_on(peer2);

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
        debug!("try get peer ID from keypair at file");
       },
        Err(_e) =>  debug!("try to use peer ID from random keypair"),
    };

    let noise_keys = Keypair::<X25519Spec>::new().into_authentic(&id_keys).unwrap();
    (peer_id, TcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(libp2p_yamux::YamuxConfig::default())
        .boxed())
}