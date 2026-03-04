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
//! # Robonomics XCM Teleport Pallet (TeleportXrt)
//!
//! This pallet provides a simplified interface for sending native XRT tokens from the
//! Robonomics parachain to Asset Hub using XCM teleport with explicit fee control.
//!
//! ## Overview
//!
//! The pallet implements a restricted XCM teleport with the following characteristics:
//! - **Single Asset**: Only supports the native asset (via pallet_balances)
//! - **Single Destination**: Hardcoded to Asset Hub parachain (para ID 1000)
//! - **Explicit Fees**: Separate fee parameter for relay chain asset fees on Asset Hub
//! - **Simple API**: Beneficiary specified as raw AccountId32 bytes ([u8; 32])
//!
//! ## XCM Flow
//!
//! The send operation constructs an XCM message with the following instruction sequence:
//! 1. `WithdrawAsset` - Withdraws native assets from the sender's account
//! 2. `InitiateTransfer` - Initiates the teleport to Asset Hub with:
//!    - Teleport filter for both remote_fees and assets
//!    - Local execution handled automatically
//! 3. Remote XCM on Asset Hub:
//!    - `PayFees` - Pays execution fees using relay chain asset
//!    - `DepositAsset` - Deposits teleported assets to beneficiary
//!
//! ## Example
//!
//! ```rust,ignore
//! use pallet_robonomics_teleport;
//!
//! // Send 1000 XRT to beneficiary on Asset Hub, paying 50 relay chain tokens for fees
//! let beneficiary = [0x01; 32]; // AccountId32 on Asset Hub
//! let amount = 1_000_000_000; // 1000 XRT (assuming 9 decimals)
//! let fee = 50_000_000; // Fee in relay chain asset
//!
//! TeleportXrt::send(
//!     origin,
//!     beneficiary,
//!     amount,
//!     fee
//! )?;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::Currency,
    };
    use frame_system::pallet_prelude::*;
    use sp_std::vec;
    use xcm::prelude::*;
    use xcm::opaque::latest::AssetTransferFilter;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type for balance operations on native asset
        type Currency: Currency<Self::AccountId>;

        /// XCM message sender for cross-chain communication
        type XcmSender: SendXcm;

        /// Location of Asset Hub parachain (typically parachain 1000 on relay chain)
        #[pallet::constant]
        type AssetHubLocation: Get<Location>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Native assets have been sent to Asset Hub via XCM teleport.
        /// 
        /// Parameters:
        /// - `origin`: The account that initiated the send
        /// - `beneficiary`: The destination account location on Asset Hub
        /// - `asset`: The XCM asset that was sent (contains amount and asset ID)
        Sent {
            origin: T::AccountId,
            beneficiary: Location,
            asset: Asset,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Failed to send XCM message to destination
        SendFailure,
        /// Failed to execute XCM locally (unused but kept for compatibility)
        LocalExecutionFailed,
        /// Amount exceeds maximum u128 value
        AmountOverflow,
        /// Failed to construct asset transfer filter for XCM
        InvalidAssetFilter,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Send native assets to Asset Hub parachain via XCM teleport.
        ///
        /// This extrinsic sends native assets (XRT) from the caller's account to a beneficiary
        /// account on the Asset Hub parachain using XCM's InitiateTransfer instruction with
        /// teleport semantics.
        ///
        /// # Process
        /// 
        /// 1. Validates and converts the amount to u128 for XCM
        /// 2. Constructs an XCM message with:
        ///    - `WithdrawAsset`: Withdraws native assets from sender
        ///    - `InitiateTransfer`: Teleports to Asset Hub (handles local execution automatically)
        ///    - Remote XCM: `PayFees` (relay asset) + `DepositAsset` to beneficiary
        /// 3. Sends the XCM message to Asset Hub
        /// 4. Emits `Sent` event with transfer details
        ///
        /// # Parameters
        /// 
        /// - `origin`: Must be a signed account with sufficient native asset balance
        /// - `beneficiary`: The 32-byte AccountId32 of the recipient on Asset Hub
        /// - `amount`: Amount of native asset to send (will be converted to u128)
        /// - `fee`: Amount of relay chain asset to use for execution fees on Asset Hub
        ///
        /// # Errors
        /// 
        /// - `AmountOverflow`: If amount cannot be converted to u128
        /// - `InvalidAssetFilter`: If asset transfer filter construction fails
        /// - `SendFailure`: If XCM message cannot be sent to Asset Hub
        ///
        /// # Example
        /// 
        /// ```rust,ignore
        /// TeleportXrt::send(
        ///     Origin::signed(alice),
        ///     [0x01; 32],        // Beneficiary AccountId32
        ///     1_000_000_000,     // 1000 XRT
        ///     50_000_000,        // 50 relay tokens for fees
        /// )?;
        /// ```
        #[pallet::call_index(0)]
        #[pallet::weight({
            // Simple weight based on XCM execution
            Weight::from_parts(100_000_000, 10_000)
        })]
        pub fn send(
            origin: OriginFor<T>,
            beneficiary: [u8; 32],
            amount: BalanceOf<T>,
            fee: u128,
        ) -> DispatchResult {
            let origin_account = ensure_signed(origin)?;

            // Build beneficiary location from AccountId32
            let beneficiary_location: Location = AccountId32 {
                network: None,
                id: beneficiary,
            }
            .into();

            // Destination is always Asset Hub
            let dest = T::AssetHubLocation::get();

            // Convert amount to u128 for XCM (u128 is the widest type available)
            let xcm_amount: u128 = amount.try_into().map_err(|_| Error::<T>::AmountOverflow)?;

            // Build the native asset
            let native_asset = Asset {
                id: AssetId(Location::here()),
                fun: Fungibility::Fungible(xcm_amount),
            };

            let assets: Assets = vec![native_asset.clone()].into();

            // Build the relay asset for fees (on Asset Hub, fees are paid in relay chain asset)
            let relay_fee_asset = Asset {
                id: AssetId(Location::parent()),
                fun: Fungibility::Fungible(fee),
            };

            // Build the XCM message using InitiateTransfer for teleport
            // InitiateTransfer will execute locally and send to destination
            // Note: Using Xcm<()> instead of Xcm<RuntimeCall> because SendXcm trait requires ()
            let message: Xcm<()> = Xcm(vec![
                WithdrawAsset(assets.clone()),
                InitiateTransfer {
                    destination: dest.clone(),
                    remote_fees: Some(AssetTransferFilter::Teleport(Wild(AllCounted(1)))),
                    preserve_origin: false,
                    assets: vec![AssetTransferFilter::Teleport(Wild(AllCounted(1)))]
                        .try_into()
                        .map_err(|_| Error::<T>::InvalidAssetFilter)?,
                    remote_xcm: Xcm(vec![
                        PayFees {
                            asset: relay_fee_asset,
                        },
                        DepositAsset {
                            assets: Wild(AllCounted(1)),
                            beneficiary: beneficiary_location.clone(),
                        },
                    ]),
                },
            ]);

            // Send the message using XcmSender
            let (ticket, _) = T::XcmSender::validate(&mut Some(dest.clone()), &mut Some(message))
                .map_err(|_| Error::<T>::SendFailure)?;
            T::XcmSender::deliver(ticket)
                .map_err(|_| Error::<T>::SendFailure)?;

            // Emit event
            Self::deposit_event(Event::Sent {
                origin: origin_account,
                beneficiary: beneficiary_location,
                asset: native_asset,
            });

            Ok(())
        }
    }
}
