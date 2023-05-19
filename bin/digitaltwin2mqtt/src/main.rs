//! To run this example, a local robonomics node should be running, i.e.:
//! ./robonomics --dev --tmp
//! ```

extern crate paho_mqtt as mqtt;

use crate::polkadot::digital_twin::events::NewDigitalTwin;
use crate::polkadot::digital_twin::events::TopicChanged;

use futures::StreamExt;
use subxt::{OnlineClient, PolkadotConfig};

use std::env;
use std::process;

#[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
pub mod polkadot {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    let mut mqtt_srv = "tcp://localhost:1883";

    if args.len() > 1 {
        mqtt_srv = &args[1];
    }

    // Create a client & define connect options
    let cli = mqtt::AsyncClient::new(mqtt_srv).unwrap_or_else(|err| {
        println!("Error creating the client: {}", err);
        process::exit(1);
    });

    let conn_opts = mqtt::ConnectOptions::new();

    // Connect and wait for it to complete or fail
    if let Err(e) = cli.connect(conn_opts).wait() {
        println!("Unable to connect: {:?}", e);
        process::exit(1);
    }

    // Create a client to use:
    let api = OnlineClient::<PolkadotConfig>::new()
        .await
        .expect("Robonomics node not started\n");

    // Subscribe to all finalized blocks:
    let mut blocks_sub = api.blocks().subscribe_finalized().await?;

    while let Some(block) = blocks_sub.next().await {
        let block = block?;

        let block_number = block.header().number;
        let block_hash = block.hash();

        println!("Block #{block_number}:");
        println!("  Hash: {block_hash}");
        println!("  Extrinsics:");

        let body = block.body().await?;
        for ext in body.extrinsics() {
            let idx = ext.index();
            let events = ext.events().await?;
            let bytes_hex = format!("0x{}", hex::encode(ext.bytes()));

            println!("    Extrinsic #{idx}:");
            println!("      Bytes: {bytes_hex}");

            for evt in events.iter() {
                let evt = evt?;

                let pallet_name = evt.pallet_name();
                let event_name = evt.variant_name();
                let is_launch = evt.as_event::<NewLaunch>()?.is_some();
                let is_new_twin = evt.as_event::<NewDigitalTwin>()?.is_some();
                let is_twin = evt.as_event::<TopicChanged>()?.is_some();

                if is_new_twin {
                    let new_twin_event = events.find_first::<NewDigitalTwin>()?;
                    if let Some(evt) = new_twin_event {
                        println!(
                            "Detected {pallet_name}::{event_name} values: \n sender: {} \n id: {}",
                            evt.0, evt.1
                        );
                    } else {
                        println!("No new digital twin event found in this block.");
                    }
                }

                if is_twin {
                    let twin_event = events.find_first::<TopicChanged>()?;
                    if let Some(evt) = twin_event {
                        println!("Detected {pallet_name}::{event_name} with values: \n sender: {} \n id: {} \n topic: {:?} \n source: {}", evt.0, evt.1, evt.2, evt.3);
                        println!("Publishing a message on the 'digitaltwin' topic");
                        let payload = format!("{{\"sender\": \"{}\",\"id\":{},\"topic\":\"{:#x}\",\"source\":\"{}\"}}",  evt.0,  evt.1, evt.2,  evt.3);
                        let msg = mqtt::Message::new("digitaltwin", payload, 0);
                        let tok = cli.publish(msg);
                        if let Err(e) = tok.wait() {
                            println!("Error sending message: {:?}", e);
                        }
                    } else {
                        println!("No digital twin event found in this block.");
                    }
                }
            }
        }
    }
    Ok(())
}
