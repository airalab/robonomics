use crate::bond::{BondInterest, BondPeriod, BondPeriodNumber, BondStruct};
use crate::EverUSDBalance;
use frame_support::{
    codec::{Decode, Encode},
    sp_runtime::RuntimeDebug,
    sp_std::cmp::Ordering,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// ... |         period            |   ...
// --- | ------------------------- | -------------...
//     |                  |        |          |
//   start              report   payment    interest pay
//    >----------------------------< coupon accrual
// report release period  >--------<
//              coupon pay period  >----------<

pub struct PeriodDescr {
    pub start_period: BondPeriod,            // sec from activation
    pub impact_data_send_period: BondPeriod, // sec from activation
    pub payment_period: BondPeriod,          // sec from activation
    pub interest_pay_period: BondPeriod,     // sec from activation
}

impl PeriodDescr {
    /// Returns bond usage time for the period.
    /// moment is a time when bond was bought.
    pub fn duration(&self, moment: BondPeriod) -> BondPeriod {
        if moment <= self.start_period {
            self.payment_period - self.start_period
        } else if moment >= self.payment_period {
            0
        } else {
            self.payment_period - moment
        }
    }
}

/// Struct, storing per-period coupon_yield and effective interest_rate for given bond
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct PeriodYield {
    /// bond cumulative accrued yield for this period
    #[codec(compact)]
    pub total_yield: EverUSDBalance,
    // obsolete
    // pub coupon_yield_before: EverUSDBalance,
    /// effective interest rate for current period
    #[codec(compact)]
    pub interest_rate: BondInterest,
}

/// Struct, used by BondApi::get_impact_reports
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct PeriodDataStruct {
    pub interest_rate: BondInterest,
    pub create_period: BondPeriod,
    pub impact_data: u64,
    pub signed: bool,
}

pub struct PeriodIterator<'a, AccountId, Moment, Hash> {
    bond: &'a BondStruct<AccountId, Moment, Hash>,
    index: BondPeriodNumber,
}

impl<'a, AccountId, Moment, Hash> PeriodIterator<'a, AccountId, Moment, Hash> {
    pub fn new(bond: &'a BondStruct<AccountId, Moment, Hash>) -> Self {
        PeriodIterator { bond, index: 0 }
    }
    pub fn starts_with(
        bond: &'a BondStruct<AccountId, Moment, Hash>,
        index: BondPeriodNumber,
    ) -> Self {
        PeriodIterator { bond, index }
    }
}

impl<'a, AccountId, Moment, Hash> core::iter::Iterator
    for PeriodIterator<'a, AccountId, Moment, Hash>
{
    type Item = PeriodDescr;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = &self.bond.inner;
        let index = self.index as BondPeriodNumber;
        self.index += 1;
        let payment_period: BondPeriod =
            inner.start_period + inner.payment_period.saturating_mul(index);
        match (inner.bond_duration + 1).cmp(&index) {
            Ordering::Greater => {
                let start_period = if index == 0 {
                    0
                } else {
                    payment_period - inner.payment_period
                };

                Some(PeriodDescr {
                    payment_period,
                    start_period,
                    impact_data_send_period: payment_period - inner.impact_data_send_period,
                    interest_pay_period: start_period + inner.interest_pay_period,
                })
            }
            Ordering::Less => None,
            Ordering::Equal => Some(PeriodDescr {
                payment_period,
                start_period: payment_period - inner.payment_period,
                impact_data_send_period: payment_period,
                // last pay period is special and lasts bond_finishing_period seconds
                interest_pay_period: payment_period + inner.bond_finishing_period,
            }),
        }
    }
}
