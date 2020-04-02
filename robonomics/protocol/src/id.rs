///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life> 
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
//! Robonomics Network node identity.

use libp2p::identity::Keypair;
use libp2p::PeerId;

pub fn random() -> Keypair {
    let key = Keypair::generate_ed25519();
    let peer_id = PeerId::from(key.public());
    log::info!(target: "robonomics-identity",
               "Generated random peer id: {}", peer_id.to_base58());
    key
}
