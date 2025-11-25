// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tools for reclaiming PoV weight in parachain runtimes.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime_interface::runtime_interface;

pub const PROOF_RECORDING_DISABLED: u64 = u64::MAX;

/// Interface that provides access to the current storage proof size.
///
/// Should return the current storage proof size if [`ProofSizeExt`] is registered. Otherwise, needs
/// to return u64::MAX.
#[runtime_interface]
pub trait StorageProofSize {
    /// Returns the current storage proof size.
    fn storage_proof_size(&mut self) -> u64 {
        PROOF_RECORDING_DISABLED
    }
}
