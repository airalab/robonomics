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
//! Robonomics crowdloan runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, traits::{Currency, VestingSchedule}};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Zero;

    use super::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config + TypeInfo {
        /// Crowdloan reward currentcy.
        type Currency: Currency<Self::AccountId>;
        /// Vesting implementation for reward transfers.
        type Vesting: VestingSchedule<Self::AccountId, Currency = Self::Currency, Moment = Self::BlockNumber>;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Crowdloan vesting release schedule: lock duration.
        #[pallet::constant]
        type VestingLockPeriod: Get<BalanceOf<Self>>;
        /// Crowdloan vesting release schedule: start block.
        #[pallet::constant]
        type VestingStartBlock: Get<Self::BlockNumber>;
    }

    #[pallet::error]
    pub enum Error<T> {
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Crowdloan reward paid
        RewardPaid(T::AccountId, BalanceOf<T>),
        /// Reward is too small for vesting, sent general transfer
        SmallReward(T::AccountId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Reward crowdloan participant.
        #[pallet::weight(100_000_000)]
        pub fn send_reward(
            origin: OriginFor<T>,
            recipient: T::AccountId,
            #[pallet::compact] reward_value: BalanceOf<T>,
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;

            // Mint reward balance
            let _ = T::Currency::deposit_into_existing(&recipient, reward_value)?;
            Self::deposit_event(Event::RewardPaid(recipient.clone(), reward_value));

            // Add vesting schedule
            let per_block = reward_value / T::VestingLockPeriod::get();
            if per_block.is_zero() {
                // Reward is too small, do not add vesting schedule
                Self::deposit_event(Event::SmallReward(recipient.clone()));
                Ok(())
            } else {
                T::Vesting::add_vesting_schedule(
                    &recipient,
                    reward_value,
                    per_block,
                    T::VestingStartBlock::get(),
                )
            }
        }
    }
}
