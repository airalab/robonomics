///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2023 Robonomics Network <research@robonomics.network>
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
//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
    use robonomics_primitives::Balance;
    pub const U_MITO: Balance = 1_000_000;
    pub const MITO: Balance = 1_000_000 * U_MITO;
    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 15 * MITO / 100 + (bytes as Balance) * 6 * MITO / 100
    }
}

/// Time constants.
pub mod time {
    use robonomics_primitives::{BlockNumber, Moment};
    pub const MILLISECS_PER_BLOCK: Moment = 12000;
    pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 7 * DAYS;
    pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}
