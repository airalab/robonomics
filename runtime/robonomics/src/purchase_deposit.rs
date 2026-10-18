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
//! Inbound transfers of the subscription payment asset (USDT from AssetHub).
//!
//! The asset never becomes a local asset and nothing is held on this chain:
//! the reserve chain keeps the real USDT in the sovereign account of this
//! chain. A deposit to an account buys the subscription product for that
//! account in `pallet-robonomics-subscription-purchase` and the whole
//! deposited amount is then burned. A deposit that cannot buy the product
//! (below the price, product disabled, capacity exceeded, ...) is rejected,
//! so the asset stays in the XCM holding and the sender gets it back through
//! its own error handler or appendix, otherwise it is trapped and claimable.

use crate::{
    xcm_config::{LocationToAccountId, UsdtId},
    Balance, Purchase,
};
use alloc::boxed::Box;
use frame_support::traits::tokens::imbalance::{
    ImbalanceAccounting, UnsafeConstructorDestructor, UnsafeManualAccounting,
};
use xcm::latest::prelude::*;
use xcm_executor::{
    traits::{ConvertLocation, TransactAsset},
    AssetsInHolding,
};

/// Amount of the payment asset that was received and is not yet burned.
///
/// It carries no on-chain value by itself: the reserve chain holds the real
/// asset, so dropping it merely leaves the backing unclaimed.
pub struct PaymentCredit(pub u128);

impl UnsafeConstructorDestructor<u128> for PaymentCredit {
    fn unsafe_clone(&self) -> Box<dyn ImbalanceAccounting<u128>> {
        Box::new(PaymentCredit(self.0))
    }

    fn forget_imbalance(&mut self) -> u128 {
        core::mem::take(&mut self.0)
    }
}

impl UnsafeManualAccounting<u128> for PaymentCredit {
    fn saturating_subsume(&mut self, mut other: Box<dyn ImbalanceAccounting<u128>>) {
        self.0 = self.0.saturating_add(other.forget_imbalance());
    }
}

impl ImbalanceAccounting<u128> for PaymentCredit {
    fn amount(&self) -> u128 {
        self.0
    }

    fn saturating_take(&mut self, amount: u128) -> Box<dyn ImbalanceAccounting<u128>> {
        let taken = self.0.min(amount);
        self.0 -= taken;
        Box::new(PaymentCredit(taken))
    }
}

/// Transactor that accepts only the payment asset and spends every deposit
/// on the subscription product of the account the deposit location converts
/// to.
///
/// Withdrawals and transfers are not supported.
pub struct PurchaseDepositTransactor;

impl TransactAsset for PurchaseDepositTransactor {
    fn deposit_asset(
        mut what: AssetsInHolding,
        who: &Location,
        _context: Option<&XcmContext>,
    ) -> Result<(), (AssetsInHolding, XcmError)> {
        let id = UsdtId::get();
        if !what.non_fungible.is_empty()
            || what.fungible.len() != 1
            || !what.fungible.contains_key(&id)
        {
            return Err((what, XcmError::AssetNotFound));
        }
        let Some(account) = LocationToAccountId::convert_location(who) else {
            return Err((what, XcmError::AssetNotFound));
        };
        let amount: Balance = what.fungible[&id].amount();
        if Purchase::purchase_with_deposit(&account, amount).is_err() {
            return Err((
                what,
                XcmError::FailedToTransactAsset("subscription purchase"),
            ));
        }
        // Burn the whole deposit: the product is bought and nothing is held.
        if let Some(mut credit) = what.fungible.remove(&id) {
            credit.forget_imbalance();
        }
        Ok(())
    }

    fn mint_asset(what: &Asset, _context: &XcmContext) -> Result<AssetsInHolding, XcmError> {
        match what {
            Asset {
                id,
                fun: Fungible(amount),
            } if *id == UsdtId::get() => Ok(AssetsInHolding::new_from_fungible_credit(
                id.clone(),
                Box::new(PaymentCredit(*amount)),
            )),
            _ => Err(XcmError::AssetNotFound),
        }
    }
}
