///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
use frame_support::weights::Weight;

pub trait WeightInfo {
    fn record() -> Weight;
    fn erase(win: u64) -> Weight;
}

impl WeightInfo for () {
    fn record() -> Weight {
        Default::default()
    }

    fn erase(_win: u64) -> Weight {
        Default::default()
    }
}
