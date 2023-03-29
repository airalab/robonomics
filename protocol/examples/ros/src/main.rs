use futures::StreamExt;
use libp2p::{
    gossipsub::{GossipsubEvent, IdentTopic as Topic},
    identity,
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId,
};
use std::{env, error::Error, time::Duration};
use tokio::io::{self, AsyncBufReadExt};

use robonomics_protocol::network::behaviour::{OutEvent, RobonomicsNetworkBehaviour};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id}");

    // Set params
    let heartbeat_interval = Duration::from_millis(1000);
    let disable_pubsub = false;
    let disable_mdns = true;
    let disable_kad = true;
    let network_listen_address = "/ip4/0.0.0.0/tcp/0"
        .parse()
        .expect("network listen address");

    // Set up an encrypted DNS-enabled TCP Transport over the Mplex protocol
    let transport = libp2p::tokio_development_transport(local_key.clone())?;

    // Create robonomics network behaviour
    let behaviour = RobonomicsNetworkBehaviour::new(
        local_key,
        local_peer_id,
        heartbeat_interval,
        disable_pubsub,
        disable_mdns,
        disable_kad,
    )
    .expect("Correct behaviour");

    // Create swarm
    let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    // Create topic
    let topic = Topic::new("ROS");

    // Subscribe to topic
    if let Some(pubsub) = swarm.behaviour_mut().pubsub.as_mut() {
        pubsub.subscribe(&topic)?;
    }

    // Add nodes
    if let Some(to_dial) = env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial(addr.clone())?;

        // Add node to pubsub swarm
        if let Some(pubsub) = swarm.behaviour_mut().pubsub.as_mut() {
            if let Some(peer) = PeerId::try_from_multiaddr(&addr) {
                pubsub.add_explicit_peer(&peer);
            }
        }
    }

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on(network_listen_address)?;

    println!("Enter messages via STDIN and they will be sent to connected peers using Pubsub");

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                if let Some(pubsub) = swarm.behaviour_mut().pubsub.as_mut() {
                    pubsub.publish(topic.clone(), line.expect("Stdin not to close").expect("").as_bytes())?;
                }
            },
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(OutEvent::Pubsub(GossipsubEvent::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => println!(
                        "Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    ),
                _ => {},
            }
        }
    }
}
