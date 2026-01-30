///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
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
//! # Wrapped Asset Pallet
//!
//! The Wrapped Asset pallet enables bidirectional conversion between the parachain's
//! native token and a foreign asset representation on Asset Hub via XCM.
//!
//! ## Overview
//!
//! When the parachain's native token is represented as a foreign asset on Asset Hub,
//! users need a way to:
//! 1. **Wrap**: Burn native tokens locally and send equivalent foreign assets from the
//!    sovereign account on Asset Hub
//! 2. **Unwrap**: Receive foreign assets via XCM and mint equivalent native tokens locally
//!
//! The pallet tracks the total amount of foreign assets held in the sovereign account on
//! Asset Hub to prevent sending more than is available.
//!
//! ## Security Model
//!
//! - **No unauthorized minting**: Native tokens can only be minted when foreign assets
//!   arrive via XCM
//! - **Conservation**: `TotalWrapped` must always equal the foreign asset balance in the
//!   sovereign account on Asset Hub
//! - **No over-wrapping**: Cannot send more foreign assets than available in sovereign account
//! - **Atomic operations**: All storage updates are atomic with token operations
//! - **XCM origin validation**: Only Asset Hub can trigger unwrapping (validate in XCM configuration)
//!
//! ## Setup Requirements
//!
//! The sovereign account on Asset Hub must be funded with relay chain tokens for XCM
//! execution fees. The sovereign account can be calculated using the parachain ID.
//!
//! ## Usage
//!
//! ### Wrapping Tokens
//!
//! ```ignore
//! // Wrap 100 native tokens and send to caller
//! WrappedNative::wrap_and_send(origin, 100, None)?;
//!
//! // Wrap 100 native tokens and send to custom beneficiary
//! let beneficiary = MultiLocation::new(1, X1(AccountId32 { network: None, id: [0u8; 32] }));
//! WrappedNative::wrap_and_send(origin, 100, Some(beneficiary))?;
//! ```
//!
//! ### Unwrapping Tokens
//!
//! Unwrapping is handled automatically when foreign assets arrive via XCM. The runtime's
//! XCM configuration must be set up to call `handle_incoming_unwrap` when foreign assets
//! matching `ForeignAssetLocation` are received.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated)] // Allow deprecated RuntimeEvent for now
#![allow(clippy::let_unit_value)] // False positive on type declarations

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame_support::{
        pallet_prelude::*,
        traits::{fungible::Mutate as FungibleMutate, Currency, ExistenceRequirement},
    };
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::Encode;
    use sp_runtime::{traits::Zero, Saturating};
    use sp_std::prelude::*;
    use xcm::latest::prelude::*;

    pub type BalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    /// Configuration trait for the Wrapped Asset pallet.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_xcm::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The native currency (must support minting/burning).
        type NativeCurrency: Currency<Self::AccountId>
            + FungibleMutate<Self::AccountId, Balance = BalanceOf<Self>>;

        /// MultiLocation of this chain's native token when represented as foreign asset on Asset Hub.
        #[pallet::constant]
        type ForeignAssetLocation: Get<Location>;

        /// Asset Hub location.
        #[pallet::constant]
        type AssetHubLocation: Get<Location>;

        /// Amount of relay chain asset to use for XCM execution fees on Asset Hub.
        #[pallet::constant]
        type XcmFeeAmount: Get<u128>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: crate::weights::WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Total native tokens minted through unwrapping.
    ///
    /// This represents the sovereign account's foreign asset balance on Asset Hub.
    /// It tracks how much can be wrapped and sent without exceeding the available balance.
    #[pallet::storage]
    #[pallet::getter(fn total_wrapped)]
    pub type TotalWrapped<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Native tokens wrapped and sent to Asset Hub as foreign asset.
        ///
        /// Parameters:
        /// - `AccountId`: The account that initiated the wrap
        /// - `Balance`: Amount of native tokens wrapped
        /// - `Location`: Destination where foreign assets were sent
        NativeWrapped {
            who: T::AccountId,
            amount: BalanceOf<T>,
            destination: Location,
        },
        /// Foreign asset received from Asset Hub, unwrapped to native tokens.
        ///
        /// Parameters:
        /// - `AccountId`: The beneficiary who received native tokens
        /// - `Balance`: Amount of native tokens minted
        NativeUnwrapped {
            who: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient native token balance.
        InsufficientBalance,
        /// Insufficient wrapped balance on Asset Hub (TotalWrapped < amount).
        InsufficientWrappedBalance,
        /// Invalid amount (e.g., zero).
        InvalidAmount,
        /// Failed to burn native tokens.
        BurnFailed,
        /// Failed to mint native tokens.
        MintFailed,
        /// Failed to send XCM message.
        XcmSendFailed,
        /// Amount conversion overflow.
        AmountOverflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Wrap native tokens and send equivalent foreign assets from Asset Hub.
        ///
        /// Burns native tokens from the caller and sends an XCM message to Asset Hub to
        /// withdraw foreign assets from the sovereign account and send them to the beneficiary.
        ///
        /// # Parameters
        ///
        /// - `origin`: The account wrapping tokens (must be signed)
        /// - `amount`: Amount of native tokens to wrap
        /// - `beneficiary`: Optional destination for foreign assets. If `None`, defaults to caller's AccountId
        ///
        /// # Errors
        ///
        /// - `InvalidAmount`: If amount is zero
        /// - `InsufficientBalance`: If caller doesn't have enough native tokens
        /// - `InsufficientWrappedBalance`: If TotalWrapped < amount (sovereign account doesn't have enough)
        /// - `BurnFailed`: If burning native tokens fails
        /// - `XcmSendFailed`: If sending XCM message fails
        /// - `AmountOverflow`: If amount conversion overflows
        ///
        /// # Events
        ///
        /// - `NativeWrapped`: Emitted when tokens are successfully wrapped
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::wrap_and_send())]
        pub fn wrap_and_send(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            beneficiary: Option<Location>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Validate amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::InvalidAmount);

            // Check caller has sufficient balance
            let balance = T::NativeCurrency::free_balance(&who);
            ensure!(balance >= amount, Error::<T>::InsufficientBalance);

            // Check sovereign account has sufficient wrapped balance
            let total_wrapped = TotalWrapped::<T>::get();
            ensure!(
                total_wrapped >= amount,
                Error::<T>::InsufficientWrappedBalance
            );

            // Burn native tokens
            let _imbalance = T::NativeCurrency::withdraw(
                &who,
                amount,
                frame_support::traits::WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|_| Error::<T>::BurnFailed)?;

            // Decrement TotalWrapped
            TotalWrapped::<T>::put(total_wrapped.saturating_sub(amount));

            // Determine destination
            let destination = if let Some(dest) = beneficiary {
                dest
            } else {
                // Convert AccountId to Location
                Location::new(
                    0,
                    [Junction::AccountId32 {
                        network: None,
                        id: Self::account_to_bytes(&who),
                    }],
                )
            };

            // Convert amount to u128
            let amount_u128: u128 = Self::balance_to_u128(amount)?;

            // Build XCM message
            let fee_amount = T::XcmFeeAmount::get();
            let foreign_asset_location = T::ForeignAssetLocation::get();
            let relay_asset_location = Location::parent();

            let assets_to_withdraw: Assets = vec![
                (foreign_asset_location.clone(), amount_u128).into(),
                (relay_asset_location.clone(), fee_amount).into(),
            ]
            .into();

            let message = Xcm(vec![
                WithdrawAsset(assets_to_withdraw),
                BuyExecution {
                    fees: (relay_asset_location, fee_amount).into(),
                    weight_limit: Unlimited,
                },
                DepositAsset {
                    assets: Wild(AllOf {
                        id: AssetId(foreign_asset_location),
                        fun: WildFungibility::Fungible,
                    }),
                    beneficiary: destination.clone(),
                },
            ]);

            // Send XCM message to Asset Hub
            let asset_hub_location = T::AssetHubLocation::get();
            pallet_xcm::Pallet::<T>::send_xcm(Here, asset_hub_location, message)
                .map_err(|_| Error::<T>::XcmSendFailed)?;

            // Emit event
            Self::deposit_event(Event::NativeWrapped {
                who,
                amount,
                destination,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Handle incoming unwrap request from Asset Hub.
        ///
        /// This function should be called by the XCM transactor when foreign assets
        /// arrive from Asset Hub. It mints native tokens to the beneficiary and
        /// increments `TotalWrapped`.
        ///
        /// # Parameters
        ///
        /// - `beneficiary`: Account to receive minted native tokens
        /// - `amount`: Amount of native tokens to mint
        ///
        /// # Errors
        ///
        /// - `AmountOverflow`: If amount conversion overflows
        /// - `MintFailed`: If minting native tokens fails
        ///
        /// # Events
        ///
        /// - `NativeUnwrapped`: Emitted when tokens are successfully unwrapped
        pub fn handle_incoming_unwrap(beneficiary: T::AccountId, amount: u128) -> DispatchResult {
            // Convert amount to Balance
            let amount_balance = Self::u128_to_balance(amount)?;

            // Mint native tokens to beneficiary
            let _imbalance = T::NativeCurrency::deposit_creating(&beneficiary, amount_balance);

            // Increment TotalWrapped
            TotalWrapped::<T>::mutate(|total| {
                *total = total.saturating_add(amount_balance);
            });

            // Emit event
            Self::deposit_event(Event::NativeUnwrapped {
                who: beneficiary,
                amount: amount_balance,
            });

            Ok(())
        }

        /// Get total wrapped balance (sovereign account balance on Asset Hub).
        pub fn get_total_wrapped() -> BalanceOf<T> {
            TotalWrapped::<T>::get()
        }

        /// Check if amount can be wrapped and sent.
        ///
        /// Returns true if the sovereign account has enough foreign assets to fulfill
        /// the wrap request.
        pub fn can_wrap(amount: BalanceOf<T>) -> bool {
            let total_wrapped = TotalWrapped::<T>::get();
            total_wrapped >= amount && !amount.is_zero()
        }

        /// Get maximum amount that can be wrapped.
        ///
        /// Returns the current `TotalWrapped` value, which represents the maximum
        /// amount of native tokens that can be wrapped.
        pub fn max_wrappable() -> BalanceOf<T> {
            TotalWrapped::<T>::get()
        }

        // Helper function to convert Balance to u128
        fn balance_to_u128(balance: BalanceOf<T>) -> Result<u128, Error<T>> {
            balance.try_into().map_err(|_| Error::<T>::AmountOverflow)
        }

        // Helper function to convert u128 to Balance
        fn u128_to_balance(value: u128) -> Result<BalanceOf<T>, Error<T>> {
            value.try_into().map_err(|_| Error::<T>::AmountOverflow)
        }

        // Helper function to convert AccountId to bytes
        fn account_to_bytes(account: &T::AccountId) -> [u8; 32] {
            let encoded = account.encode();
            let mut bytes = [0u8; 32];
            let len = encoded.len().min(32);
            bytes[..len].copy_from_slice(&encoded[..len]);
            bytes
        }
    }
}
