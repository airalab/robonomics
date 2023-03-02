use futures::StreamExt;
use libp2p::{
    gossipsub::{GossipsubEvent, IdentTopic as Topic},
    identity,
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId,
};
use std::{env, error::Error};
use tokio::io::{self, AsyncBufReadExt};

use robonomics_protocol::network::behaviour::{OutEvent, RobonomicsNetworkBehaviour};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id}");

    // Set up an encrypted DNS-enabled TCP Transport over the Mplex protocol
    let transport = libp2p::tokio_development_transport(local_key.clone())?;

    // Create robonomics network behaviour
    let mut behaviour = RobonomicsNetworkBehaviour::new(local_key, local_peer_id, 1000, true, true)
        .expect("Correct behaviour");

    // Create topic
    let topic = Topic::new("ROS");

    // Subscribe to topic
    behaviour.pubsub.subscribe(&topic)?;

    // Create swarm
    let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    // Add nodes
    if let Some(to_dial) = env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial(addr.clone())?;
        println!("Dialed {to_dial:?}");

        if let Some(peer) = PeerId::try_from_multiaddr(&addr) {
            swarm.behaviour_mut().pubsub.add_explicit_peer(&peer);
        }
    }

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("Enter messages via STDIN and they will be sent to connected peers using Pubsub");

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                match swarm
                    .behaviour_mut()
                    .pubsub
                    .publish(topic.clone(), line.expect("Stdin not to close").expect("").as_bytes()) {
                Ok(mid) =>
                    println!("Message : {mid:?}"),
                Err(e) =>
                    println!("Publish error: {e:?}"),
                }
                // if let Err(e) = swarm
                //     .behaviour_mut()
                //     .pubsub
                //     .publish(topic.clone(), line.expect("Stdin not to close").expect("").as_bytes()) {
                //     println!("Publish error: {e:?}");
                // }
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
                event => println!("event: {event:?}"),
            }
        }
    }
}
