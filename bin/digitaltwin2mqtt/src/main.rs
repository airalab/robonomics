//! To run this example, a local robonomics node should be running, i.e.:
//! ./robonomics --dev --tmp
//! ```

extern crate paho_mqtt as mqtt;

use crate::polkadot::digital_twin::events::NewDigitalTwin;
use crate::polkadot::digital_twin::events::TopicChanged;
use crate::polkadot::launch::events::NewLaunch;

use futures::StreamExt;
use sp_keyring::AccountKeyring;
use subxt::{tx::PairSigner, OnlineClient, PolkadotConfig};

use std::env;
use std::process;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tokio::runtime::Runtime;

#[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
pub mod polkadot {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx_channel, rx_channel) = mpsc::channel();

    let mqtt2robonomics = async move {
        let api = OnlineClient::<PolkadotConfig>::new()
            .await
            .expect("Robonomics node not started\n");
        println!("Started rx_channel");

        loop {
            let message_received: String = rx_channel.recv().unwrap();
            println!("Got channel message: \n{message_received}");

            let from = PairSigner::new(AccountKeyring::Bob.pair());
            let remark_tx = polkadot::tx().system().remark(message_received.into());
            let _events = api
                .tx()
                .sign_and_submit_then_watch_default(&remark_tx, &from)
                .await
                .unwrap()
                .wait_for_finalized_success()
                .await
                .unwrap();
        } // loop
    };

    let rt = Runtime::new().unwrap();
    rt.spawn(mqtt2robonomics);

    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    let mut mqtt_srv = "tcp://localhost:1883";
    let mut mqtt_sub_srv = "tcp://localhost:1883".to_string();

    let topic_pub = "digitaltwin_pub";
    let topic_sub = "digitaltwin_sub";

    if args.len() > 1 {
        mqtt_srv = &args[1];
        mqtt_sub_srv = mqtt_srv.to_string();
    }

    // Create publisher client
    let cli_pub = mqtt::AsyncClient::new(mqtt_srv).unwrap_or_else(|err| {
        println!("Error creating the client: {}", err);
        process::exit(1);
    });

    let lwt_pub = mqtt::Message::new("test", "Robonomics subscriber lost connection", 1);

    let conn_pub_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(30))
        .clean_session(true)
        .will_message(lwt_pub)
        .finalize();

    // Connect and wait for it to complete or fail
    if let Err(e) = cli_pub.connect(conn_pub_opts).wait() {
        println!("Unable to connect: {:?}", e);
        process::exit(1);
    }

    // Create mqtt subscriber client
    let _mqtt_sub = thread::spawn(move || {
        let mut cli_sub = mqtt::AsyncClient::new(mqtt_sub_srv.clone()).unwrap_or_else(|err| {
            println!("Error creating the client: {}", err);
            process::exit(1);
        });

        let rx = cli_sub.start_consuming();

        let lwt_sub = mqtt::Message::new("test", "Robonomics subscriber lost connection", 1);

        let conn_sub_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(30))
            .clean_session(true)
            .will_message(lwt_sub)
            .finalize();

        // Connect and wait for it to complete or fail
        if let Err(e) = cli_sub.connect(conn_sub_opts).wait() {
            println!("Unable to connect: {:?}", e);
            process::exit(1);
        }

        if let Err(e) = cli_sub.subscribe(topic_sub, 0).wait() {
            println!("Error subscribe topic: {:?}", e);
            process::exit(1);
        }

        println!("Started mqtt subscriber client at {}", mqtt_sub_srv.clone());

        for msg in rx.iter() {
            if let Some(message) = msg {
                println!(
                    "Get at subsriber topic '{}':\n{}",
                    message.clone().topic(),
                    String::from_utf8_lossy(message.clone().payload())
                );
                // resend received message to redirecting to blockchain channel
                let val = format!("{}", String::from_utf8_lossy(message.clone().payload()));
                tx_channel.send(val).unwrap();
            } else {
                println!("Subscriber lost connection. Attempting reconnect...");
                loop {
                    let rec = cli_sub.reconnect();
                    if let Err(e) = rec.wait() {
                        println!("Cannot to reconect at subscriber {:?}", e);
                    } else {
                        println!(
                            "Subscriber reconnected again to the server {}",
                            mqtt_sub_srv.clone()
                        );
                        if let Err(e) = cli_sub.subscribe(topic_sub, 0).wait() {
                            println!("Error subscribe topic {}: {:?}", topic_sub, e);
                            process::exit(1);
                        } else {
                            println!("Subscribed again to the topic: {}", topic_sub);
                        }
                        break;
                    }
                    thread::sleep(Duration::from_millis(1000));
                } // reconnect loop
            }
        } // rx for
    });

    thread::sleep(Duration::from_millis(1000));
    // Create robonomics client:
    let api = OnlineClient::<PolkadotConfig>::new()
        .await
        .expect("Robonomics node not started\n");

    // Subscribe to all finalized blocks:
    let mut blocks_sub = api.blocks().subscribe_finalized().await?;

    while let Some(block) = blocks_sub.next().await {
        let block = block?;

        let block_number = block.header().number;
        let block_hash = block.hash();

        println!("\nBlock #{block_number}:");
        println!("  Hash: {block_hash}");
        println!("  Extrinsics:");

        let body = block.body().await?;

        for ext in body.extrinsics().iter() {
            let ext = ext?;
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

                if is_launch {
                    let launch_event = events.find_first::<NewLaunch>()?;
                    if let Some(evt) = launch_event {
                        println!("Detected {pallet_name}::{event_name} values: \n sender: {} \n robot: {} \n params: {:?}", evt.0, evt.1, evt.2 );
                    } else {
                        println!("No launch event found in this block.");
                    }
                }

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
                        let payload = format!("{{\"sender\": \"{}\",\"id\":{},\"topic\":\"{:#x}\",\"source\":\"{}\"}}",  evt.0,  evt.1, evt.2,  evt.3);
                        println!(
                            "Publish in a topic '{}' a message:\n{}",
                            topic_pub,
                            payload.clone()
                        );
                        let msg = mqtt::Message::new(topic_pub, payload, 0);
                        let tok = cli_pub.publish(msg.clone());
                        if let Err(e) = tok.wait() {
                            println!("Error sending message: {:?}", e);
                            println!("Publisher lost connection. Attempting reconnect...");
                            loop {
                                thread::sleep(Duration::from_millis(1000));
                                let rec = cli_pub.reconnect();
                                if let Err(e) = rec.wait() {
                                    println!("Cannot to reconect at publisher: {:?}", e);
                                } else {
                                    println!(
                                        "Publisher reconnected again to the server {}",
                                        mqtt_srv.clone()
                                    );
                                    println!("Try to resend message: {}", msg.clone());
                                    cli_pub.publish(msg.clone());
                                    break;
                                }
                            } // reconnect loop
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
