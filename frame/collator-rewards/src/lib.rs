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
//! # Robonomics Collator Rewards
//!
//! A small, runtime-agnostic helper that mints a fixed per-block reward
//! directly to the block author (collator) every time the runtime's
//! `pallet_authorship` notifies a new author.
//!
//! See `README.md` for the rationale, formula, and wiring example.
//!
//! ## Why a dedicated crate?
//!
//! The reward logic is independent of any specific Robonomics runtime: it
//! only needs an `AccountId`, a fungible currency, and a constant reward
//! amount. Keeping it here keeps the runtime crate small and lets us test
//! the behaviour in isolation against a minimal mock runtime.
//!
//! ## Why mint directly to the author?
//!
//! `pallet_collator_selection::note_author` only forwards **half** of the
//! pot to the current author — the other half stays in the pot. Routing
//! the reward via the pot would therefore silently halve the per-block
//! reward seen by the collator. Minting directly to the author guarantees
//! the author receives exactly `Reward::get()` per authored block.
//!
//! Consequently, `AuthorRewards` MUST be ordered **before**
//! `CollatorSelection` in the `pallet_authorship::Config::EventHandler`
//! tuple so the author is paid regardless of the pot's state.

#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;

use frame_support::{
    defensive,
    dispatch::DispatchClass,
    traits::{
        fungible::{Inspect, Mutate},
        Get,
    },
    weights::Weight,
};
use frame_system::pallet_prelude::BlockNumberFor;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Balance type associated with `Currency` for accounts of `T`.
pub type BalanceOf<T, Currency> =
    <Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// `pallet_authorship::EventHandler` that mints a fixed `Reward` to the
/// block author each block.
///
/// Generic over:
/// - `T`: a `frame_system::Config`-bearing runtime (provides `AccountId`,
///   `BlockNumber`, and `DbWeight`).
/// - `Reward`: a `Get` returning the per-block reward amount in the
///   smallest unit of `Currency`.
/// - `Currency`: the fungible used to mint the reward (e.g.
///   `pallet_balances::Pallet<Runtime>`).
///
/// Wire it ahead of `pallet_collator_selection` in the `EventHandler`
/// tuple so that the author is paid regardless of the pot's state — see
/// the crate-level docs for the rationale.
pub struct AuthorRewards<T, Reward, Currency>(PhantomData<(T, Reward, Currency)>);

impl<T, Reward, Currency> pallet_authorship::EventHandler<T::AccountId, BlockNumberFor<T>>
    for AuthorRewards<T, Reward, Currency>
where
    T: frame_system::Config,
    Reward: Get<BalanceOf<T, Currency>>,
    Currency: Mutate<T::AccountId>,
    BalanceOf<T, Currency>: Copy,
{
    fn note_author(author: T::AccountId) {
        // `note_author` runs from `on_initialize`. Account for the extra
        // storage mutation (one read + one write on `System::Account`)
        // performed by `mint_into` as mandatory weight so the block weight
        // limits are respected. The `from_parts` overhead matches the
        // benchmarked `pallet_balances::force_set_balance_creating`
        // weight, which performs the same `System::Account` access
        // pattern.
        let mint_weight = T::DbWeight::get()
            .reads_writes(1, 1)
            .saturating_add(Weight::from_parts(25_000_000, 3593));
        frame_system::Pallet::<T>::register_extra_weight_unchecked(
            mint_weight,
            DispatchClass::Mandatory,
        );

        let reward = Reward::get();

        // Mint the per-block reward directly to the author.
        //
        // Failure should be impossible in normal operation: a sensible
        // `Reward` is well above the existential deposit and the total
        // issuance cannot overflow `Balance` (u128) within any realistic
        // chain lifetime. We therefore log defensively rather than panic,
        // to avoid bricking block production on an unexpected runtime
        // invariant violation.
        if let Err(e) = Currency::mint_into(&author, reward) {
            defensive!(
                "failed to mint collator block reward to author",
                (&author, reward, e)
            );
        }
    }
}
