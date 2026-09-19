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
//! Runtime API definition exposing canonical CPS Scope resolution and
//! capability checks to off-chain clients (e.g. Subxt-based tooling such as
//! `libcps`).
//!
//! This crate must be imported and implemented by the runtime of a node that
//! wants clients to resolve the active `ScopeId` of a CPS node, or check
//! whether an account holds a given `Capability` at a node, without
//! reimplementing Scope-inheritance / `Access`-traversal logic client-side.
//!
//! Both methods are thin, read-only wrappers over the pallet's canonical
//! logic: [`pallet_robonomics_cps::Pallet::resolve_scope`] (the single
//! Scope resolver used by every authorization check) and
//! [`pallet_robonomics_cps::Pallet::has_capability`] (which itself dispatches
//! to the same `authorize` / `authorize_create_scope` helpers used by the
//! pallet's dispatchables). They are callable through the standard generic
//! runtime-call mechanism (e.g. Substrate's `state_call` RPC, as used by
//! Subxt and `polkadot-omni-node`), so no custom Robonomics JSON-RPC endpoint
//! or node-side customization is required.
//!
//! Only the `ScopeId` is returned by `resolve_scope`: the Scope's root and
//! owner are ordinary `ScopeRoot` / `ScopeOwner` storage entries that a
//! client can already read directly once it knows the `ScopeId`.
//!
//! Because these are normal Runtime API methods, they are automatically
//! included in runtime metadata and can be queried at any historical block
//! height through the standard Subxt block API, which is important since a
//! node's resolved Scope and granted capabilities may change over time.
#![cfg_attr(not(feature = "std"), no_std)]

use pallet_robonomics_cps::{Capability, NodeId, ScopeId};
use parity_scale_codec::{Codec, Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;

/// Stable numeric identifier for a [`Capability`].
///
/// [`Capability`]'s SCALE representation is derived from its declaration
/// order, so inserting a new variant anywhere but the end (or reordering
/// existing ones) would silently change the encoding of every existing
/// `Access` entry and of the `CpsApi::has_capability` runtime API. Callers
/// of that Runtime API - most importantly off-chain clients that cannot be
/// upgraded in lockstep with the runtime - should instead use this
/// explicit, hand-assigned identifier, which only ever grows by appending a
/// new mapping and never reassigns an existing one.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
)]
pub struct CapabilityId(#[codec(compact)] pub u32);

impl From<Capability> for CapabilityId {
    fn from(capability: Capability) -> Self {
        match capability {
            Capability::CreateScope => CapabilityId(0),
            Capability::Write => CapabilityId(1),
        }
    }
}

impl TryFrom<CapabilityId> for Capability {
    type Error = ();

    /// Fails for any `CapabilityId` not backed by a known `Capability`
    /// variant - e.g. an ID introduced by a newer runtime that an older
    /// client is still linked against.
    fn try_from(id: CapabilityId) -> Result<Self, Self::Error> {
        match id.0 {
            0 => Ok(Capability::CreateScope),
            1 => Ok(Capability::Write),
            _ => Err(()),
        }
    }
}

sp_api::decl_runtime_apis! {
    /// The API to resolve canonical CPS Scope and query CPS capabilities.
    pub trait CpsApi<AccountId> where
        AccountId: Codec,
    {
        /// Resolve the `ScopeId` of the Scope currently active for `node`.
        ///
        /// This is the `ScopeId` of `node` itself if it carries an
        /// `ActiveScope` entry, or of the nearest ancestor that does.
        ///
        /// Returns `None` if `node` does not exist or if no active Scope
        /// could be found while walking its ancestry (this should not
        /// normally happen, since every valid tree has a root Scope, but a
        /// malformed/incomplete tree state is represented as `None` rather
        /// than trapping the call).
        fn resolve_scope(node: NodeId) -> Option<ScopeId>;

        /// Check whether `account_id` currently holds `capability_id` at
        /// `node_id`.
        ///
        /// `capability_id` is a stable [`CapabilityId`], decoupled from
        /// `Capability`'s SCALE representation, so this method's signature
        /// does not need to change as new capabilities are introduced.
        /// Unrecognized IDs are reported as `false` rather than trapping
        /// the call.
        ///
        /// Reuses the same canonical authorization logic enforced by the
        /// pallet's dispatchables (Scope owner implicit authority, exact /
        /// inherited `Access` never crossing a nested Scope boundary).
        /// Returns `false` (rather than trapping the call) if `node_id`
        /// does not exist or no Scope can be resolved for it.
        fn has_capability(node_id: NodeId, account_id: AccountId, capability_id: CapabilityId) -> bool;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_id_round_trips_for_every_variant() {
        for capability in [Capability::Write, Capability::CreateScope] {
            let id = CapabilityId::from(capability);
            assert_eq!(Capability::try_from(id), Ok(capability));
        }
    }

    #[test]
    fn capability_id_assignments_are_stable() {
        // `CapabilityId` values are part of the runtime API's stable wire
        // format: once assigned, an ID must never be reassigned to a
        // different `Capability`, even if `Capability`'s own declaration
        // order changes.
        assert_eq!(CapabilityId::from(Capability::CreateScope).0, 0);
        assert_eq!(CapabilityId::from(Capability::Write).0, 1);
    }

    #[test]
    fn capability_id_rejects_unknown_ids() {
        assert_eq!(Capability::try_from(CapabilityId(42)), Err(()));
    }
}
