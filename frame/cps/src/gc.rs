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
//! Background garbage collection for stale Scope `Access` state.
//!
//! See [`Cleanup Queue and Background GC`](crate#cleanup-queue-and-background-gc)
//! in the crate-level docs for the full algorithm description. This file
//! holds the GC-only types/constants and [`Pallet`]'s GC helper methods;
//! the `CleanupQueue` / `CleanupState` / `CurrentCleanup` storage items and
//! the `on_idle` hook that drives this code stay in `lib.rs` alongside the
//! rest of the pallet's storage and hooks.

use crate::{
    pallet::{Access, CleanupQueue, CleanupState, CurrentCleanup, Event},
    Config, Pallet, ScopeId, WeightInfo,
};
use frame_support::{traits::ConstU32, traits::Get, weights::Weight, BoundedVec};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// Maximum encoded length of the raw continuation cursor returned by
/// [`clear_prefix`](frame_support::storage::StorageDoubleMap::clear_prefix)
/// between bounded GC steps, stored in [`CurrentCleanup`].
///
/// Trie-derived removal cursors are small (bounded by an encoded storage
/// key length); 256 bytes is a generous upper bound.
pub const MAX_CLEANUP_CURSOR_LEN: u32 = 256;

/// [`ConstU32`] wrapper around [`MAX_CLEANUP_CURSOR_LEN`] for use as a
/// `BoundedVec` bound.
pub type MaxCleanupCursorLen = ConstU32<MAX_CLEANUP_CURSOR_LEN>;

/// Bounded continuation cursor for an in-progress `Access(scope_id, *)`
/// `clear_prefix` removal, persisted in [`CurrentCleanup`] between
/// `Pallet::on_idle` calls.
pub type CleanupCursor = BoundedVec<u8, MaxCleanupCursorLen>;

/// Hard upper bound on the number of `Access` entries a single
/// batch may remove via one `clear_prefix` call, independent of
/// the runtime-configured weight budget.
///
/// This bounds both the benchmark domain for [`WeightInfo::gc_access`] and
/// the worst-case `clear_prefix` batch, so raising it requires
/// regenerating weights rather than silently invalidating them.
pub const MAX_GC_BATCH: u32 = 64;

/// Deterministic, defensive upper bound on the number of iterations GC performs per
/// `Pallet::on_idle` call, independent of the weight budget (which already bounds real work;
/// this only guards against an unexpected zero-progress loop).
pub const MAX_GC_ITERATIONS_PER_IDLE: u32 = 32;

/// Combined head/tail cursors of [`CleanupQueue`], replacing what used to be
/// two independent `StorageValue`s so common queue operations only need a
/// single storage read/write for both cursors.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Debug,
)]
pub struct CleanupQueueState {
    /// Index of the oldest not-yet-processed entry. The queue is empty
    /// when `head == tail`.
    pub head: u64,
    /// Index one past the newest entry; the slot the next enqueue will use.
    pub tail: u64,
}

impl<T: Config> Pallet<T> {
    /// Enqueue `scope_id` (already logically invalidated - no longer
    /// reachable through any `ActiveScope` entry) for background
    /// physical cleanup, appending it to the tail of [`CleanupQueue`].
    ///
    /// Must be called atomically with the `ActiveScope` removal/
    /// replacement that invalidates `scope_id`: since `ScopeId`s are
    /// never reused, an enqueued Scope can never become active again.
    pub(crate) fn enqueue_cleanup(scope_id: ScopeId) {
        <CleanupState<T>>::mutate(|state| {
            <CleanupQueue<T>>::insert(state.tail, scope_id);
            state.tail = state.tail.saturating_add(1);
        });

        Self::deposit_event(Event::CleanupEnqueued(scope_id));
    }

    /// Run background GC for up to `remaining_weight`, performing as
    /// many bounded [`Self::do_gc_step`] batches as fit - across one or
    /// several stale Scopes - rather than a single step per call.
    ///
    /// Bounded by [`MAX_GC_ITERATIONS_PER_IDLE`] as a deterministic,
    /// defensive cap independent of the weight budget (which already
    /// bounds real work; the iteration cap only guards against
    /// unexpected zero-cost loops).
    pub(crate) fn run_gc(remaining_weight: Weight) -> Weight {
        let mut consumed = Weight::zero();
        let mut remaining = remaining_weight;

        for _ in 0..MAX_GC_ITERATIONS_PER_IDLE {
            // `progressed` - not `used.is_zero()` - is the loop's
            // "keep going" signal: a step that only performed a couple
            // of cheap reads before concluding it cannot (yet) do more
            // still reports that (non-zero) weight so it is charged to
            // the block, but must not be mistaken for "made progress,
            // try again immediately".
            let (used, progressed) = Self::do_gc_step(remaining);
            consumed = consumed.saturating_add(used);
            remaining = remaining.saturating_sub(used);
            if !progressed {
                break;
            }
        }

        consumed
    }

    /// Perform at most one bounded step of stale Scope garbage
    /// collection, consuming no more than `remaining_weight`.
    ///
    /// If [`CurrentCleanup`] already holds an in-progress Scope, its
    /// `clear_prefix` continuation is resumed; otherwise the next
    /// `ScopeId` is dequeued from [`CleanupQueue`] and a fresh
    /// continuation (`cursor = None`) is started for it. Batch size is
    /// the largest `n` (up to [`MAX_GC_BATCH`]) such that
    /// `T::WeightInfo::gc_access(n)` fits the weight left after fixed
    /// bookkeeping, found directly from the generated weight function
    /// via binary search rather than multiplying `gc_access(1)`.
    ///
    /// Every branch that mutates storage (dequeuing, parking, running
    /// `clear_prefix`) reports at least the real cost of that mutation,
    /// and never mutates anything it cannot fully account for within
    /// `remaining_weight`. Branches that only perform a couple of cheap
    /// `StorageValue` reads before concluding there is nothing to do
    /// (insufficient weight, or an empty queue) still report the exact
    /// weight of the reads that actually happened - reading storage is
    /// never "free" just because no further progress was possible this
    /// call - and signal [`Self::run_gc`] to stop iterating via the
    /// returned `bool` (`false` = no progress, do not retry this block)
    /// rather than by returning [`Weight::zero`].
    ///
    /// Returns `(consumed_weight, progressed)`, where `progressed` is
    /// `true` only when real cleanup work advanced (a Scope was
    /// dequeued/parked, or an `Access` batch was removed), so
    /// [`Self::run_gc`] knows whether retrying with the leftover weight
    /// budget is worthwhile.
    ///
    /// A `cursor` that fails to fit [`CleanupCursor`]'s bound is
    /// treated as an invariant violation - `clear_prefix` returning
    /// `Some` must never be silently treated as "finished".
    pub(crate) fn do_gc_step(remaining_weight: Weight) -> (Weight, bool) {
        let read_current = T::DbWeight::get().reads(1);
        if remaining_weight.any_lt(read_current) {
            // Not enough weight budget left to even read
            // `CurrentCleanup`; nothing was read, so there is nothing
            // to charge, and retrying at the same budget cannot help.
            return (Weight::zero(), false);
        }

        if let Some((scope_id, cursor)) = <CurrentCleanup<T>>::get() {
            // Resuming an in-progress Scope: fixed cost is the read
            // above plus the write-back that always follows (either a
            // persisted new cursor, or clearing on completion).
            let write_current = T::DbWeight::get().writes(1);
            let fixed = read_current.saturating_add(write_current);
            if remaining_weight.any_lt(fixed) {
                // Not enough weight for the mandatory write-back;
                // `CurrentCleanup` is unchanged (no write needed to
                // "persist" a value that was never modified), but the
                // read above did happen and must still be charged.
                return (read_current, false);
            }
            return Self::run_access_batch(scope_id, cursor, remaining_weight, fixed);
        }

        // No Scope is currently mid-cleanup: try to dequeue the next
        // one. Fixed cost so far: the `CurrentCleanup` read above plus
        // a `CleanupState` read.
        let peek_cost = read_current.saturating_add(T::DbWeight::get().reads(1));
        if remaining_weight.any_lt(peek_cost) {
            // Only the `CurrentCleanup` read happened before giving up.
            return (read_current, false);
        }
        let mut state = <CleanupState<T>>::get();
        if state.head >= state.tail {
            // Queue is empty; both reads above happened and are
            // charged, but there is genuinely nothing left to do.
            return (peek_cost, false);
        }

        // Dequeuing removes the `CleanupQueue` entry, advances
        // `CleanupState`, and must park the dequeued Scope into
        // `CurrentCleanup` in the same step (it is no longer in the
        // queue, so losing track of it would leak its `Access` state
        // forever) - check room for all three together before
        // mutating anything.
        let dequeue_fixed = peek_cost
            .saturating_add(T::DbWeight::get().reads(1)) // CleanupQueue read
            .saturating_add(T::DbWeight::get().writes(3)); // CleanupQueue remove + CleanupState put + CurrentCleanup park
        if remaining_weight.any_lt(dequeue_fixed) {
            // Only the two reads in `peek_cost` happened so far.
            return (peek_cost, false);
        }

        let Some(scope_id) = <CleanupQueue<T>>::get(state.head) else {
            // Defensive: a missing entry at a valid queue index should
            // never happen, but skip past it rather than getting stuck.
            // No Scope is parked, so only the queue-advance is charged.
            // The queue still advanced, so it is worth retrying.
            state.head = state.head.saturating_add(1);
            <CleanupState<T>>::put(state);
            return (
                peek_cost.saturating_add(T::DbWeight::get().reads_writes(1, 1)),
                true,
            );
        };

        <CleanupQueue<T>>::remove(state.head);
        state.head = state.head.saturating_add(1);
        <CleanupState<T>>::put(state);

        Self::run_access_batch(scope_id, None, remaining_weight, dequeue_fixed)
    }

    /// Shared tail of [`Self::do_gc_step`]'s two entry paths (resuming
    /// vs. freshly dequeued): given `fixed` (already verified by the
    /// caller to be `<= remaining_weight`, covering every mandatory
    /// bookkeeping read/write up to this point), spend whatever weight
    /// remains on a single bounded `Access(scope_id, *)` `clear_prefix`
    /// batch, then persist the resulting `CurrentCleanup` state.
    fn run_access_batch(
        scope_id: ScopeId,
        cursor: Option<CleanupCursor>,
        remaining_weight: Weight,
        fixed: Weight,
    ) -> (Weight, bool) {
        let available = remaining_weight.saturating_sub(fixed);

        // Largest batch (up to `MAX_GC_BATCH`) that
        // `T::WeightInfo::gc_access(n)` reports as fitting within
        // `available`, found by binary search over the generated
        // weight function directly - this is exact even if the
        // function is not a pure linear `base + n * per_item` formula,
        // and never multiplies `gc_access(1)` to estimate a batch.
        let mut lo: u32 = 0;
        let mut hi: u32 = MAX_GC_BATCH;
        while lo < hi {
            let mid = lo + (hi - lo + 1) / 2;
            if T::WeightInfo::gc_access(mid).all_lte(available) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let limit = lo;
        if limit == 0 {
            // Not enough weight for even one `Access` removal this
            // call; park the Scope as `CurrentCleanup` for a future
            // call to resume, charging exactly the mandatory
            // bookkeeping already committed above. No removal
            // happened, and retrying immediately at the same leftover
            // budget cannot help, so signal "no progress".
            <CurrentCleanup<T>>::put((scope_id, cursor));
            return (fixed, false);
        }

        let cursor_slice = cursor.as_ref().map(|c: &CleanupCursor| c.as_slice());
        let result = <Access<T>>::clear_prefix(scope_id, limit, cursor_slice);
        // Charge the weight of the batch that was *actually* completed
        // directly via the generated `gc_access(result.loops)` rather
        // than multiplying a single-item estimate (`gc_access(1) *
        // result.loops`): the two are only equivalent if the weight
        // function is exactly linear in the item count, which is not
        // guaranteed by the benchmark-derived formula.
        let consumed = fixed.saturating_add(T::WeightInfo::gc_access(result.loops));

        let scope_completed = result.maybe_cursor.is_none();
        match result.maybe_cursor {
            None => {
                // `clear_prefix` reports the prefix is now fully empty.
                <CurrentCleanup<T>>::kill();
                Self::deposit_event(Event::CleanupCompleted(scope_id));
            }
            Some(raw_cursor) => {
                // A returned cursor means work remains. Failing to fit
                // it into `CleanupCursor`'s bound is an invariant
                // violation (the bound is sized from the real storage
                // key layout) - it must never be mistaken for "done".
                let cursor = CleanupCursor::try_from(raw_cursor)
                    .expect("clear_prefix cursor must fit MAX_CLEANUP_CURSOR_LEN; qed");
                <CurrentCleanup<T>>::put((scope_id, Some(cursor)));
            }
        }

        // Progress happened either if entries were actually removed, or
        // the Scope's cleanup was fully completed (even if it turned
        // out to have zero remaining `Access` entries left to remove).
        let progressed = result.loops > 0 || scope_completed;
        (consumed.min(remaining_weight), progressed)
    }
}
