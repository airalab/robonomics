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
//! Lighthouse is a block author in robonomics parachain.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use sp_inherents::InherentData;
use sp_inherents::{InherentIdentifier, IsFatalError};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::{Currency, Imbalance, OnUnbalanced};
    use frame_system::pallet_prelude::*;

    type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The native token.
        type Currency: Currency<Self::AccountId>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Reward amount for block proposer.
        #[pallet::constant]
        type BlockReward: Get<BalanceOf<Self>>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Lighthouse already set in block.
        LighthouseAlreadySet,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {
        /// An account rewarded for block production. \[lighthouse, amount\]
        BlockReward(T::AccountId, BalanceOf<T>),
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// Current block lighthouse account.
    #[pallet::storage]
    #[pallet::getter(fn lighthouse)]
    pub(super) type Lighthouse<T: Config> = StorageValue<_, T::AccountId>;

    /// Current block lighthouse reward.
    #[pallet::storage]
    #[pallet::getter(fn fees_reward)]
    pub(super) type BlockReward<T: Config> = StorageValue<_, BalanceOf<T>>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            <Lighthouse<T>>::kill();
            <BlockReward<T>>::kill();
            0
        }

        fn on_finalize(_n: T::BlockNumber) {
            let lighthouse = <Lighthouse<T>>::get().expect("Lighthouse must be set");
            let block_reward = <BlockReward<T>>::get().unwrap_or(T::BlockReward::get());

            let reward_imbalance = T::Currency::issue(block_reward);
            T::Currency::resolve_creating(&lighthouse, reward_imbalance);

            Self::deposit_event(Event::BlockReward(lighthouse, block_reward))
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Inherent to set the lighthouse of a block.
        #[pallet::weight((0, DispatchClass::Mandatory))]
        pub fn set(origin: OriginFor<T>, lighthouse: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;
            ensure!(
                <Lighthouse<T>>::get().is_none(),
                Error::<T>::LighthouseAlreadySet
            );

            // Update storage
            <Lighthouse<T>>::put(&lighthouse);

            Ok(().into())
        }
    }

    #[pallet::inherent]
    impl<T: Config> ProvideInherent for Pallet<T> {
        type Call = Call<T>;
        type Error = InherentError;

        const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

        fn create_inherent(data: &InherentData) -> Option<Self::Call> {
            let lighthouse_raw = data
                .get_data::<InherentType>(&INHERENT_IDENTIFIER)
                .expect("Gets and decodes authorship inherent data")?;
            let lighthouse = T::AccountId::decode(&mut &lighthouse_raw[..])
                .expect("Decodes author raw inherent data");
            Some(Call::set(lighthouse))
        }

        fn check_inherent(_call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
            Ok(())
        }

        fn is_inherent(call: &Self::Call) -> bool {
            matches!(call, Call::set(_))
        }
    }

    impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
        fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
            let current = <BlockReward<T>>::get().unwrap_or(T::BlockReward::get());
            <BlockReward<T>>::put(current + amount.peek());
        }
    }
}

/// Lighthouse inherent identifier
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"lgthouse";

#[derive(codec::Encode)]
#[cfg_attr(feature = "std", derive(Debug, codec::Decode, thiserror::Error))]
pub enum InherentError {
    Other(sp_runtime::RuntimeString),
}

#[cfg(feature = "std")]
impl std::fmt::Display for InherentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "{:?}", self)
    }
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        match *self {
            InherentError::Other(_) => true,
        }
    }
}

impl InherentError {
    /// Try to create an instance ouf of the given identifier and data.
    #[cfg(feature = "std")]
    pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
        if id == &INHERENT_IDENTIFIER {
            <InherentError as codec::Decode>::decode(&mut &data[..]).ok()
        } else {
            None
        }
    }
}

/// The type of data that the inherent will contain.
/// Just a byte array. It will be decoded to an actual account id later.
pub type InherentType = sp_std::vec::Vec<u8>;

/// The thing that the outer node will use to actually inject the inherent data
#[cfg(feature = "std")]
pub struct InherentDataProvider(pub InherentType);

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for InherentDataProvider {
    fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> Result<(), sp_inherents::Error> {
        inherent_data.put_data(INHERENT_IDENTIFIER, &self.0)
    }

    async fn try_handle_error(
        &self,
        identifier: &InherentIdentifier,
        error: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        if *identifier != INHERENT_IDENTIFIER {
            return None;
        }
        match InherentError::try_from(&INHERENT_IDENTIFIER, error)? {
            o => Some(Err(sp_inherents::Error::Application(Box::from(o)))),
        }
    }
}
