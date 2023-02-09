use futures::StreamExt;
use libp2p::{
    core::{identity::Keypair, Multiaddr, PeerId},
    gossipsub::IdentTopic as Topic,
    swarm::{SwarmBuilder, SwarmEvent},
};
use robonomics_protocol::network::behaviour::RobonomicsNetworkBehaviour;
use std::{env::args, error::Error};
use tokio::io::{self, AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let transport = libp2p::tokio_development_transport(local_key.clone())?;
    let behaviour = RobonomicsNetworkBehaviour::new(local_key, local_peer_id, 1000, true, true)?;
    let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    if let Some(addr) = args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
    }

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let topic = Topic::new("ros");
    swarm.behaviour_mut().pubsub.subscribe(&topic)?;
    // swarm.behaviour_mut().ros.subscribe(&topic)?;

    println!("Enter messages");
    let mut stdin = BufReader::new(io::stdin()).lines();

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                let message = line?.expect("stdin closed");
                swarm.behaviour_mut().ros.publish(local_peer_id, message);
            },
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(event) => println!("new event: {:?}", event),
                _ => {}
            }
        }
    }
}
