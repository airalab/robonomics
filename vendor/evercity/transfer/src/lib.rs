#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{
        Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, WithdrawReasons,
    },
    weights::Weight,
};
use frame_system::ensure_signed;
#[cfg(test)]
mod tests;

const EVERCITY_LOCK_ID: LockIdentifier = *b"ever/fee";

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait WeightInfo {
    fn transfer() -> Weight;
}

impl WeightInfo for () {
    #[allow(clippy::unnecessary_cast)]
    fn transfer() -> Weight {
        10000_u64 as Weight
    }
}

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// The currency in which fees are paid and contract balances are held.
    type Currency: LockableCurrency<Self::AccountId>;
    type WeightInfo: WeightInfo;
    /// The maximum value that can be transferred at once
    type MaximumTransferValue: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        BalanceOf = BalanceOf<T>,
    {
        /// Account endowed. \[account, value\]
        Endow(AccountId, BalanceOf),
    }
);

decl_storage! {
    trait Store for Module<T: Config> as EvercityTransfer {

    }
}

decl_error! {
    /// Error for the Transfer module
    pub enum Error for Module<T: Config> {
        /// Attempt to transfer more than defined limit
        TransferRestriction,
    }
}

decl_module! {
    /// Transfer module declaration.
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        const MaximumTransferValue: BalanceOf<T> = T::MaximumTransferValue::get();

        fn deposit_event() = default;

        #[weight = <T as Config>::WeightInfo::transfer()]
        fn transfer(origin,  who: T::AccountId, value: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(value<= T::MaximumTransferValue::get(), Error::<T>::TransferRestriction);

            T::Currency::transfer(&sender, &who, value, ExistenceRequirement::AllowDeath )?;

            T::Currency::extend_lock(EVERCITY_LOCK_ID, &who, value, WithdrawReasons::except(WithdrawReasons::FEE) );
            Self::deposit_event(RawEvent::Endow(who, value));
            Ok(())
        }
    }
}
