use crate::EverUSDBalance;

pub struct EvercityBalance {
    /// custodian supply
    pub supply: EverUSDBalance,
    /// account balance
    pub account: EverUSDBalance,
    /// bond fund balance
    pub bond_fund: EverUSDBalance,
}

impl EvercityBalance {
    pub fn is_ok(&self) -> bool {
        self.supply == self.account + self.bond_fund
    }
}
