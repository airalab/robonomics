///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! Staking approach for a parachain bandwidth distribution.

// This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::traits::{Currency, LockIdentifier, LockableCurrency, WithdrawReasons};
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, Saturating, StaticLookup, Zero},
    RuntimeDebug,
};

//pub mod weights;

pub use pallet::*;
//pub use weights::WeightInfo;

const STAKING_ID: LockIdentifier = *b"rbcstake";
const MAX_UNLOCKING_CHUNKS: usize = 32;

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct UnlockChunk<Balance: HasCompact, Moment: HasCompact> {
    /// Amount of funds to be unlocked.
    #[codec(compact)]
    value: Balance,
    /// Time moment at which point it'll be unlocked.
    #[codec(compact)]
    moment: Moment,
}

/// The ledger of a (bonded) stash.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct StakerLedger<AccountId, Balance: HasCompact, Moment: HasCompact> {
    /// The stash account whose balance is actually locked and at stake.
    pub stash: AccountId,
    /// The total amount of the stash's balance that we are currently accounting for.
    /// It's just `active` plus all the `unlocking` balances.
    #[codec(compact)]
    pub total: Balance,
    /// The total amount of the stash's balance that will be at stake in any forthcoming
    /// rounds.
    #[codec(compact)]
    pub active: Balance,
    /// Any balance that is becoming free, which may eventually be transferred out of the stash.
    pub unlocking: Vec<UnlockChunk<Balance, Moment>>,
    /// Time moment of rewards claimed last time.
    pub claimed_rewards: Moment,
}

impl<
        AccountId,
        Balance: HasCompact + Copy + Saturating + AtLeast32BitUnsigned,
        Moment: HasCompact + PartialOrd,
    > StakerLedger<AccountId, Balance, Moment>
{
    /// Remove entries from `unlocking` that are sufficiently old and reduce the
    /// total by the sum of their balances.
    fn consolidate_unlocked(self, current_moment: Moment) -> Self {
        let mut total = self.total;
        let unlocking = self
            .unlocking
            .into_iter()
            .filter(|chunk| {
                if chunk.moment > current_moment {
                    true
                } else {
                    total = total.saturating_sub(chunk.value);
                    false
                }
            })
            .collect();

        Self {
            stash: self.stash,
            total,
            active: self.active,
            unlocking,
            claimed_rewards: self.claimed_rewards,
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The staking balance.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        // Weight information for extrinsics in this pallet.
        //type WeightInfo: WeightInfo;

        /// Time duration that staked funds must remain bonded for.
        #[pallet::constant]
        type BondingDuration: Get<Self::BlockNumber>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Not a controller account.
        NotController,
        /// Not a stash account.
        NotStash,
        /// Stash is already bonded.
        AlreadyBonded,
        /// Controller is already paired.
        AlreadyPaired,
        /// Internal state has become somehow corrupted and the operation cannot continue.
        BadState,
        /// Can not bond with value less than minimum balance.
        InsufficientValue,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {
        /// An account has bonded this amount. \[stash, amount\]
        ///
        /// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
        /// it will not be emitted for staking rewards when they are added to stake.
        Bonded(T::AccountId, BalanceOf<T>),
        /// An account has unbonded this amount. \[stash, amount\]
        Unbonded(T::AccountId, BalanceOf<T>),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue. \[stash, amount\]
        Withdrawn(T::AccountId, BalanceOf<T>),
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn bonded)]
    /// Map from all locked "stash" accounts to the controller account.
    pub(super) type Bonded<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn ledger)]
    /// Map from all (unlocked) "controller" accounts to the info regarding the staking.
    pub(super) type Ledger<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        StakerLedger<T::AccountId, BalanceOf<T>, T::BlockNumber>,
    >;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Take the origin account as a stash and lock up `value` of its balance. `controller` will
        /// be the account that controls it.
        ///
        /// `value` must be more than the `minimum_balance` specified by `T::Currency`.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash account.
        ///
        /// Emits `Bonded`.
        ///
        /// # <weight>
        /// - Independent of the arguments. Moderate complexity.
        /// - O(1).
        /// - Three extra DB entries.
        ///
        /// NOTE: Two of the storage writes (`Self::bonded`) are _never_ cleaned
        /// unless the `origin` falls below _existential deposit_ and gets removed as dust.
        /// ------------------
        /// Weight: O(1)
        /// DB Weight:
        /// - Read: Bonded, Ledger, [Origin Account], Locks
        /// - Write: Bonded, [Origin Account], Locks, Ledger
        /// # </weight>
        #[pallet::weight(100_000)]
        pub fn bond(
            origin: OriginFor<T>,
            controller: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let stash = ensure_signed(origin)?;
            if <Bonded<T>>::contains_key(&stash) {
                Err(Error::<T>::AlreadyBonded)?
            }

            let controller = T::Lookup::lookup(controller)?;
            if <Ledger<T>>::contains_key(&controller) {
                Err(Error::<T>::AlreadyPaired)?
            }

            <frame_system::Pallet<T>>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;
            <Bonded<T>>::insert(&stash, &controller);

            let stash_balance = T::Currency::free_balance(&stash);
            let value = value.min(stash_balance);

            Self::deposit_event(Event::Bonded(stash.clone(), value));
            let block_number = <frame_system::Pallet<T>>::block_number();
            let item = StakerLedger {
                stash,
                total: value,
                active: value,
                unlocking: vec![],
                claimed_rewards: block_number,
            };
            Self::update_ledger(&controller, &item);
            Ok(().into())
        }

        /// Schedule a portion of the stash to be unlocked ready for transfer out after the bond
        /// period ends. If this leaves an amount actively bonded less than
        /// T::Currency::minimum_balance(), then it is increased to the full amount.
        ///
        /// Once the unlock period is done, you can call `withdraw_unbonded` to actually move
        /// the funds out of management ready for transfer.
        ///
        /// No more than a limited number of unlocking chunks (see `MAX_UNLOCKING_CHUNKS`)
        /// can co-exists at the same time. In that case, [`Call::withdraw_unbonded`] need
        /// to be called first to remove some of the chunks (if possible).
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// Emits `Unbonded`.
        ///
        /// See also [`Call::withdraw_unbonded`].
        ///
        /// # <weight>
        /// - Independent of the arguments. Limited but potentially exploitable complexity.
        /// - Contains a limited number of reads.
        /// - Each call (requires the remainder of the bonded balance to be above `minimum_balance`)
        ///   will cause a new entry to be inserted into a vector (`Ledger.unlocking`) kept in storage.
        ///   The only way to clean the aforementioned storage item is also user-controlled via
        ///   `withdraw_unbonded`.
        /// - One DB entry.
        /// ----------
        /// Weight: O(1)
        /// DB Weight:
        /// - Read: Ledger, Locks, BalanceOf Stash,
        /// - Write: Locks, Ledger, BalanceOf Stash,
        /// </weight>
        #[pallet::weight(100_000)]
        pub fn unbond(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            ensure!(
                ledger.unlocking.len() < MAX_UNLOCKING_CHUNKS,
                Error::<T>::NoMoreChunks,
            );

            let mut value = value.min(ledger.active);

            if !value.is_zero() {
                ledger.active -= value;

                // Avoid there being a dust balance left in the staking system.
                if ledger.active < T::Currency::minimum_balance() {
                    value += ledger.active;
                    ledger.active = Zero::zero();
                }

                let block_number = <frame_system::Pallet<T>>::block_number();
                let moment = block_number + T::BondingDuration::get();
                ledger.unlocking.push(UnlockChunk { value, moment });
                Self::update_ledger(&controller, &ledger);
                Self::deposit_event(Event::Unbonded(ledger.stash, value));
            }
            Ok(().into())
        }

        /// Remove any unlocked chunks from the `unlocking` queue from our management.
        ///
        /// This essentially frees up that balance to be used by the stash account to do
        /// whatever it wants.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// Emits `Withdrawn`.
        ///
        /// See also [`Call::unbond`].
        ///
        /// # <weight>
        /// - Could be dependent on the `origin` argument and how much `unlocking` chunks exist.
        ///  It implies `consolidate_unlocked` which loops over `Ledger.unlocking`, which is
        ///  indirectly user-controlled. See [`unbond`] for more detail.
        /// - Contains a limited number of reads, yet the size of which could be large based on `ledger`.
        /// - Writes are limited to the `origin` account key.
        /// ---------------
        /// Complexity O(S) where S is the number of slashing spans to remove
        /// Update:
        /// - Reads: Ledger, Locks, [Origin Account]
        /// - Writes: [Origin Account], Locks, Ledger
        /// Kill:
        /// - Reads: Ledger, Bonded, [Origin Account], Locks, BalanceOf stash
        /// - Writes: Bonded, Ledger, [Origin Account], Locks, BalanceOf stash.
        /// NOTE: Weight annotation is the kill scenario, we refund otherwise.
        /// # </weight>
        #[pallet::weight(100_000)]
        fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            let (stash, old_total) = (ledger.stash.clone(), ledger.total);

            let block_number = <frame_system::Pallet<T>>::block_number();
            ledger = ledger.consolidate_unlocked(block_number);

            let post_info_weight =
                if ledger.unlocking.is_empty() && ledger.active < T::Currency::minimum_balance() {
                    // This account must have called `unbond()` with some value that caused the active
                    // portion to fall below existential deposit + will have no more unlocking chunks
                    // left. We can now safely remove all staking-related information.
                    Self::kill_stash(&stash)?;
                    // remove the lock.
                    T::Currency::remove_lock(STAKING_ID, &stash);
                    // This is worst case scenario, so we use the full weight and return None
                    None
                } else {
                    // This was the consequence of a partial unbond. just update the ledger and move on.
                    Self::update_ledger(&controller, &ledger);

                    // This is only an update, so we use less overall weight.
                    //Some(T::WeightInfo::withdraw_unbonded_update(num_slashing_spans))
                    Some(100_000)
                };

            // `old_total` should never be less than the new total because
            // `consolidate_unlocked` strictly subtracts balance.
            if ledger.total < old_total {
                // Already checked that this won't overflow by entry condition.
                let value = old_total - ledger.total;
                Self::deposit_event(Event::Withdrawn(stash, value));
            }

            Ok(post_info_weight.into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Update the ledger for a controller.
        ///
        /// This will also update the stash lock.
        fn update_ledger(
            controller: &T::AccountId,
            ledger: &StakerLedger<T::AccountId, BalanceOf<T>, T::BlockNumber>,
        ) {
            T::Currency::set_lock(
                STAKING_ID,
                &ledger.stash,
                ledger.total,
                WithdrawReasons::all(),
            );
            <Ledger<T>>::insert(controller, ledger);
        }

        /// Remove all associated data of a stash account from the staking system.
        ///
        /// Assumes storage is upgraded before calling.
        ///
        /// This is called:
        /// - after a `withdraw_unbonded()` call that frees all of a stash's bonded balance.
        fn kill_stash(stash: &T::AccountId) -> DispatchResult {
            let controller = <Bonded<T>>::get(stash).ok_or(Error::<T>::NotStash)?;
            <Bonded<T>>::remove(stash);
            <Ledger<T>>::remove(&controller);
            <frame_system::Pallet<T>>::dec_consumers(stash);
            Ok(())
        }
    }
}
