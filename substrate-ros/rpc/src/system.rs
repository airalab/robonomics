///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////

use msgs::{std_msgs, robonomics_msgs};

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

const BEST_HASH_ROS_NAME: &str = "blockchain/best_hash";
const BEST_NUMBER_ROS_NAME: &str = "blockchain/best_number";
const FINALIZED_HASH_ROS_NAME: &str = "blockchain/finalized_hash";
const FINALIZED_NUMBER_ROS_NAME: &str = "blockchain/finalized_number";
const NETWORK_PEERS_ROS_NAME: &str = "network/peers";

/// Robonomics node substrate RPC.
fn rpc_stream<C>(
    client: Arc<ComponentClient<C>>,
) -> impl Future<Item=(),Error=()> where
    C: Components, 
    ComponentClient<C>: ProvideRuntimeApi,
	<ComponentClient<C> as ProvideRuntimeApi>::Api: runtime_api::Metadata<ComponentBlock<C>>,

{
    let hash_pub   = rosrust::publish(BEST_HASH_ROS_NAME, QUEUE_SIZE).unwrap();
    let number_pub = rosrust::publish(BEST_NUMBER_ROS_NAME, QUEUE_SIZE).unwrap();
    let peers_pub  = rosrust::publish(NETWORK_PEERS_ROS_NAME, QUEUE_SIZE).unwrap();

    client.import_notification_stream().for_each(move |block| {
        if block.is_new_best {
            let mut hash_msg = std_msgs::String::default(); 
            hash_msg.data = block.header.hash().to_string();
            hash_pub.send(hash_msg).unwrap();

            let mut peers_msg = std_msgs::UInt64::default();
            peers_msg.data = network.peers().len() as u64;
            peers_pub.send(peers_msg).unwrap();

            let mut number_msg = std_msgs::UInt64::default();
		    number_msg.data = block.header.number().as_();
            number_pub.send(number_msg).unwrap();
        }
        Ok(())
    })
}
/*

/// Robonomics node status.
fn finality_stream<B, C, N>(
    client: Arc<C>,
    network: Arc<N>,
) -> impl Future<Item=(),Error=()> where
    C: BlockchainEvents<B> + HeaderBackend<B>,
    N: SyncProvider<B>,
    B: Block,
{
    let finalized_number_pub = rosrust::publish(FINALIZED_NUMBER_ROS_NAME, QUEUE_SIZE).unwrap();
    let finalized_hash_pub = rosrust::publish(FINALIZED_HASH_ROS_NAME, QUEUE_SIZE).unwrap();

    client.finality_notification_stream().for_each(move |block| {
        let mut finalized_number_msg = std_msgs::UInt64::default();
        finalized_number_msg.data = block.header.number().as_();
        finalized_number_pub.send(finalized_number_msg).unwrap();

        let mut finalized_hash_msg = std_msgs::String::default();
        finalized_hash_msg.data = block.header.hash().to_string();
        finalized_hash_pub.send(finalized_hash_msg).unwrap();

        Ok(())
    })
}

fn rpc_api_stream<B, C, N>(
    client_original: Arc<C>,
    network: Arc<N>,
) -> impl Future<Item=(),Error=()> + 'static where
    C: BlockchainEvents<B> + HeaderBackend<B> + BlockNumberToHash<> + 'static,
//    C: BlockchainEvents<B> + HeaderBackend<B> + ProvideRuntimeApi<> + 'static,
    N: SyncProvider<B>,
    B: Block,
//    P: ChainApi,
//    P::Block: Block<Hash=H256>,
{
    thread::spawn(move || {
        info!("Start rosrust service");

        let client = client_original.clone();
        let _best_hash_service =
            rosrust::service::<msg::robonomics_msgs::BlockHash, _>(BEST_HASH_ROS_NAME, move |_req| {
                // Callback for handling requests
                let mut res = msg::robonomics_msgs::BlockHashRes::default();
                res.hash = client.info().unwrap().best_hash.to_string();
                debug!("rosservice get best hash res {}", res.hash);
                Ok(res)
            });

        let client = client_original.clone();
        let _best_number_service =
            rosrust::service::<msg::robonomics_msgs::BlockNumber, _>(BEST_NUMBER_ROS_NAME, move |_req| {
                let mut res = msg::robonomics_msgs::BlockNumberRes::default();
                res.number = client.info().unwrap().best_number.as_();
                debug!("rosservice get best number res {}", res.number);
                Ok(res)
            });

        let client = client_original.clone();
        let _finalized_hash_service =
            rosrust::service::<msg::robonomics_msgs::BlockHash, _>(FINALIZED_HASH_ROS_NAME, move |_req| {
                let mut res = msg::robonomics_msgs::BlockHashRes::default();
                res.hash = client.info().unwrap().finalized_hash.to_string();
                debug!("rosservice get finalized hash res {}", res.hash);
                Ok(res)
            });

        let client = client_original.clone();
        let _finalized_number_service =
            rosrust::service::<msg::robonomics_msgs::BlockNumber, _>(FINALIZED_NUMBER_ROS_NAME, move |_req| {
                let mut res = msg::robonomics_msgs::BlockNumberRes::default();
                res.number = client.info().unwrap().finalized_number.as_();
                debug!("rosservice get finalized number res {}", res.number);
                Ok(res)
            });

        let client = client_original.clone();
//        let _block_number_by_hash_service =
//            rosrust::service::<msg::robonomics_msgs::BlockNumberByHash, _>("blockchain/block_number_by_hash", move |_req| {
//                let mut res = msg::robonomics_msgs::BlockNumberByHashRes::default();
//
//                //let block_id = BlockId::hash(client.info().unwrap().best_hash);
//                //let mut block_id = BlockId::hash(_req.hash);
//                //println!("block_id is {:?}", block_id);
//
//                //
//                //let block_id = client.number(client.info().unwrap().best_hash);
//                //
//
//                //let bn = _req.hash.parse();
//                let number = number::NumberOrHex(1);
//                //let block_id = client.number((H256::from_low_u64_be(1)).as_());
//                let block_id = client.number(number.to_number());
//
//                res.number = block_id.unwrap().unwrap().as_();
//
//                Ok(res)
//            } );

        let _block_hash_by_number_service =
            rosrust::service::<msg::robonomics_msgs::BlockHashByNumber, _>("blockchain/block_hash_by_number", move |_req| {
                let mut res = msg::robonomics_msgs::BlockHashByNumberRes::default();
                //res.number = client.info().unwrap().finalized_number.as_();
                //let known_block = client.header(BlockId::number(_req.number)).unwrap().take().unwrap().hash();
                //let number = _req.number;
                //Ok(match number {
                //    None => Some(client.info()?.chain.best_hash),
                //    Some(num_or_hex) => res.hash = client.header(BlockId::number(num_or_hex.to_number()?))?.map(|h| h.hash()),
                //});

                //let block_number : <B as runtime_primitives::traits::Header>::Number = 1;
                //let remote_block_id = BlockId::Number(Header::Number::decode((1 as u128).encode()));
                //let remote_block_id : <Type as runtime_primitives::traits::Header>::Number = BlockId::Number(1);
                //let rbi : <<B as runtime_primitives::traits::Block>::Header as runtime_primitives::traits::Header>::Number = BlockId::number(1);
                //let remote_block_id = client.info()?.chain.best_number;
                //let mut remote_block_header = client.header(&remote_block_id).unwrap().unwrap();
                //let remote_block_hash =
                //let block = client.block(remote_block_id).unwrap().unwrap();

                //let known_block = client.header(BlockId::number(number.to_number()?))?.map(|h| h.hash());
                //es.hash = client.block_hash_from_id(&remote_block_id).unwrap().unwrap().to_string();
                //res.hash = client.block_hash_from_id(&remote_block_id).unwrap().unwrap().to_string();
                //res.hash = known_block.to_string();

                //let blnum = client.hash(Some(0u64.into()).into());
                //let rbi : <<B as runtime_primitives::traits::Block>::Header as runtime_primitives::traits::Header>::Number = 0u64.into();
                //let rbi = NumberFor(0u64.into());

                ///////
                ////let rbi = <NumberFor<B>>::from(0u64);
                ////let blnum = client.hash(rbi);
                ////res.hash = blnum.unwrap().unwrap().to_string();
                ///////

                //res.hash = client.info().system_name().unwrap();
                //let bn: <C as runtime_primitives::traits::BlockNumberToHash>::BlockNumber = 1u64.into();
                res.hash = client.block_number_to_hash(bn).unwrap().to_string();
                debug!("rosservice get finalized number res {}", res.hash);
                Ok(res)
            });
        // Block the thread until a shutdown signal is received
        rosrust::spin();
    });
    Ok(()).into_future()
}
*/
