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

use msgs::substrate_ros_msgs::{
    GetBlock, GetBlockRes,
    GetBlockHash, GetBlockHashRes,
    GetBlockHeader, GetBlockHeaderRes,
    GetBestHead, GetBestHeadRes,
    GetFinalizedHead, GetFinalizedHeadRes,
    BlockHash,
};
use rosrust::api::error::Error;

const BLOCK_SRV_NAME: &str          = "/chain/block";
const BLOCK_HASH_SRV_NAME: &str     = "/chain/block_hash";
const BLOCK_HEADER_SRV_NAME: &str   = "/chain/block_header";
const BEST_HEAD_SRV_NAME: &str      = "/chain/best_head";
const FINALIZED_HEAD_SRV_NAME: &str = "/chain/finalized_head";

pub type Hash = [u8; 32];

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

fn get_block<T>(
    api: T
) -> Result<rosrust::Service, Error> where
    T: ChainApi + Send + Sync + 'static
{
    rosrust::service::<GetBlock, _>(BLOCK_SRV_NAME, move |req| {
        let mut res = GetBlockRes::default();
        let block = api.block(to_hash(req.hash))?;
        res.block_json = block;
        Ok(res)
    })
}

fn get_block_hash<T>(
    api: T
) -> Result<rosrust::Service, Error> where
    T: ChainApi + Send + Sync + 'static
{
    rosrust::service::<GetBlockHash, _>(BLOCK_HASH_SRV_NAME, move |req| {
        let mut res = GetBlockHashRes::default();
        let hash = api.block_hash(Some(req.number))?;
        res.hash.data = hash;
        Ok(res)
    })
}

fn get_block_header<T>(
    api: T
) -> Result<rosrust::Service, Error> where
    T: ChainApi + Send + Sync + 'static
{
    rosrust::service::<GetBlockHeader, _>(BLOCK_HEADER_SRV_NAME, move |req| {
        let mut res = GetBlockHeaderRes::default(); 
        let header = api.header(to_hash(req.hash))?;
        res.header_json = header; 
        Ok(res)
    })
}

fn get_best_head<T>(
    api: T
) -> Result<rosrust::Service, Error> where
    T: ChainApi + Send + Sync + 'static
{
    rosrust::service::<GetBestHead, _>(BEST_HEAD_SRV_NAME, move |_| {
        let mut res = GetBestHeadRes::default();
        res.hash.data = api.block_hash(None)?;
        Ok(res)
    })
}

fn get_finalized_head<T>(
    api: T
) -> Result<rosrust::Service, Error> where
    T: ChainApi + Send + Sync + 'static
{
    rosrust::service::<GetFinalizedHead, _>(FINALIZED_HEAD_SRV_NAME, move |_| {
        let mut res = GetFinalizedHeadRes::default();
        res.hash.data = api.finalized_head();
        Ok(res)
    })
}

pub fn start_services<T>(
    api: &T
) -> Result<Vec<rosrust::Service>, Error> where 
    T: ChainApi + Clone + Sync + Send + 'static
{
    let services = vec![
        get_block(api.clone())?,
        get_block_hash(api.clone())?,
        get_block_header(api.clone())?,
        get_best_head(api.clone())?,
        get_finalized_head(api.clone())?,
    ];
    Ok(services)
}
