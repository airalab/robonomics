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
pub mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::{boxed::Box, vec};
    use xcm::prelude::*;
    use xcm_builder::{ExecuteController, SendController};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// XCM pallet instance (used for execute & send XCM)
        type XcmPallet: ExecuteController<OriginFor<Self>, Self::RuntimeCall>
            + SendController<OriginFor<Self>>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Max weight for local XCM execution (relatively small - just burn asset)
        #[pallet::constant]
        type MaxWeight: Get<Weight>;

        /// AssetId for teleport (usually native asset)
        #[pallet::constant]
        type AssetId: Get<AssetId>;

        /// Teleport fee amount (will be deducted from parachain account on destination)
        #[pallet::constant]
        type FeeAsset: Get<Asset>;

        /// Teleport Target Location (usually AssetHub or sibling parachain)
        #[pallet::constant]
        type TargetLocation: Get<Location>;

        /// Parachain Location (in Asset Hub perspective)
        #[pallet::constant]
        type ParachainLocation: Get<Location>;

        /// This chain's Universal Location (used for asset reanchor).
        #[pallet::constant]
        type UniversalLocation: Get<InteriorLocation>;

        /// This parameter used for benchmarking only,
        /// transactor helps deposit asset to test account
        #[cfg(feature = "runtime-benchmarks")]
        type AssetTransactor: xcm_executor::traits::TransactAsset;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Assets have been sent. [origin, beneficiary, amount, xcm_hash]
        Teleported {
            origin: T::AccountId,
            beneficiary: Location,
            amount: u128,
            xcm_hash: XcmHash,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Unable to burn asset (not enough balance?)
        BurnFailure,
        /// Failed to send XCM message
        SendFailure,
        /// Failed to reanchor asset (wrong pallet configuration).
        CannotReanchor,
    }

    #[pallet::call(weight(<T as Config>::WeightInfo))]
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
        ///     1_000_000_000,     // 1 XRT
        /// )?;
        /// ```
        #[pallet::call_index(0)]
        pub fn send(
            origin: OriginFor<T>,
            beneficiary: Location,
            amount: u128,
        ) -> DispatchResultWithPostInfo {
            let origin_account = ensure_signed(origin.clone())?;

            // Create asset from amount
            let assets: Assets = vec![T::AssetId::get().into_asset(Fungible(amount))].into();

            // Burn asset locally (prepare for teleport)
            let local_xcm: VersionedXcm<T::RuntimeCall> = VersionedXcm::V5(Xcm(vec![
                WithdrawAsset(assets.clone()),
                ExpectAsset(assets.clone()),
                BurnAsset(assets.clone()),
            ]));
            let weight_used =
                T::XcmPallet::execute(origin, Box::new(local_xcm), T::MaxWeight::get())
                    .map_err(|_| Error::<T>::BurnFailure)?;

            // Get asset for fees (parachain pays it)
            let fee_asset = T::FeeAsset::get();

            // Reanchor asset for remote chain
            let reanchored_assets = assets
                .reanchored(&T::TargetLocation::get(), &T::UniversalLocation::get())
                .map_err(|_| Error::<T>::CannotReanchor)?;

            // Build the XCM message
            let message: VersionedXcm<()> = VersionedXcm::V5(Xcm(vec![
                // Pay forward
                WithdrawAsset(vec![fee_asset.clone()].into()),
                PayFees { asset: fee_asset },
                // Deposit teleported asset
                ReceiveTeleportedAsset(reanchored_assets),
                DepositAsset {
                    assets: AssetFilter::Wild(WildAsset::All),
                    beneficiary: beneficiary.clone(),
                },
                // Refund fees back to parachain account
                RefundSurplus,
                DepositAsset {
                    assets: AssetFilter::Wild(WildAsset::All),
                    beneficiary: T::ParachainLocation::get(),
                },
            ]));

            // Get destination location
            let dest = VersionedLocation::V5(T::TargetLocation::get());

            // Send XCM
            let xcm_hash =
                T::XcmPallet::send(T::RuntimeOrigin::root(), Box::new(dest), Box::new(message))
                    .map_err(|_| Error::<T>::SendFailure)?;

            // Emit event
            Self::deposit_event(Event::Teleported {
                origin: origin_account,
                beneficiary,
                amount,
                xcm_hash,
            });

            Ok(Some(weight_used.saturating_add(T::WeightInfo::send())).into())
        }
    }
}
