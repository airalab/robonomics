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
use frame_support::traits::{
    Currency, Imbalance, LockIdentifier, LockableCurrency, WithdrawReasons,
};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::Zero,
    Perbill, RuntimeDebug,
};
use sp_std::prelude::*;

pub use pallet::*;

const STAKING_ID: LockIdentifier = *b"rbcstake";

pub trait OnBondHandler<Balance> {
    fn on_bond(value: Balance);
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UnlockChunk<Balance: HasCompact, Moment: HasCompact> {
    /// Amount of funds to be unlocked.
    #[codec(compact)]
    value: Balance,
    /// Time moment at which point it'll be unlocked.
    #[codec(compact)]
    moment: Moment,
}

/// The ledger of a (bonded) stash.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
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

        /// Some time that staked funds must remain bonded for.
        #[pallet::constant]
        type BondingDuration: Get<Self::BlockNumber>;

        /// Standard stake reward for 1 XRT each block.
        /// Note: Perbill has accuracy 10^9, that equal to Wn.
        #[pallet::constant]
        type StakeReward: Get<Perbill>;

        /// Bonus stake income for 1 XRT each block.
        /// Note: Perbill has accuracy 10^9, that equal to Wn.
        #[pallet::constant]
        type BonusReward: Get<Perbill>;
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
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not bond with value less than minimum required.
        InsufficientBond,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An account has bonded this amount. \[stash, amount\]
        Bonded(T::AccountId, BalanceOf<T>),
        /// An account has unbonded this amount. \[stash, amount\]
        Unbonded(T::AccountId, BalanceOf<T>),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue. \[stash, amount\]
        Withdrawn(T::AccountId, BalanceOf<T>),
        /// The staker has been rewarded by this amount. \[stash, amount\]
        Reward(T::AccountId, BalanceOf<T>),
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
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

    #[pallet::storage]
    #[pallet::getter(fn bonus)]
    /// Map from all locked "stash" accounts to the bonus token amount.
    /// Note: Bonus tokens has increased staking rate.
    pub(super) type Bonus<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>>;


    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        BalanceOf<T>: From<T::BlockNumber>,
    {
        #[pallet::weight(100_000)]
        pub fn release(origin: OriginFor<T>, controller: T::AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

            // Send rewards
            Self::claim(&controller, &mut ledger)?;

            // Release locks
            Self::kill_stash(&ledger.stash)?;
            T::Currency::remove_lock(STAKING_ID, &ledger.stash);

            Ok(())
        }
    }

    impl<T: Config> Pallet<T>
    where
        BalanceOf<T>: From<T::BlockNumber>,
    {
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

        /// Get reward for given ledger at block number.
        ///
        /// Each reward has bonus and standard rate. When bonus value is zero,
        /// only standard reward payed for all locked value.
        ///
        /// When locked value is more then bonus value,
        /// value over bonus paid in standard rate.
        fn get_reward(
            ledger: &StakerLedger<T::AccountId, BalanceOf<T>, T::BlockNumber>,
            block_number: T::BlockNumber,
        ) -> BalanceOf<T> {
            if block_number <= ledger.claimed_rewards {
                return Zero::zero();
            }
            let duration = block_number - ledger.claimed_rewards;

            let bonus = Self::bonus(&ledger.stash).unwrap_or(Zero::zero());
            let bonus_stake = if bonus > ledger.active {
                ledger.active
            } else {
                bonus
            };
            let bonus_reward = T::BonusReward::get() * bonus_stake;

            let stake = ledger.active - bonus_stake;
            let stake_reward = T::StakeReward::get() * stake;

            (bonus_reward + stake_reward) * duration.into()
        }

        /// Claim rewards and update ledger.
        fn claim(
            controller: &T::AccountId,
            ledger: &mut StakerLedger<T::AccountId, BalanceOf<T>, T::BlockNumber>,
        ) -> DispatchResult {
            let block_number = <frame_system::Pallet<T>>::block_number();
            let reward = Self::get_reward(&ledger, block_number);
            if reward > Zero::zero() {
                let imbalance = T::Currency::deposit_into_existing(&ledger.stash, reward)?;
                ledger.claimed_rewards = block_number;
                Self::update_ledger(&controller, &ledger);
                Self::deposit_event(Event::Reward(ledger.stash.clone(), imbalance.peek()));
            }
            Ok(().into())
        }
    }
}
