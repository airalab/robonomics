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
//! SubXt compatible robonomics-datalog pallet abstration.

use codec::{EncodeLike, Codec};
use sp_runtime::traits::Member;
use substrate_subxt::{system, Call};

/// The subset of the `pallet_robonomics_datalog::Trait` that a client must implement.
pub trait Datalog: system::System {
    type Record: Codec + EncodeLike + Member;
}

const MODULE: &str = "Datalog";
const RECORD: &str = "record";

/// Arguments for datalog record call. 
#[derive(codec::Encode)]
pub struct RecordArgs<T: Datalog> {
    record: <T as Datalog>::Record
}

pub fn record<T: Datalog>(
    record: <T as Datalog>::Record,
) -> Call<RecordArgs<T>> {
    Call::new(MODULE, RECORD, RecordArgs { record })
}
