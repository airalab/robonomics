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
///! Robonomics liability providers.
///

///! 
pub trait LiabilityProvider {
    type Liability: traits::Liability;

    ///! Get liability by it's MultiID.
    fn get(&self, id: MultiID) -> Option<Liability>;
}

type ProviderPool = Vec<LiabilityProvider>;

pub fn get_liability(pool: &ProviderPool, id: MultiID) -> impl traits::Liability {
    pool.filter_map(|p| p.get(id)).collect()
}

pub struct EthereumLiability {

}

impl EthereumLiability {
    pub fn new(address: u8[32]) {
    }
}

pub struct EthereumRPCProvider;

impl EthereumRPCProvider {
    pub fn new(public_node_uri: String) {
    }
}

impl LiabilityProvider for EthereumRPCProvider {
    type Liability = EthereumLiability; 
    fn get(&self, id: MultiID) -> Result<EthereumLiability, & {
        if id.network == 1 {
            Some(EthereumLiability::new(id.liability,
        }
    }
}
