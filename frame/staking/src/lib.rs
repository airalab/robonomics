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
    traits::{AtLeast32BitUnsigned, CheckedSub, Saturating, StaticLookup, Zero},
    Perbill, RuntimeDebug,
};
use sp_std::prelude::*;

//pub mod weights;

pub use pallet::*;
//pub use weights::WeightInfo;

const STAKING_ID: LockIdentifier = *b"rbcstake";
const MAX_UNLOCKING_CHUNKS: usize = 32;

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

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub bonus: Vec<(T::AccountId, BalanceOf<T>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            GenesisConfig { bonus: vec![] }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for &(ref who, bonus_value) in self.bonus.iter() {
                <Bonus<T>>::insert(who, bonus_value)
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        BalanceOf<T>: From<T::BlockNumber>,
    {
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
        ) -> DispatchResult {
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

        /// Add some extra amount that have appeared in the stash `free_balance` into the balance up
        /// for staking.
        ///
        /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
        ///
        /// Use this if there are additional funds in your stash account that you wish to bond.
        /// Unlike [`bond`](Self::bond) or [`unbond`](Self::unbond) this function does not impose any limitation
        /// on the amount that can be added.
        ///
        /// Emits `Bonded`.
        ///
        /// # <weight>
        /// - Independent of the arguments. Insignificant complexity.
        /// - O(1).
        /// # </weight>
        #[pallet::weight(100_000)]
        pub fn bond_extra(
            origin: OriginFor<T>,
            #[pallet::compact] max_additional: BalanceOf<T>,
        ) -> DispatchResult {
            let stash = ensure_signed(origin)?;

            let controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            Self::claim(&controller, &mut ledger)?;

            let stash_balance = T::Currency::free_balance(&stash);
            if let Some(extra) = stash_balance.checked_sub(&ledger.total) {
                let extra = extra.min(max_additional);
                ledger.total += extra;
                ledger.active += extra;
                // Last check: the new active amount of ledger must be more than ED.
                ensure!(
                    ledger.active >= T::Currency::minimum_balance(),
                    Error::<T>::InsufficientBond
                );

                Self::deposit_event(Event::<T>::Bonded(stash, extra));
                Self::update_ledger(&controller, &ledger);
            }
            Ok(())
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
        ) -> DispatchResult {
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
        pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
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

        /// Claim rewards accumulated from last `claim_rewards` call.
        ///
        /// Generally, current reward scheme has fixed income for each block. For example,
        /// if you have locked 10 XRT you will get 0.00000004 * 10 XRT for each block.
        ///
        /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        ///
        /// Emits `Reward`.
        ///
        /// See also [`Call::bond`].
        ///
        /// # <weight>
        /// - Contains a limited number of reads, yet the size of which could be large based on `ledger`.
        /// - Writes are limited to the `origin` account key.
        /// ---------------
        /// Weight: O(1)
        /// DB Weight:
        /// - Reads: Ledger, System BlockNumber, Locks, [Origin Account]
        /// - Writes: [Origin Account], Locks, System Account, Ledger
        /// # </weight>
        #[pallet::weight(100_000)]
        pub fn claim_rewards(origin: OriginFor<T>) -> DispatchResult {
            let controller = ensure_signed(origin)?;
            let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;
            Self::claim(&controller, &mut ledger)
        }

        /// Sudo call for extending list of bonus rates.
        #[pallet::weight(100_000)]
        pub fn extend_bonus(
            origin: OriginFor<T>,
            extra: Vec<(T::AccountId, BalanceOf<T>)>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            for &(ref who, bonus_value) in extra.iter() {
                <Bonus<T>>::mutate(who, |value| {
                    *value = Some(value.unwrap_or(Zero::zero()) + bonus_value);
                })
            }
            Ok(().into())
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

#[cfg(test)]
mod tests {
    use frame_support::{assert_err, assert_ok, parameter_types, traits::GenesisBuild};
    use sp_core::H256;
    use sp_runtime::{testing::Header, traits::IdentityLookup};

    use crate::{self as staking, *};

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
    type Block = frame_system::mocking::MockBlock<Runtime>;
    type Balance = u128;

    const XRT: Balance = 1_000_000_000;

    const ALICE: u64 = 1;
    const ALICE_C: u64 = 10;

    const BOB: u64 = 2;
    const BOB_C: u64 = 20;

    const CHARLIE: u64 = 3;
    const CHARLIE_C: u64 = 30;

    frame_support::construct_runtime!(
        pub enum Runtime where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
            Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
            Staking: staking::{Pallet, Call, Storage, Event<T>, Config<T>},
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    impl frame_system::Config for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Call = Call;
        type Hash = H256;
        type Hashing = sp_runtime::traits::BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = Event;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<Balance>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BaseCallFilter = ();
        type SystemWeightInfo = ();
        type BlockWeights = ();
        type BlockLength = ();
        type SS58Prefix = ();
        type OnSetCode = ();
    }

    parameter_types! {
        pub const MaxLocks: u32 = 50;
        pub const MaxReserves: u32 = 50;
        pub const ExistentialDeposit: Balance = 10;
    }

    impl pallet_balances::Config for Runtime {
        type MaxLocks = MaxLocks;
        type MaxReserves = MaxReserves;
        type ReserveIdentifier = [u8; 8];
        type Balance = Balance;
        type Event = Event;
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
    }

    parameter_types! {
        pub const BondingDuration: u64 = 32;
        pub const StakeReward: Perbill = Perbill::from_parts(40);
        pub const BonusReward: Perbill = Perbill::from_parts(200);
    }

    impl Config for Runtime {
        type Currency = Balances;
        type Event = Event;

        type BondingDuration = BondingDuration;
        type StakeReward = StakeReward;
        type BonusReward = BonusReward;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        let _ = pallet_balances::GenesisConfig::<Runtime> {
            balances: vec![(ALICE, 10 * XRT), (BOB, 42 * XRT), (CHARLIE, 10_000 * XRT)],
        }
        .assimilate_storage(&mut storage);

        let _ = staking::GenesisConfig::<Runtime> {
            bonus: vec![(BOB, 30 * XRT)],
        }
        .assimilate_storage(&mut storage);

        storage.into()
    }

    fn events() -> Vec<Event> {
        System::events().iter().map(|e| e.event.clone()).collect()
    }

    #[test]
    fn bond_should_works() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            assert_eq!(System::account(ALICE).data.free, 10 * XRT);

            assert_ok!(Staking::bond(Origin::signed(ALICE), ALICE_C, 5 * XRT));
            assert_eq!(<Bonded<Runtime>>::get(ALICE), Some(ALICE_C));
            assert_eq!(
                events(),
                vec![staking::Event::Bonded(ALICE, 5 * XRT).into(),]
            );
            assert_eq!(
                <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.claimed_rewards),
                Some(1),
            );
            assert_eq!(Balances::locks(ALICE)[0].amount, 5 * XRT);
        })
    }

    #[test]
    fn unbond_should_works() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            assert_ok!(Staking::bond(Origin::signed(ALICE), ALICE_C, 5 * XRT));
            assert_eq!(Balances::locks(ALICE)[0].amount, 5 * XRT);

            System::set_block_number(2);
            assert_ok!(Staking::unbond(Origin::signed(ALICE_C), 2 * XRT));
            assert_eq!(<Bonded<Runtime>>::get(ALICE), Some(ALICE_C));
            assert_eq!(
                <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.active),
                Some(3 * XRT),
            );
            assert_eq!(
                <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.unlocking),
                Some(vec![UnlockChunk {
                    value: 2 * XRT,
                    moment: 34
                }]),
            );
            assert_eq!(Balances::locks(ALICE)[0].amount, 5 * XRT);

            System::set_block_number(20);
            assert_ok!(Staking::withdraw_unbonded(Origin::signed(ALICE_C)));
            assert_eq!(
                <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.unlocking),
                Some(vec![UnlockChunk {
                    value: 2 * XRT,
                    moment: 34
                }]),
            );

            System::set_block_number(35);
            assert_ok!(Staking::withdraw_unbonded(Origin::signed(ALICE_C)));
            assert_eq!(
                <Ledger<Runtime>>::get(ALICE_C).map(|ledger| ledger.unlocking),
                Some(vec![]),
            );
            assert_eq!(Balances::locks(ALICE)[0].amount, 3 * XRT);

            assert_ok!(Staking::unbond(Origin::signed(ALICE_C), 3 * XRT));
            System::set_block_number(100);
            assert_ok!(Staking::withdraw_unbonded(Origin::signed(ALICE_C)));
            assert_eq!(Balances::locks(ALICE), vec![]);
            assert_eq!(<Bonded<Runtime>>::get(ALICE), None);
        })
    }

    #[test]
    fn bond_extra_should_works() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 1 * XRT));
            System::set_block_number(10);
            assert_eq!(System::account(BOB).data.free, 42000000000);

            assert_ok!(Staking::bond_extra(Origin::signed(BOB), 41 * XRT));
            assert_eq!(System::account(BOB).data.free, 42000001800);

            System::set_block_number(1_000);
            assert_ok!(Staking::claim_rewards(Origin::signed(BOB_C)));
            assert_eq!(System::account(BOB).data.free, 42006417000);
        })
    }

    #[test]
    fn reward_should_works() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 42 * XRT));
            assert_ok!(Staking::bond(
                Origin::signed(CHARLIE),
                CHARLIE_C,
                10_000 * XRT
            ));

            System::set_block_number(1_000);
            assert_ok!(Staking::claim_rewards(Origin::signed(BOB_C)));
            assert_ok!(Staking::claim_rewards(Origin::signed(CHARLIE_C)));

            assert_eq!(
                <Ledger<Runtime>>::get(BOB_C).map(|ledger| ledger.claimed_rewards),
                Some(1_000),
            );
            assert_eq!(
                <Ledger<Runtime>>::get(CHARLIE_C).map(|ledger| ledger.claimed_rewards),
                Some(1_000),
            );
            assert_eq!(System::account(BOB).data.free, 42006473520);
            assert_eq!(System::account(CHARLIE).data.free, 10000399600000);

            System::set_block_number(5_000_000);
            assert_ok!(Staking::claim_rewards(Origin::signed(CHARLIE_C)));
            assert_eq!(System::account(CHARLIE).data.free, 11999999600000);
        })
    }

    #[test]
    fn extend_bonus_should_works() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            assert_err!(
                Staking::extend_bonus(Origin::signed(BOB), Default::default()),
                sp_runtime::traits::BadOrigin,
            );
            assert_eq!(Staking::bonus(BOB), Some(30000000000));

            assert_ok!(Staking::extend_bonus(Origin::root(), vec![(BOB, 100_000)]));
            assert_eq!(Staking::bonus(BOB), Some(30000100000));

            assert_ok!(Staking::extend_bonus(Origin::root(), vec![(BOB, 100_000)]));
            assert_eq!(Staking::bonus(BOB), Some(30000200000));
        })
    }

    #[test]
    fn fail_double_bonding() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 1 * XRT));
            assert_err!(
                Staking::bond(Origin::signed(ALICE), BOB_C, 1 * XRT),
                staking::Error::<Runtime>::AlreadyPaired,
            );
            assert_err!(
                Staking::bond(Origin::signed(BOB), ALICE_C, 1 * XRT),
                staking::Error::<Runtime>::AlreadyBonded,
            );
        })
    }

    #[test]
    fn fail_controller_calls() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            assert_ok!(Staking::bond(Origin::signed(BOB), BOB_C, 1 * XRT));
            assert_err!(
                Staking::claim_rewards(Origin::signed(BOB)),
                staking::Error::<Runtime>::NotController,
            );
            assert_err!(
                Staking::unbond(Origin::signed(BOB), 1 * XRT),
                staking::Error::<Runtime>::NotController,
            );
        })
    }
}
