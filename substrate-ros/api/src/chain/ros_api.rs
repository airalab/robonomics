///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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

use futures::future::{join, Future, FutureExt};
use futures::stream::StreamExt;
use msgs::std_msgs::UInt64;
use msgs::substrate_ros_msgs::{
    BlockHash, GetBestHead, GetBestHeadRes, GetBlock, GetBlockHash, GetBlockHashRes,
    GetBlockHeader, GetBlockHeaderRes, GetBlockRes, GetFinalizedHead, GetFinalizedHeadRes,
};
use rosrust::api::error::Error;
use sc_client_api::BlockchainEvents;
use sp_core::H256;
use sp_runtime::traits::{self, Header};

const BLOCK_SRV_NAME: &str = "/chain/block";
const BLOCK_HASH_SRV_NAME: &str = "/chain/block_hash";
const BLOCK_HEADER_SRV_NAME: &str = "/chain/block_header";
const BEST_HASH_SRV_NAME: &str = "/chain/best_hash";
const BEST_NUMBER_SRV_NAME: &str = "/chain/best_number";
const FINALIZED_HASH_SRV_NAME: &str = "/chain/finalized_hash";
const FINALIZED_NUMBER_SRV_NAME: &str = "/chain/finalized_number";

pub type Hash = [u8; 32];

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

pub trait ChainApi {
    /// Get header of a chain block.
    fn header(&self, hash: Option<Hash>) -> Result<String, String>;

    /// Get header and body of a chain block.
    fn block(&self, hash: Option<Hash>) -> Result<String, String>;

    /// Get hash of the n-th block in the canon chain.
    ///
    /// By default returns latest block hash.
    fn block_hash(&self, number: Option<u32>) -> Result<Hash, String>;

    /// Get hash of the last finalized block in the canon chain.
    fn finalized_head(&self) -> Hash;
}

fn to_hash(hash: BlockHash) -> Option<Hash> {
    if hash.data == [0; 32] {
        None
    } else {
        Some(hash.data.into())
    }
}

fn get_block<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: ChainApi + Send + Sync + 'static,
{
    rosrust::service::<GetBlock, _>(BLOCK_SRV_NAME, move |req| {
        let mut res = GetBlockRes::default();
        let block = api.block(to_hash(req.hash))?;
        res.block_json = block;
        Ok(res)
    })
}

fn get_block_hash<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: ChainApi + Send + Sync + 'static,
{
    rosrust::service::<GetBlockHash, _>(BLOCK_HASH_SRV_NAME, move |req| {
        let mut res = GetBlockHashRes::default();
        let hash = api.block_hash(Some(req.number))?;
        res.hash.data = hash;
        Ok(res)
    })
}

fn get_block_header<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: ChainApi + Send + Sync + 'static,
{
    rosrust::service::<GetBlockHeader, _>(BLOCK_HEADER_SRV_NAME, move |req| {
        let mut res = GetBlockHeaderRes::default();
        let header = api.header(to_hash(req.hash))?;
        res.header_json = header;
        Ok(res)
    })
}

fn get_best_hash<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: ChainApi + Send + Sync + 'static,
{
    rosrust::service::<GetBestHead, _>(BEST_HASH_SRV_NAME, move |_| {
        let mut res = GetBestHeadRes::default();
        res.hash.data = api.block_hash(None)?;
        Ok(res)
    })
}

fn get_finalized_hash<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: ChainApi + Send + Sync + 'static,
{
    rosrust::service::<GetFinalizedHead, _>(FINALIZED_HASH_SRV_NAME, move |_| {
        let mut res = GetFinalizedHeadRes::default();
        res.hash.data = api.finalized_head();
        Ok(res)
    })
}

fn import_notifications<T, Block>(api: T) -> Result<impl Future<Output = ()>, Error>
where
    T: ChainApi + BlockchainEvents<Block> + Clone + Sync + Send + 'static,
    Block: traits::Block<Hash = H256>,
    u64: From<<<Block as traits::Block>::Header as Header>::Number>,
{
    let hash_pub = rosrust::publish(BEST_HASH_SRV_NAME, QUEUE_SIZE)?;
    let number_pub = rosrust::publish(BEST_NUMBER_SRV_NAME, QUEUE_SIZE)?;

    let stream = api.import_notification_stream().for_each(move |block| {
        if block.is_new_best {
            let mut number_msg = UInt64::default();
            number_msg.data = (*block.header.number()).into();
            number_pub
                .send(number_msg)
                .expect("Unable to publish best number");

            let mut hash_msg = BlockHash::default();
            hash_msg.data = block.hash.into();
            hash_pub
                .send(hash_msg)
                .expect("Unable to publish best hash");
        }
        futures::future::ready(())
    });
    Ok(stream)
}

fn finality_notifications<T, Block>(api: T) -> Result<impl Future<Output = ()>, Error>
where
    T: ChainApi + BlockchainEvents<Block> + Clone + Sync + Send + 'static,
    Block: traits::Block<Hash = H256>,
    u64: From<<<Block as traits::Block>::Header as Header>::Number>,
{
    let finalized_number_pub = rosrust::publish(FINALIZED_NUMBER_SRV_NAME, QUEUE_SIZE)?;
    let finalized_hash_pub = rosrust::publish(FINALIZED_HASH_SRV_NAME, QUEUE_SIZE)?;

    let stream = api.finality_notification_stream().for_each(move |block| {
        let mut finalized_number_msg = UInt64::default();
        finalized_number_msg.data = (*block.header.number()).into();
        finalized_number_pub
            .send(finalized_number_msg)
            .expect("Unable to publish finalized number");

        let mut finalized_hash_msg = BlockHash::default();
        finalized_hash_msg.data = block.hash.into();
        finalized_hash_pub
            .send(finalized_hash_msg)
            .expect("Unable to publish finalized hash");

        futures::future::ready(())
    });
    Ok(stream)
}

pub fn start_services<T>(api: T) -> Result<Vec<rosrust::Service>, Error>
where
    T: ChainApi + Clone + Sync + Send + 'static,
{
    let services = vec![
        get_block(api.clone())?,
        get_block_hash(api.clone())?,
        get_block_header(api.clone())?,
        get_best_hash(api.clone())?,
        get_finalized_hash(api)?,
    ];
    Ok(services)
}

pub fn start_publishers<T, Block>(api: T) -> Result<impl Future<Output = ()>, Error>
where
    T: ChainApi + BlockchainEvents<Block> + Clone + Sync + Send + 'static,
    Block: traits::Block<Hash = H256>,
    u64: From<<<Block as traits::Block>::Header as Header>::Number>,
{
    let task = join(
        import_notifications(api.clone())?,
        finality_notifications(api)?,
    )
    .map(|_| ());
    Ok(task)
}
