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
//! Resource authorization and limit lookup interfaces.

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::DispatchError;

/// Authorization context in which an actor wants to use a resource.
///
/// The authorization provider decides what a context means; consumers only
/// pass it through.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub enum AuthorizationContext<AccountId, NodeId> {
    /// Direct use of the subscription owned by the account.
    Account(AccountId),
    /// Delegated use authorized against a canonical node identifier.
    Node(NodeId),
    /// Delegated use where a node is addressed through its account identity.
    ///
    /// The identity is an authorization address only: the provider resolves it
    /// to the canonical node, and it must not become the local accounting
    /// identifier. Changing a node identity must not reset local accounting.
    NodeIdentity(AccountId),
}

/// Result of a successful authorization.
#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
)]
pub struct ResourceContext<AccountId, LocalId> {
    /// Owner of the global subscription allocation.
    pub owner: AccountId,
    /// Local accounting context.
    ///
    /// - `None`: global (direct) accounting only.
    /// - `Some(..)`: both global and local accounting are required.
    pub local_id: Option<LocalId>,
}

/// Answers: may this actor use this resource in this context?
///
/// The trait deliberately exposes no scope traversal, node hierarchy,
/// identity lookup, or access storage; those are provider internals.
///
/// `ContextId` is the authorization context (for a CPS-backed provider, a
/// `NodeId`), while `LocalId` is the local accounting and limit context (for
/// a CPS-backed provider, a `ScopeId`). They must not be used
/// interchangeably.
pub trait ResourceAuthorization<AccountId> {
    /// Authorization context identifier.
    type ContextId;
    /// Local accounting and resource-limit context identifier.
    type LocalId;
    /// Resource identifier.
    type ResourceId;

    /// Authorize `actor` to use `resource` in `context`.
    ///
    /// Returns the subscription owner and the optional local accounting
    /// context, or an error if the actor is not authorized.
    fn authorize(
        actor: &AccountId,
        context: &Self::ContextId,
        resource: Self::ResourceId,
    ) -> Result<ResourceContext<AccountId, Self::LocalId>, DispatchError>;
}

/// Answers: what limit applies to this resource in this local accounting
/// context?
///
/// This is a read-only policy interface, separate from authorization. The
/// `Limit` type is resource-specific: for Transaction it is a rate, other
/// resources may use a different policy quantity.
pub trait ResourceLimit {
    /// Local accounting and resource-limit context identifier.
    type LocalId;
    /// Resource identifier.
    type ResourceId;
    /// Resource-specific limit.
    type Limit;

    /// Limit of `resource` in the `local` context, or `None` if none is set.
    fn limit(local: &Self::LocalId, resource: Self::ResourceId) -> Option<Self::Limit>;
}
