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

pub use pallet::*;

/// Counter for the number of eras that have passed.                                                                      
pub type EraIndex = u32;

/// Indicates the initial status of the staker.                                                                           
#[derive(RuntimeDebug)]                                                                                                   
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]                                                
pub enum StakerStatus<AccountId> {                                                                                        
    /// Chilling (just take a protocol inflation).
    Idle,                                                                                                                 
    // Trade bandwidth to get fees rewards.
    //TradeFees,
    // Trade bandwidth to get subscription rewards.
    //TradeSubscription,
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

        /// Time used for computing era duration.
        type UnixTime: UnixTime;

        /// The overarching event type.
        type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Number of eras that staked funds must remain bonded for.
        #[pallet::constant]
        type BondingDuration: Get<Moment>;
    }

    #[pallet::error]
    pub enum Error<T> {
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn total_stake)]
    pub(super) type TotalStake<T> = StorageMap<_, BalanceOf<T>>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[weight = T::WeightInfo::bond()]
        pub fn bond(origin,
            controller: <T::Lookup as StaticLookup>::Source,
            #[compact] value: BalanceOf<T>,
        ) {
            let stash = ensure_signed(origin)?;
            if <Bonded<T>>::contains_key(&stash) {
                Err(Error::<T>::AlreadyBonded)?
            }

            let controller = T::Lookup::lookup(controller)?;
            if <Ledger<T>>::contains_key(&controller) {
                Err(Error::<T>::AlreadyPaired)?
            }

            system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;
            <Bonded<T>>::insert(&stash, &controller);

            let current_era = CurrentEra::get().unwrap_or(0);
            let history_depth = Self::history_depth();
            let last_reward_era = current_era.saturating_sub(history_depth);

            let stash_balance = T::Currency::free_balance(&stash);
            let value = value.min(stash_balance);

            Self::deposit_event(RawEvent::Bonded(stash.clone(), value));
            let item = StakingLedger {
                stash,
                total: value,
                active: value,
                unlocking: vec![],
                claimed_rewards: (last_reward_era..current_era).collect(),
            };
            Self::update_ledger(&controller, &item);
        }

        pub fn unbond() {
        }

        pub fn unstake() {
        }
    }
}
