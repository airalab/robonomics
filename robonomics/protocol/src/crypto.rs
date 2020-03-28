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
//! Robonomics node identity.

use libp2p::identity::Keypair;
use libp2p::PeerId;

pub fn random_id() -> (Keypair, PeerId) {
    let key = Keypair::generate_ed25519();
    let peer_id = PeerId::from(key.public());
    log::info!(target: "robonomics-identity",
               "Generated random id: {}", peer_id.to_base58());

    (key, peer_id)
}

#[cfg(test)]
mod tests {
    #[test]
    fn random_id_works() {
        let (k, p) = super::random_id();
        assert_eq!(k.public().into_peer_id(), p);
    }
}
