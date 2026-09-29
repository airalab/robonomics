///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
//! # CPS Runtime API
//!
//! Runtime API of `pallet-robonomics-cps` for off-chain clients.
//!
//! [`CpsApi`] provides two read-only queries that run the pallet's own
//! functions, so clients do not need to reimplement Scope resolution or
//! authorization:
//!
//! - `resolve_scope` returns the result of
//!   [`Pallet::resolve_scope`](pallet_robonomics_cps::Pallet::resolve_scope);
//! - `has_capability` returns the result of
//!   [`Pallet::has_capability`](pallet_robonomics_cps::Pallet::has_capability).
//!
//! Clients call the API through the `state_call` RPC (e.g. Subxt runtime
//! API calls or polkadot.js `api.call.cpsApi`), at any block.
//!
//! ## Runtime implementation
//!
//! ```ignore
//! impl pallet_robonomics_cps_runtime_api::CpsApi<Block, AccountId> for Runtime {
//!     fn resolve_scope(node: NodeId) -> Option<ResolvedScope<AccountId>> {
//!         Cps::resolve_scope(node).ok()
//!     }
//!
//!     fn has_capability(node_id: NodeId, account_id: AccountId, capability: Capability) -> bool {
//!         Cps::has_capability(node_id, &account_id, capability)
//!     }
//! }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]

use pallet_robonomics_cps::{Capability, NodeId, ResolvedScope};
use parity_scale_codec::Codec;

sp_api::decl_runtime_apis! {
    /// Read-only CPS queries: Scope resolution and capability checks.
    pub trait CpsApi<AccountId> where
        AccountId: Codec
    {
        /// Resolve the Scope that `node` belongs to.
        ///
        /// Returns the Scope's id, root node, and owner, and the path from
        /// `node` to the Scope root (see [`ResolvedScope`]), or `None` if
        /// `node` does not exist or no Scope can be resolved for it.
        fn resolve_scope(node: NodeId) -> Option<ResolvedScope<AccountId>>;

        /// Whether `account_id` may use `capability` at `node_id`, with the
        /// same checks as the pallet calls that require it: ownership of the
        /// node's Scope or Access entries, and for
        /// [`Capability::CreateScope`] also ownership of the immediately
        /// enclosing Scope when `node_id` is a nested Scope root.
        ///
        /// Returns `false` if `node_id` does not exist or no Scope can be
        /// resolved for it.
        fn has_capability(node_id: NodeId, account_id: AccountId, capability: Capability) -> bool;
    }
}
