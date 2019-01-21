use std::sync::Arc;
use network::SyncProvider;
use futures::{Future, Stream};
use runtime_primitives::traits::{As, Block};
use primitives::storage::StorageKey;
use client::{BlockchainEvents, blockchain::HeaderBackend};

use rosrust::api::Ros;

mod msg {
    rosmsg_include!(std_msgs / UInt64, std_msgs / String);
}

pub fn start_ros<B, C, N>(
    network: Arc<N>,
    client: Arc<C>,
    on_exit: impl Future<Item=(),Error=()>,
) -> impl Future<Item=(),Error=()> where
    B: Block,
    C: BlockchainEvents<B> + HeaderBackend<B>,
    N: SyncProvider<B>
{
    let mut ros = Ros::new("robonomics").unwrap();

    let demand = ros.subscribe("liability/demand", |v: msg::std_msgs::String| {
    }).unwrap();

    let offer = ros.subscribe("liability/offer", |v: msg::std_msgs::String| {
    }).unwrap();

    let mut hash_pub = ros.publish("blockchain/best_hash").unwrap();
    let mut number_pub = ros.publish("blockchain/best_number").unwrap();
    let mut peers_pub = ros.publish("network/peers").unwrap();
    //let mut liability_pub = ros.publish("liability/new").unwrap();

    let events_key: StorageKey = StorageKey(b"system_events".to_vec());
    let stream = client.storage_changes_notification_stream(Some(&[events_key])).unwrap();
    stream.for_each(move |(best_hash, changes)| {
        println!("{:?}", changes);

        let mut hash_msg = msg::std_msgs::String::default(); 
        hash_msg.data = best_hash.to_string();
        hash_pub.send(hash_msg).unwrap();

		let sync_status = network.status();
        let mut peers_msg = msg::std_msgs::UInt64::default();
        peers_msg.data = sync_status.num_peers as u64;
        peers_pub.send(peers_msg).unwrap();

		if let Ok(info) = client.info() {
            let mut number_msg = msg::std_msgs::UInt64::default();
		    number_msg.data = info.best_number.as_();
            number_pub.send(number_msg).unwrap();
		}
        Ok(())
    }).select(on_exit).then(move |_| {
        demand; offer; Ok(())
    })
}
