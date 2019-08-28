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
///! Network tagged Robonomics liabilities.
/// 
/// To make clear what network own this liability lets concat network identifier
/// with liability identifier. Encode it into Base58 when string representation needed.
///

#[derive(Serialize, Deserialize, Debug)]
pub struct MultiID {
    u32 network;
    u8[] id;
}

impl ToString for MultiID {

}
