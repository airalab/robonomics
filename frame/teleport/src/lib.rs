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
//! # Robonomics XCM Teleport Pallet
//!
//! This pallet provides a simplified send functionality for Robonomics parachain
//! to send assets to Asset Hub parachain using XCM.
//!
//! ## Overview
//!
//! The pallet implements a strict version of XCM teleport with the following constraints:
//! - Only supports sending the native asset (pallet_balances)
//! - Only supports sending to Asset Hub parachain
//! - Uses relay chain asset for fees on Asset Hub
//! - Beneficiary specified as AccountId32 (32-byte account ID)
//!
//! The send process follows this pattern:
//! 1. Build XCM message with WithdrawAsset and InitiateTransfer instructions
//! 2. InitiateTransfer executes locally to withdraw assets and sends to destination
//! 3. On Asset Hub: PayFees (with relay asset) → DepositAsset to beneficiary

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use xcm_builder::{ExecuteController, SendController};
    use sp_std::{vec, boxed::Box};
    use xcm::prelude::*;

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
        /// Send native assets to Asset Hub parachain.
        ///
        /// This extrinsic sends native assets (XRT) from the caller's account to a beneficiary
        /// account on the Asset Hub parachain. The transfer uses XCM InitiateTransfer to:
        /// 1. Initiate transfer to Asset Hub with teleport semantics
        /// 2. Pay fees on Asset Hub using relay chain asset
        /// 3. Deposit assets to beneficiary on Asset Hub
        ///
        /// # Parameters
        /// - `origin`: The account sending the assets
        /// - `beneficiary`: The recipient AccountId32 (32-byte account ID) on Asset Hub
        /// - `amount`: The amount of native asset to send
        /// - `fee`: The amount of relay chain asset to use for fees on Asset Hub
        ///
        /// # Errors
        /// - `SendFailure`: Failed to send XCM message
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
            let weight_used = T::XcmPallet::execute(
                origin,
                Box::new(local_xcm),
                T::MaxWeight::get(),
            ).map_err(|_| Error::<T>::BurnFailure)?;

            // Get asset for fees (parachain pays it)
            let fee_asset = T::FeeAsset::get();

            // Reanchor asset for remote chain
            let reanchored_assets = assets.reanchored(
                    &T::TargetLocation::get(),
                    &T::UniversalLocation::get(),
                ).map_err(|_| Error::<T>::CannotReanchor)?;

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
                }
            ]));

            // Get destination location
            let dest = VersionedLocation::V5(T::TargetLocation::get());

            // Send XCM
            let xcm_hash = T::XcmPallet::send(
                T::RuntimeOrigin::root(),
                Box::new(dest),
                Box::new(message),
            ).map_err(|_| Error::<T>::SendFailure)?;

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
