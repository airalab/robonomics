use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub trait WeightInfo {
    fn on_finalize() -> Weight;
    fn account_disable() -> Weight;
    fn account_add_with_role_and_data() -> Weight;
    fn account_set_with_role_and_data() -> Weight;
    fn token_mint_request_create_everusd() -> Weight;
    fn token_mint_request_revoke_everusd() -> Weight;
    fn token_mint_request_confirm_everusd() -> Weight;
    fn token_mint_request_decline_everusd() -> Weight;
    fn token_burn_request_create_everusd() -> Weight;
    fn token_burn_request_revoke_everusd() -> Weight;
    fn token_burn_request_confirm_everusd() -> Weight;
    fn token_burn_request_decline_everusd() -> Weight;
    fn bond_add_new() -> Weight;
    fn bond_set() -> Weight;
    fn bond_update() -> Weight;
    fn bond_release() -> Weight;
    fn bond_unit_package_buy() -> Weight;
    fn bond_unit_package_return() -> Weight;
    fn bond_withdraw() -> Weight;
    fn bond_activate() -> Weight;
    fn bond_impact_report_send() -> Weight;
    fn bond_impact_report_approve() -> Weight;
    fn bond_redeem() -> Weight;
    fn bond_declare_bankrupt() -> Weight;
    fn bond_accrue_coupon_yield() -> Weight;
    fn bond_revoke() -> Weight;
    fn bond_withdraw_everusd() -> Weight;
    fn bond_deposit_everusd() -> Weight;
    fn bond_unit_lot_bid() -> Weight;
    fn bond_unit_lot_settle() -> Weight;
}

#[allow(clippy::unnecessary_cast)]
impl WeightInfo for () {
    fn on_finalize() -> Weight {
        10000_u64 as Weight
    }

    fn account_disable() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(4_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn account_add_with_role_and_data() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(2_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn account_set_with_role_and_data() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn token_mint_request_create_everusd() -> Weight {
        (20000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn token_mint_request_revoke_everusd() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(2_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn token_mint_request_confirm_everusd() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(5_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn token_mint_request_decline_everusd() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(2_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn token_burn_request_create_everusd() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(4_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn token_burn_request_revoke_everusd() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(2_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn token_burn_request_confirm_everusd() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(2_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn token_burn_request_decline_everusd() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_add_new() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_set() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_update() -> Weight {
        (50000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(1_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_release() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_unit_package_buy() -> Weight {
        (1000000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(4_u64 as Weight))
            .saturating_add(DbWeight::get().writes(3_u64 as Weight))
    }
    fn bond_unit_package_return() -> Weight {
        (1000000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(4_u64 as Weight))
            .saturating_add(DbWeight::get().writes(3_u64 as Weight))
    }
    fn bond_withdraw() -> Weight {
        (1000000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(4_u64 as Weight))
            .saturating_add(DbWeight::get().writes(10_u64 as Weight))
    }
    fn bond_activate() -> Weight {
        (100000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(3_u64 as Weight))
    }
    fn bond_impact_report_send() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_impact_report_approve() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_redeem() -> Weight {
        (1000000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(2_u64 as Weight))
    }
    fn bond_declare_bankrupt() -> Weight {
        (1000000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_accrue_coupon_yield() -> Weight {
        (1000000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(14_u64 as Weight))
            .saturating_add(DbWeight::get().writes(13_u64 as Weight))
    }
    fn bond_revoke() -> Weight {
        (10000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_withdraw_everusd() -> Weight {
        (1000000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(2_u64 as Weight))
    }
    fn bond_deposit_everusd() -> Weight {
        (1000000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(3_u64 as Weight))
            .saturating_add(DbWeight::get().writes(2_u64 as Weight))
    }
    fn bond_unit_lot_bid() -> Weight {
        (20000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(4_u64 as Weight))
            .saturating_add(DbWeight::get().writes(1_u64 as Weight))
    }
    fn bond_unit_lot_settle() -> Weight {
        (20000_u64 as Weight)
            .saturating_add(DbWeight::get().reads(9_u64 as Weight))
            .saturating_add(DbWeight::get().writes(5_u64 as Weight))
    }
}
