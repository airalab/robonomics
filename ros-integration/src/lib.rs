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
use primitives::storage::{StorageData, StorageKey};
use runtime_primitives::codec::{Decode, Encode};
use runtime_primitives::generic::{BlockId, Era};
use runtime_primitives::traits::{As, Block, Header, BlockNumberToHash};
use client::{BlockchainEvents, BlockBody, blockchain::HeaderBackend};
use transaction_pool::txpool::{self, Pool as TransactionPool};
use robonomics_runtime::{
    AccountId, Call, UncheckedExtrinsic, Runtime, StorageValue,
    robonomics, Robonomics, RobonomicsCall
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
    let key = keystore.generate("1").unwrap();
    let accountId: AccountId = key.public().0.into();
    println!("ROS account: {:?}", key.public().to_ss58check());

    let mut ros = Ros::new("robonomics").unwrap();

    let infoMaker = client.clone();
    let demand = ros.subscribe("liability/demand", move |v: msg::std_msgs::String| {
        let nonce = 0;
        let demand_call = Call::Robonomics(RobonomicsCall::demand(vec![0],vec![0],42));
        let payload = (
            nonce,
            demand_call,
            Era::mortal(256, 0),
            infoMaker.genesis_hash(),
        );
        let signature = key.sign(&payload.encode()).into();
        let extrinsic = UncheckedExtrinsic::new_signed(
            payload.0,
            payload.1,
            accountId.into(),
            signature,
            payload.2
        );
        let block = infoMaker.info().unwrap().best_number;
        let utx: <B as Block>::Extrinsic = Decode::decode(&mut extrinsic.encode().as_slice()).unwrap();
        println!("utx: {:?}", utx);
        println!("result: {:?}", pool.submit_one(&BlockId::number(block), utx));
    }).unwrap();

    let offer = ros.subscribe("liability/offer", |v: msg::std_msgs::String| {
    }).unwrap();

    let mut hash_pub = ros.publish("blockchain/best_hash").unwrap();
    let mut number_pub = ros.publish("blockchain/best_number").unwrap();
    let mut peers_pub = ros.publish("network/peers").unwrap();
    /*
    let mut liability_pub = ros.publish("liability/new").unwrap();
    */

    client.import_notification_stream().for_each(move |block| {
        if block.is_new_best {
            let mut hash_msg = msg::std_msgs::String::default(); 
            hash_msg.data = block.header.hash().to_string();
            hash_pub.send(hash_msg).unwrap();

		    let sync_status = network.status();
            let mut peers_msg = msg::std_msgs::UInt64::default();
            peers_msg.data = sync_status.num_peers as u64;
            peers_pub.send(peers_msg).unwrap();

            let mut number_msg = msg::std_msgs::UInt64::default();
		    number_msg.data = 9;
            number_pub.send(number_msg).unwrap();

            if let Ok(Some(xts)) = client.block_body(&BlockId::hash(block.hash)) {
                println!("{:?}", xts);
            }
        }
        Ok(())
    }).select(on_exit).then(move |_| {
        demand; offer; Ok(())
    })
}
