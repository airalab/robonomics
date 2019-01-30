//! This module exports Robonomics API into ROS namespace.

#[macro_use]
extern crate hex_literal;
#[macro_use]
extern crate rosrust;
extern crate robonomics_runtime;
extern crate sr_io as runtime_io;
extern crate sr_primitives as runtime_primitives;
extern crate substrate_client as client;
extern crate substrate_network as network;
extern crate substrate_keystore as keystore;
extern crate substrate_primitives as primitives;
extern crate substrate_transaction_pool as transaction_pool;

use std::sync::Arc;
use primitives::ed25519;
use network::SyncProvider;
use futures::{Future, Stream};
use keystore::Store as Keystore;
use runtime_primitives::codec::{Decode, Encode};
use runtime_primitives::generic::{BlockId, Era};
use runtime_primitives::traits::{As, Block, Header, BlockNumberToHash, Extrinsic};
use client::{BlockchainEvents, BlockBody, blockchain::HeaderBackend};
use primitives::storage::{self, StorageKey, StorageData, StorageChangeSet};
use transaction_pool::txpool::{self, Pool as TransactionPool, ExtrinsicFor};
use robonomics_runtime::{
    AccountId, Call, UncheckedExtrinsic, Runtime, StorageValue,
    robonomics::*, Robonomics, RobonomicsCall, EventRecord, Event
};

use rosrust::api::Ros;

mod msg {
    rosmsg_include!(std_msgs / UInt64, std_msgs / String);
}

pub fn start_ros<A, B, C, N>(
    network: Arc<N>,
    client: Arc<C>,
    pool: Arc<TransactionPool<A>>,
    keystore: &Keystore,
    on_exit: impl Future<Item=(),Error=()>,
) -> impl Future<Item=(),Error=()> where
    A: txpool::ChainApi<Block = B> + 'static,
    B: Block + 'static,
    C: BlockchainEvents<B> + BlockBody<B> + HeaderBackend<B> + BlockNumberToHash + 'static,
    N: SyncProvider<B> + 'static
{
    let key = keystore.load(&keystore.contents().unwrap()[0], "").unwrap();
    let local_id: AccountId = key.public().0.into();
    println!("ROS account: {:?}", key.public().to_ss58check());

    let mut ros = Ros::new("robonomics").unwrap();

    let info_maker = client.clone();
    let _demand = ros.subscribe("liability/demand", move |v: msg::std_msgs::String| {
        let block = info_maker.info().unwrap().best_number;
        let payload = (
            0,
            Call::Robonomics(RobonomicsCall::demand(vec![0, 1], vec![2, 3], 42)),
            Era::immortal(),
            info_maker.genesis_hash(),
        );
        let signature = key.sign(&payload.encode());
        let extrinsic = UncheckedExtrinsic::new_signed(
            payload.0,
            payload.1,
            local_id.into(),
            signature.into(),
            payload.2
        ).encode();
        let xt: ExtrinsicFor<A> = Decode::decode(&mut &extrinsic[..]).unwrap();
        println!("result: {:?}", pool.submit_one(&BlockId::number(block), xt));
    }).unwrap();

    let _offer = ros.subscribe("liability/offer", |v: msg::std_msgs::String| {
    }).unwrap();

    let mut hash_pub = ros.publish("blockchain/best_hash").unwrap();
    let mut number_pub = ros.publish("blockchain/best_number").unwrap();
    let mut peers_pub = ros.publish("network/peers").unwrap();
    let mut liability_pub = ros.publish("liability/new").unwrap();

    let events_key = StorageKey(runtime_io::twox_128(b"System Events").to_vec());
    let storage_stream = client.storage_changes_notification_stream(Some(&[events_key])).unwrap()
        .map(|(block, changes)| StorageChangeSet { block, changes: changes.iter().cloned().collect()})
        .for_each(move |change_set| {
            let records: Vec<Vec<EventRecord<Event>>> = change_set.changes
                .iter()
                .filter_map(|(_, mbdata)| if let Some(StorageData(data)) = mbdata {
                    Decode::decode(&mut &data[..])
                } else { None })
                .collect();
            let events: Vec<Event> = records
                .concat()
                .iter()
                .cloned()
                .map(|r| r.event)
                .collect();
            //println!("changes: {:?}", events);
            events.iter().for_each(|event| {
                if let Event::robonomics(e) = event {
                    match e {
                        RawEvent::NewDemand(hash) => println!("NewDemand: {:?}", hash),
                        RawEvent::NewOffer(hash) => println!("NewOffer: {:?}", hash),
                        RawEvent::NewLiability(id, _, _) => {
                            let mut liability_msg = msg::std_msgs::UInt64::default();
                            liability_msg.data = *id;
                            liability_pub.send(liability_msg).unwrap()
                        },
                        _ => ()
                    }
                }
            });
            Ok(())
        });

    let import_stream = client.import_notification_stream().for_each(move |block| {
        if block.is_new_best {
            let mut hash_msg = msg::std_msgs::String::default(); 
            hash_msg.data = block.header.hash().to_string();
            hash_pub.send(hash_msg).unwrap();

		    let sync_status = network.status();
            let mut peers_msg = msg::std_msgs::UInt64::default();
            peers_msg.data = sync_status.num_peers as u64;
            peers_pub.send(peers_msg).unwrap();

            let mut number_msg = msg::std_msgs::UInt64::default();
		    number_msg.data = block.header.number().as_();
            number_pub.send(number_msg).unwrap();

            if let Ok(Some(xts)) = client.block_body(&BlockId::hash(block.hash)) {
                let decoded: Vec<UncheckedExtrinsic> = xts
                    .iter()
                    .map(|xt| Decode::decode(&mut &xt.encode()[..]).unwrap())
                    .collect();
                //println!("{:?}", decoded);
            }
        }
        Ok(())
    });
 
    import_stream
        .join(storage_stream)
        .map(|_| ())
        .select(on_exit)
        .then(move |_| { _demand; _offer; Ok(()) })
}
