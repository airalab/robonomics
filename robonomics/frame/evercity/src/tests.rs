use frame_support::{
    assert_noop, assert_ok, dispatch::DispatchResult, sp_std::ops::RangeInclusive, Blake2_256,
    StorageHasher,
};

use crate::bond::transfer_bond_units;
use crate::mock::*;
use crate::{
    BondId, BondImpactReportStruct, BondInnerStructOf, BondPeriodNumber, BondState, BondStructOf,
    BondUnitAmount, BondUnitPackage, BondUnitSaleLotStructOf, Error, EverUSDBalance, Module,
    AUDITOR_ROLE_MASK, DEFAULT_DAY_DURATION, ISSUER_ROLE_MASK, MASTER_ROLE_MASK,
};

type Evercity = Module<TestRuntime>;
type Timestamp = pallet_timestamp::Module<TestRuntime>;
type Moment = <TestRuntime as pallet_timestamp::Config>::Moment;
type BondInnerStruct = BondInnerStructOf<TestRuntime>;
type BondStruct = BondStructOf<TestRuntime>;
type RuntimeError = Error<TestRuntime>;
type AccountId = <TestRuntime as frame_system::Config>::AccountId;
type BondUnitSaleLotStruct = BondUnitSaleLotStructOf<TestRuntime>;

//////////////////////////////////////////////////////////////////////////////////////////////////////////
// Test uses pack of accounts, pre-set in new_test_ext in mock.rs:
// (1, EvercityAccountStruct { roles: MASTER,            identity: 10u64}), // MASTER    (accountId: 1)
// (2, EvercityAccountStruct { roles: CUSTODIAN,         identity: 20u64}), // CUSTODIAN (accountID: 2)
// (3, EvercityAccountStruct { roles: ISSUER,            identity: 30u64}), // ISSUER   (accountID: 3)
// (4, EvercityAccountStruct { roles: INVESTOR,          identity: 40u64}), // INVESTOR  (accountId: 4)
// (5, EvercityAccountStruct { roles: AUDITOR,           identity: 50u64}), // AUDITOR   (accountId: 5)
// (7, EvercityAccountStruct { roles: ISSUER | ISSUER,   identity: 70u64}), // ISSUER   (accountId: 5)
// (8, EvercityAccountStruct { roles: MANAGER,           identity: 80u64}), // MANAGER   (accountId: 8)
// (101+ : some external accounts
//////////////////////////////////////////////////////////////////////////////////////////////////////////

fn bond_current_period(bond: &BondStruct, now: Moment) -> u32 {
    bond.time_passed_after_activation(now).unwrap().1
}

/// Auxiliary function that replenish account balance
fn add_token(id: AccountId, amount: EverUSDBalance) -> DispatchResult {
    Evercity::token_mint_request_create_everusd(Origin::signed(id), amount)?;
    Evercity::token_mint_request_confirm_everusd(Origin::signed(CUSTODIAN_ID), id, amount)
}
/// Converts days into milliseconds
fn days2timestamp(days: u32) -> Moment {
    (days * DEFAULT_DAY_DURATION) as u64 * 1000_u64
}
/// Returns all accounts
fn iter_accounts() -> RangeInclusive<u64> {
    1_u64..=9
}

const CUSTODIAN_ID: u64 = 2;

#[test]
fn it_returns_true_for_correct_role_checks() {
    new_test_ext().execute_with(|| {
        assert_eq!(Evercity::account_is_master(&1), true);
        assert_eq!(Evercity::account_is_custodian(&2), true);
        assert_eq!(Evercity::account_is_issuer(&3), true);
        assert_eq!(Evercity::account_is_investor(&4), true);
        assert_eq!(Evercity::account_is_auditor(&5), true);
        assert_eq!(Evercity::account_is_manager(&8), true);
        assert_eq!(Evercity::account_is_issuer(&7), true);
        assert_eq!(Evercity::account_is_investor(&7), true);

        assert_eq!(Evercity::account_is_master(&100), false);
        assert_eq!(Evercity::account_is_custodian(&100), false);
        assert_eq!(Evercity::account_is_issuer(&100), false);
        assert_eq!(Evercity::account_is_investor(&100), false);
        assert_eq!(Evercity::account_is_auditor(&100), false);
        assert_eq!(Evercity::account_token_mint_burn_allowed(&100), false);
    });
}

#[test]
fn it_returns_false_for_incorrect_role_checks() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        //assert_ok!(AccountRegistry::insert(Origin::signed(1), EvercityAccountStruct {roles: 1u8, identity: 67u64}));
        // Read pallet storage and assert an expected result.
        assert_eq!(Evercity::account_is_auditor(&1), false);
        assert_eq!(Evercity::account_is_issuer(&2), false);
        assert_eq!(Evercity::account_is_investor(&3), false);
        assert_eq!(Evercity::account_is_custodian(&4), false);
        assert_eq!(Evercity::account_is_master(&5), false);
    });
}

#[test]
fn it_adds_new_account_with_correct_roles() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(12345);

        assert_ok!(Evercity::account_add_with_role_and_data(
            Origin::signed(1),
            101,
            MASTER_ROLE_MASK,
            88u64
        ));
        assert_eq!(Evercity::account_is_master(&101), true);
        assert_eq!(Evercity::account_is_investor(&101), false);

        assert_ok!(Evercity::account_add_with_role_and_data(
            Origin::signed(1),
            102,
            AUDITOR_ROLE_MASK,
            89u64
        ));
        assert_eq!(Evercity::account_is_master(&102), false);
        assert_eq!(Evercity::account_is_auditor(&102), true);
    });
}
#[test]
fn it_correctly_sets_new_role_to_existing_account() {
    new_test_ext().execute_with(|| {
        // add new role to existing account (allowed only for master)
        assert_eq!(Evercity::account_is_issuer(&3), true);
        assert_ok!(Evercity::account_set_with_role_and_data(
            Origin::signed(1),
            3,
            AUDITOR_ROLE_MASK,
            88u64
        ));
        assert_eq!(Evercity::account_is_issuer(&3), true);
        assert_eq!(Evercity::account_is_auditor(&3), true);
        assert_eq!(Evercity::account_is_investor(&3), false);

        assert_eq!(Evercity::account_is_custodian(&2), true);
        assert_eq!(Evercity::account_is_issuer(&2), false);
        assert_ok!(Evercity::account_set_with_role_and_data(
            Origin::signed(1),
            2,
            ISSUER_ROLE_MASK,
            89u64
        ));
        assert_eq!(Evercity::account_is_custodian(&2), true);
        assert_eq!(Evercity::account_is_issuer(&2), true);
    });
}

#[test]
fn it_disable_account() {
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::account_add_with_role_and_data(
            Origin::signed(1),
            101,
            MASTER_ROLE_MASK,
            88u64
        ));
        assert_eq!(Evercity::account_is_master(&101), true);
        assert_ok!(Evercity::account_disable(Origin::signed(1), 101));

        assert_eq!(Evercity::account_is_master(&101), false);
    });
}

#[test]
fn it_try_disable_yourself() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Evercity::account_disable(Origin::signed(1), 1),
            RuntimeError::InvalidAction
        );
        assert_noop!(
            Evercity::account_set_with_role_and_data(Origin::signed(1), 1, 0, 0),
            RuntimeError::InvalidAction
        );
    });
}

#[test]
fn it_denies_add_and_set_roles_for_non_master() {
    new_test_ext().execute_with(|| {
        // trying to add account form non-master account
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(12345);
        assert_noop!(
            Evercity::account_add_with_role_and_data(
                Origin::signed(2),
                101,
                MASTER_ROLE_MASK,
                88u64
            ),
            RuntimeError::AccountNotAuthorized
        );

        assert_noop!(
            Evercity::account_set_with_role_and_data(Origin::signed(2), 3, ISSUER_ROLE_MASK, 88u64),
            RuntimeError::AccountNotAuthorized
        );
    });
}

// mint tokens

#[test]
fn it_token_mint_create_with_confirm() {
    const ACCOUNT: u64 = 4; // INVESTOR
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT),
            100000
        ));
        assert_eq!(Evercity::total_supply(), 0);

        assert_ok!(Evercity::token_mint_request_confirm_everusd(
            Origin::signed(CUSTODIAN_ID),
            ACCOUNT,
            100000
        ));
        assert_eq!(Evercity::total_supply(), 100000);
    });
}

#[test]
fn it_token_mint_create_with_revoke() {
    const ACCOUNT: u64 = 4; // INVESTOR
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT), // INVESTOR
            100000
        ));

        assert_ok!(Evercity::token_mint_request_revoke_everusd(Origin::signed(
            ACCOUNT
        ),));

        assert_noop!(
            Evercity::token_mint_request_confirm_everusd(
                Origin::signed(CUSTODIAN_ID),
                ACCOUNT,
                100000
            ),
            RuntimeError::MintRequestDoesntExist
        );
    });
}

#[test]
fn it_token_mint_create_with_decline() {
    const ACCOUNT: u64 = 4; // INVESTOR
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT),
            100000
        ));

        assert_ok!(Evercity::token_mint_request_decline_everusd(
            Origin::signed(CUSTODIAN_ID),
            ACCOUNT
        ));

        assert_noop!(
            Evercity::token_mint_request_revoke_everusd(Origin::signed(ACCOUNT)),
            RuntimeError::MintRequestDoesntExist
        );
    });
}

#[test]
fn it_token_mint_create_denied() {
    const ACCOUNT: u64 = 5; // AUDITOR
    new_test_ext().execute_with(|| {
        assert_noop!(
            Evercity::token_mint_request_create_everusd(Origin::signed(ACCOUNT), 100000),
            RuntimeError::AccountNotAuthorized
        );
    });
}

#[test]
fn it_token_mint_create_hasty() {
    const ACCOUNT: u64 = 4; // INVESTOR
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT),
            100000
        ));

        assert_noop!(
            Evercity::token_mint_request_create_everusd(Origin::signed(ACCOUNT), 10),
            RuntimeError::MintRequestAlreadyExist
        );

        // make amend
        let ttl: u32 = <TestRuntime as crate::Config>::MintRequestTtl::get();
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(ttl.into());

        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT),
            10
        ));
    });
}

#[test]
fn it_token_mint_create_toolarge() {
    const ACCOUNT: u64 = 4;
    new_test_ext().execute_with(|| {
        assert_noop!(
            Evercity::token_mint_request_create_everusd(
                Origin::signed(ACCOUNT), // INVESTOR
                EVERUSD_MAX_MINT_AMOUNT + 1
            ),
            RuntimeError::MintRequestParamIncorrect
        );
    });
}

#[test]
fn it_token_burn_mint_overflow() {
    const ACCOUNT: u64 = 4;
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT),
            1000
        ));

        assert_ok!(Evercity::token_mint_request_confirm_everusd(
            Origin::signed(CUSTODIAN_ID),
            ACCOUNT,
            1000
        ));
        assert_noop!(
            Evercity::token_burn_request_create_everusd(
                Origin::signed(ACCOUNT),
                EverUSDBalance::MAX - 1000
            ),
            RuntimeError::BalanceOverdraft
        );
        // assert_noop!(
        //     Evercity::token_burn_request_confirm_everusd(
        //         Origin::signed(CUSTODIAN_ID),
        //         ACCOUNT,
        //         EverUSDBalance::MAX - 1000
        //     ),
        //     RuntimeError::BalanceOverdraft
        // );
    });
}

#[test]
fn it_token_mint_try_confirm_expired() {
    const ACCOUNT: u64 = 4;
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT), // INVESTOR
            1000
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(days2timestamp(10));
        assert_noop!(
            Evercity::token_mint_request_confirm_everusd(
                Origin::signed(CUSTODIAN_ID),
                ACCOUNT,
                1000
            ),
            RuntimeError::MintRequestObsolete
        );
    });
}

// burn tokens

#[test]
fn it_token_burn_create_with_confirm() {
    const ACCOUNT: u64 = 4; // INVESTOR

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(ACCOUNT, 10000));

        assert_ok!(Evercity::token_burn_request_create_everusd(
            Origin::signed(ACCOUNT),
            10000
        ));

        assert_eq!(Evercity::total_supply(), 10000);

        assert_ok!(Evercity::token_burn_request_confirm_everusd(
            Origin::signed(CUSTODIAN_ID),
            ACCOUNT,
            10000
        ));

        assert_eq!(Evercity::total_supply(), 0);
        // duplicate confirmations is not allowed
        assert_noop!(
            Evercity::token_burn_request_confirm_everusd(
                Origin::signed(CUSTODIAN_ID),
                ACCOUNT,
                10000
            ),
            RuntimeError::BurnRequestDoesntExist
        );
    });
}

#[test]
fn it_token_burn_create_overrun() {
    const ACCOUNT: u64 = 3; // ISSUER
    const BALANCE: EverUSDBalance = 10000;

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(ACCOUNT, BALANCE));

        assert_noop!(
            Evercity::token_burn_request_create_everusd(Origin::signed(ACCOUNT), BALANCE + 1),
            RuntimeError::BalanceOverdraft
        );
    });
}

#[test]
fn it_token_burn_create_with_revoke() {
    const ACCOUNT: u64 = 3; // ISSUER

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(ACCOUNT, 10000));

        assert_ok!(Evercity::token_burn_request_create_everusd(
            Origin::signed(ACCOUNT),
            10000
        ));

        assert_ok!(Evercity::token_burn_request_revoke_everusd(Origin::signed(
            ACCOUNT
        ),));

        assert_noop!(
            Evercity::token_burn_request_confirm_everusd(
                Origin::signed(CUSTODIAN_ID),
                ACCOUNT,
                10000
            ),
            RuntimeError::BurnRequestDoesntExist
        );
    });
}

#[test]
fn it_token_burn_try_confirm_expired() {
    const ACCOUNT: u64 = 4;
    new_test_ext().execute_with(|| {
        assert_ok!(add_token(ACCOUNT, 10000));
        assert_ok!(Evercity::token_burn_request_create_everusd(
            Origin::signed(ACCOUNT), // INVESTOR
            1000
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(days2timestamp(10));
        assert_noop!(
            Evercity::token_burn_request_confirm_everusd(
                Origin::signed(CUSTODIAN_ID),
                ACCOUNT,
                1000
            ),
            RuntimeError::BurnRequestObsolete
        );
    });
}

#[test]
fn it_token_burn_hasty() {
    const ACCOUNT: u64 = 4; // INVESTOR

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(ACCOUNT, 10000));

        assert_ok!(Evercity::token_burn_request_create_everusd(
            Origin::signed(ACCOUNT),
            5000
        ));
        assert_noop!(
            Evercity::token_burn_request_create_everusd(Origin::signed(ACCOUNT), 10000),
            RuntimeError::BurnRequestAlreadyExist
        );

        // make amend
        let ttl: u32 = <TestRuntime as crate::Config>::BurnRequestTtl::get();
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(ttl.into());

        assert_ok!(Evercity::token_burn_request_create_everusd(
            Origin::signed(ACCOUNT),
            10000
        ));
    })
}
// fuse

#[test]
fn fuse_is_blone() {
    new_test_ext().execute_with(|| {
        let fuse = Evercity::fuse();
        assert_eq!(fuse, true);

        assert_noop!(
            Evercity::set_master(Origin::signed(2),),
            RuntimeError::InvalidAction
        );
    })
}

#[test]
fn fuse_is_intact_on_bare_storage() {
    let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::default()
        .build_storage::<TestRuntime>()
        .unwrap()
        .into();

    ext.execute_with(|| {
        assert_eq!(Evercity::fuse(), false);

        assert_noop!(
            Evercity::account_add_with_role_and_data(Origin::signed(1), 101, MASTER_ROLE_MASK, 0),
            RuntimeError::AccountNotAuthorized
        );
        assert_ok!(Evercity::set_master(Origin::signed(1),));
        // make amend
        assert_ok!(Evercity::account_add_with_role_and_data(
            Origin::signed(1),
            101,
            MASTER_ROLE_MASK,
            0
        ));

        assert_eq!(Evercity::fuse(), true);
        assert_noop!(
            Evercity::set_master(Origin::signed(2),),
            RuntimeError::InvalidAction
        );
    });
}
// bonds

fn create_bond_unit_package(amount: Vec<BondUnitAmount>) -> Vec<BondUnitPackage> {
    amount
        .into_iter()
        .map(|bond_units| BondUnitPackage {
            bond_units,
            acquisition: 0,
            coupon_yield: 0,
        })
        .collect()
}

fn bond_unit_package_amount(package: Vec<BondUnitPackage>) -> Vec<BondUnitAmount> {
    package.into_iter().map(|item| item.bond_units).collect()
}

#[test]
fn bond_transfer_units() {
    new_test_ext().execute_with(|| {
        let mut from_package = create_bond_unit_package(vec![5, 2, 10, 1]);
        let mut to_package = create_bond_unit_package(vec![]);
        assert_ok!(transfer_bond_units::<TestRuntime>(
            &mut from_package,
            &mut to_package,
            3
        ));

        assert_eq!(bond_unit_package_amount(from_package), vec![10, 5]);
        assert_eq!(bond_unit_package_amount(to_package), vec![1, 2]);

        let mut from_package = create_bond_unit_package(vec![5, 2, 10, 1]);
        let mut to_package = create_bond_unit_package(vec![]);

        assert_ok!(transfer_bond_units::<TestRuntime>(
            &mut from_package,
            &mut to_package,
            10
        ));

        assert_eq!(bond_unit_package_amount(from_package), vec![8]);
        assert_eq!(bond_unit_package_amount(to_package), vec![1, 2, 5, 2]);

        let mut from_package = create_bond_unit_package(vec![5, 2, 10, 1]);
        let mut to_package = create_bond_unit_package(vec![]);

        assert_ok!(transfer_bond_units::<TestRuntime>(
            &mut from_package,
            &mut to_package,
            2
        ));

        assert_eq!(bond_unit_package_amount(from_package), vec![10, 5, 1]);
        assert_eq!(bond_unit_package_amount(to_package), vec![1, 1]);

        let mut from_package = create_bond_unit_package(vec![5, 2, 10, 1]);
        let mut to_package = create_bond_unit_package(vec![]);

        assert_noop!(
            transfer_bond_units::<TestRuntime>(&mut from_package, &mut to_package, 20),
            RuntimeError::BondParamIncorrect
        );
    });
}

#[test]
fn bond_validation() {
    new_test_ext().execute_with(|| {
        let bond = get_test_bond();
        assert_eq!(bond.inner.is_valid(DEFAULT_DAY_DURATION), true);
    });
}

#[test]
fn bond_check_equation() {
    new_test_ext().execute_with(|| {
        let bond1 = get_test_bond();

        let mut bond2 = bond1.clone();
        assert_eq!(bond1.inner, bond2.inner);
        bond2.inner.docs_pack_root_hash_legal = Blake2_256::hash(b"").into();

        assert!(bond1.inner.is_financial_options_eq(&bond2.inner));
        assert_ne!(bond1.inner, bond2.inner);

        bond2.inner.docs_pack_root_hash_legal = bond1.inner.docs_pack_root_hash_legal;
        bond2.inner.payment_period += 1;

        assert!(!bond1.inner.is_financial_options_eq(&bond2.inner));
        assert_ne!(bond1.inner, bond2.inner);
    });
}

#[test]
fn bond_interest_min_max() {
    new_test_ext().execute_with(|| {
        let bond = get_test_bond();
        let impact_base_value = bond.inner.impact_data_baseline[0];
        // full amplitude
        assert_eq!(
            bond.calc_effective_interest_rate(impact_base_value, impact_base_value),
            bond.inner.interest_rate_base_value
        );
        assert_eq!(
            bond.calc_effective_interest_rate(
                impact_base_value,
                bond.inner.impact_data_max_deviation_cap
            ),
            bond.inner.interest_rate_margin_floor
        );
        assert_eq!(
            bond.calc_effective_interest_rate(
                impact_base_value,
                bond.inner.impact_data_max_deviation_cap + 1
            ),
            bond.inner.interest_rate_margin_floor
        );
        assert_eq!(
            bond.calc_effective_interest_rate(
                impact_base_value,
                bond.inner.impact_data_max_deviation_floor
            ),
            bond.inner.interest_rate_margin_cap
        );
        assert_eq!(
            bond.calc_effective_interest_rate(
                impact_base_value,
                bond.inner.impact_data_max_deviation_floor - 1
            ),
            bond.inner.interest_rate_margin_cap
        );

        // partial amplitude
        assert_eq!(
            bond.calc_effective_interest_rate(impact_base_value, 25000_u64),
            1500
        );
        assert_eq!(
            bond.calc_effective_interest_rate(impact_base_value, 29000_u64),
            1100
        );

        assert_eq!(
            bond.calc_effective_interest_rate(impact_base_value, 17000_u64),
            3000
        );
        assert_eq!(
            bond.calc_effective_interest_rate(impact_base_value, 15000_u64),
            3666
        );
    });
}

#[test]
fn bond_period_interest_rate() {
    new_test_ext().execute_with(|| {
        let bond = get_test_bond();

        assert!(bond
            .inner
            .impact_data_baseline
            .iter()
            .all(|&v| v == 20000_u64));

        let mut reports = Vec::<BondImpactReportStruct>::new();
        //missing report
        reports.push(BondImpactReportStruct {
            create_period: 0,
            impact_data: 0,
            signed: false,
        });
        reports.push(BondImpactReportStruct {
            create_period: 0,
            impact_data: 20000_u64,
            signed: true,
        });
        //missing report
        reports.push(BondImpactReportStruct {
            create_period: 0,
            impact_data: 0,
            signed: false,
        });
        // worst result and maximal interest rate value
        reports.push(BondImpactReportStruct {
            create_period: 0,
            impact_data: 14000_u64,
            signed: true,
        });
        //missing report. it cannot make interest rate worse
        reports.push(BondImpactReportStruct {
            create_period: 0,
            impact_data: 0,
            signed: false,
        });
        // very good result lead to mininal interest rate
        reports.push(BondImpactReportStruct {
            create_period: 0,
            impact_data: 100000_u64,
            signed: true,
        });
        //first missing report.
        reports.push(BondImpactReportStruct {
            create_period: 0,
            impact_data: 0,
            signed: false,
        });
        //second missing report.
        reports.push(BondImpactReportStruct {
            create_period: 0,
            impact_data: 0,
            signed: false,
        });

        assert_eq!(
            bond.inner.interest_rate_start_period_value,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 0)
        );

        assert_eq!(
            bond.inner.interest_rate_start_period_value
                + bond.inner.interest_rate_penalty_for_missed_report,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 1)
        );

        assert_eq!(
            bond.inner.interest_rate_base_value,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 2)
        );

        assert_eq!(
            bond.inner.interest_rate_base_value
                + bond.inner.interest_rate_penalty_for_missed_report,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 3)
        );

        assert_eq!(
            bond.inner.interest_rate_margin_cap,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 4)
        );
        // missing report cannot increase insterested rate above maximal value
        assert_eq!(
            bond.inner.interest_rate_margin_cap,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 5)
        );

        assert_eq!(
            bond.inner.interest_rate_margin_floor,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 6)
        );

        assert_eq!(
            bond.inner.interest_rate_margin_floor
                + bond.inner.interest_rate_penalty_for_missed_report,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 7)
        );

        assert_eq!(
            bond.inner.interest_rate_margin_floor
                + 2 * bond.inner.interest_rate_penalty_for_missed_report,
            Evercity::calc_bond_interest_rate(&bond, reports.as_ref(), 8)
        );
    });
}

#[test]
fn bond_create_with_min_period() {
    let bondid1: BondId = "B1".into();
    const ACCOUNT: u64 = 3;

    new_test_ext().execute_with(|| {
        let mut bond = get_test_bond().inner;
        bond.bond_finishing_period = DEFAULT_DAY_DURATION;
        bond.payment_period = DEFAULT_DAY_DURATION;
        bond.start_period = DEFAULT_DAY_DURATION;
        bond.interest_pay_period = DEFAULT_DAY_DURATION;
        bond.impact_data_send_period = DEFAULT_DAY_DURATION;

        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid1,
            bond
        ));
    })
}

#[test]
fn bond_create_series() {
    let bond = get_test_bond();
    let bondid1: BondId = "B1".into();
    let bondid2: BondId = "B2".into();
    let bondid3: BondId = "B3".into();

    const ACCOUNT: u64 = 3;

    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid1,
            bond.inner.clone()
        ));
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid2,
            bond.inner.clone()
        ));
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid3,
            bond.inner.clone()
        ));
        assert_noop!(
            Evercity::bond_add_new(Origin::signed(ACCOUNT), bondid3, bond.inner.clone()),
            RuntimeError::BondAlreadyExists
        );
    });
}

#[test]
// unique case scenario
fn bond_buy_bond_uc() {
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const AUDITOR: u64 = 5;
    const INVESTOR1: u64 = 4;

    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 4_000_000_000_000_000));

        let mut bond = get_test_bond().inner;
        bond.mincap_deadline = 50_000;
        bond.bond_units_mincap_amount = 1000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond
        ));
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(50_000);
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            1000
        ));
        assert_ok!(Evercity::bond_set_auditor(
            Origin::signed(MASTER),
            bondid,
            AUDITOR
        ));
        assert_ok!(Evercity::bond_activate(Origin::signed(MASTER), bondid, 2));

        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.issued_amount, 1000);
        assert_eq!(Evercity::balance_everusd(&ACCOUNT), 4_000_000_000_000_000);
        assert_eq!(Evercity::balance_everusd(&INVESTOR1), 0);
    });
}

#[test]
fn bond_try_create_by_nonissuer() {
    let bond = get_test_bond();
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        for acc in iter_accounts().filter(|acc| !Evercity::account_is_issuer(acc)) {
            assert_noop!(
                Evercity::bond_add_new(Origin::signed(acc), bondid, bond.inner.clone()),
                RuntimeError::AccountNotAuthorized
            );
        }
    });
}

#[test]
fn bond_try_activate_without_release() {
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();

        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            get_test_bond().inner
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(days2timestamp(1));
        // try to buy some bonds in prepare state
        assert_noop!(
            Evercity::bond_unit_package_buy(Origin::signed(INVESTOR1), bondid, 0, 600),
            RuntimeError::BondStateNotPermitAction
        );
        // try to activate bond
        assert_noop!(
            Evercity::bond_activate(Origin::signed(MASTER), bondid, 0),
            RuntimeError::BondStateNotPermitAction
        );
    })
}

#[test]
fn bond_try_activate_by_nonmaster() {
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const AUDITOR: u64 = 5;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();

        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            get_test_bond().inner
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(days2timestamp(1));
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));
        // try to buy some bonds in prepare state
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            1000
        ));
        assert_ok!(Evercity::bond_set_auditor(
            Origin::signed(MASTER),
            bondid,
            AUDITOR
        ));
        // try to activate bond
        for acc in iter_accounts().filter(|acc| !Evercity::account_is_master(acc)) {
            assert_noop!(
                Evercity::bond_activate(Origin::signed(acc), bondid, 0),
                RuntimeError::AccountNotAuthorized
            );
        }
        // make amend
        assert_ok!(Evercity::bond_activate(Origin::signed(MASTER), bondid, 2));
    })
}

#[test]
fn bond_try_activate_without_auditor() {
    let mut bond = get_test_bond();
    let bondid: BondId = "BOND".into();
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const AUDITOR: u64 = 5;

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 50_000_000_000_000_000));

        bond.inner.mincap_deadline = 50000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner
        ));
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            1000
        ));
        assert_noop!(
            Evercity::bond_activate(Origin::signed(MASTER), bondid, 1),
            RuntimeError::BondIsNotConfigured
        );
        // make amends
        assert_ok!(Evercity::bond_set_auditor(
            Origin::signed(MASTER),
            bondid,
            AUDITOR
        ));
        assert_ok!(Evercity::bond_activate(Origin::signed(MASTER), bondid, 2));
    });
}

#[test]
fn bond_try_revoke_after_release() {
    const ACCOUNT: u64 = 3;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, get_test_bond().inner);

        assert_noop!(
            Evercity::bond_withdraw(Origin::signed(ACCOUNT), bondid),
            RuntimeError::BondStateNotPermitAction
        );
        assert_noop!(
            Evercity::bond_revoke(Origin::signed(ACCOUNT), bondid),
            RuntimeError::BondStateNotPermitAction
        );
    });
}

#[test]
fn bond_try_withdraw_before_deadline() {
    let mut bond = get_test_bond();
    let bondid: BondId = "BOND".into();
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 50_000_000_000_000_000));

        bond.inner.mincap_deadline = 50000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner
        ));
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            100
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(49000);
        assert_noop!(
            Evercity::bond_withdraw(Origin::signed(MASTER), bondid,),
            RuntimeError::BondStateNotPermitAction
        );
        // make amends
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(51000);
        assert_ok!(Evercity::bond_withdraw(Origin::signed(MASTER), bondid,));
        let chain_bond_item = Evercity::get_bond(&bondid);

        assert_eq!(chain_bond_item.state, BondState::PREPARE);
        assert_eq!(chain_bond_item.bond_credit, 0);
    });
}

#[test]
fn bond_try_withdraw_by_investor() {
    let mut bond = get_test_bond();
    let bondid: BondId = "BOND".into();
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 50_000_000_000_000_000));

        bond.inner.mincap_deadline = 50000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner
        ));
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            100
        ));

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(51000);
        assert_noop!(
            Evercity::bond_withdraw(Origin::signed(INVESTOR1), bondid,),
            RuntimeError::BondAccessDenied
        );

        // make amends
        assert_ok!(Evercity::bond_withdraw(Origin::signed(MASTER), bondid,));

        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.state, BondState::PREPARE);
        assert_eq!(chain_bond_item.issued_amount, 0);
        assert_eq!(chain_bond_item.bond_credit, 0);
        assert_eq!(chain_bond_item.bond_debit, 0);
        assert_eq!(
            Evercity::balance_everusd(&INVESTOR1),
            50_000_000_000_000_000
        );

        assert_eq!(Evercity::bond_packages(&bondid).is_empty(), true);
    });
}

#[test]
fn bond_try_manage_foreign_bond() {
    let mut bond = get_test_bond();
    let bondid: BondId = "BOND".into();
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const MANAGER: u64 = 8;

    new_test_ext().execute_with(|| {
        bond.inner.mincap_deadline = 50000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner
        ));

        let mut update = get_test_bond().inner;
        update.mincap_deadline = 60000;

        for acc in iter_accounts().filter(|acc| *acc != ACCOUNT) {
            assert_noop!(
                Evercity::bond_update(Origin::signed(acc), bondid, 0, update.clone()),
                RuntimeError::BondAccessDenied
            );
        }
        // make amend
        assert_ok!(Evercity::bond_set_manager(
            Origin::signed(MASTER),
            bondid,
            MANAGER
        ));
        assert_ok!(Evercity::bond_update(
            Origin::signed(MANAGER),
            bondid,
            1,
            update
        ),);
    });
}

#[test]
fn bond_try_update_after_release() {
    let mut bond = get_test_bond();
    let bondid: BondId = "BOND".into();
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const AUDITOR: u64 = 5;

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 50_000_000_000_000_000));

        bond.inner.mincap_deadline = 50000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner
        ));

        // release bond
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));

        // hashes can be changed
        let mut update = get_test_bond().inner;
        update.docs_pack_root_hash_finance = Blake2_256::hash(b"merkle tree hash").into();
        assert_ok!(Evercity::bond_update(
            Origin::signed(ACCOUNT),
            bondid,
            1,
            update
        ));

        // the others cannot. TODO add other fields to check
        let mut update = get_test_bond().inner;
        update.payment_period *= 2;
        assert_noop!(
            Evercity::bond_update(Origin::signed(ACCOUNT), bondid, 2, update),
            RuntimeError::BondStateNotPermitAction
        );
        let mut update = get_test_bond().inner;
        update.bond_units_base_price = 3_000_000_000_000;
        assert_noop!(
            Evercity::bond_update(Origin::signed(ACCOUNT), bondid, 2, update),
            RuntimeError::BondStateNotPermitAction
        );
        let mut update = get_test_bond().inner;
        update.impact_data_baseline[0] += 1;
        assert_noop!(
            Evercity::bond_update(Origin::signed(ACCOUNT), bondid, 2, update),
            RuntimeError::BondStateNotPermitAction
        );

        // buy bonds
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            2,
            1200
        ));
        assert_ok!(Evercity::bond_set_auditor(
            Origin::signed(MASTER),
            bondid,
            AUDITOR
        ));
        // activate
        assert_ok!(Evercity::bond_activate(Origin::signed(MASTER), bondid, 3));

        let mut update = get_test_bond().inner;
        update.docs_pack_root_hash_finance = Blake2_256::hash(b"merkle tree hash").into();
        // try change after activation
        assert_noop!(
            Evercity::bond_update(Origin::signed(ACCOUNT), bondid, 4, update),
            RuntimeError::BondStateNotPermitAction
        );
    });
}

#[test]
fn bond_try_activate_insufficient_fund_raising() {
    let mut bond = get_test_bond();
    let bondid: BondId = "BOND".into();
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const AUDITOR: u64 = 5;

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 50_000_000_000_000_000));

        bond.inner.mincap_deadline = 50000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner
        ));

        // try activate before been issued
        assert_noop!(
            Evercity::bond_activate(Origin::signed(MASTER), bondid, 0),
            RuntimeError::BondStateNotPermitAction
        );
        // try buy bonds before been issued
        assert_noop!(
            Evercity::bond_unit_package_buy(Origin::signed(INVESTOR1), bondid, 0, 100),
            RuntimeError::BondStateNotPermitAction
        );
        // release bond
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));
        // buy limited number of bonds
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            100
        ));
        assert_ok!(Evercity::bond_set_auditor(
            Origin::signed(MASTER),
            bondid,
            AUDITOR
        ));

        assert_noop!(
            Evercity::bond_activate(Origin::signed(MASTER), bondid, 2),
            RuntimeError::BondParamIncorrect
        );
        // make amends
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            2,
            900
        ));
        assert_ok!(Evercity::bond_activate(Origin::signed(MASTER), bondid, 2));
    });
}

#[test]
fn bond_try_activate_expired_fund_raising() {
    let mut bond = get_test_bond();
    let bondid: BondId = "BOND".into();
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const AUDITOR: u64 = 5;

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 50_000_000_000_000_000));
        assert!(bond.inner.mincap_deadline < days2timestamp(21));

        bond.inner.mincap_deadline = 50000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner
        ));

        // release bond
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));
        // buy limited number of bonds
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            100
        ));
        assert_ok!(Evercity::bond_set_auditor(
            Origin::signed(MASTER),
            bondid,
            AUDITOR
        ));

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(days2timestamp(21));

        assert_noop!(
            Evercity::bond_activate(Origin::signed(MASTER), bondid, 2),
            RuntimeError::BondParamIncorrect
        );
        // workaround
        assert_ok!(Evercity::bond_withdraw(Origin::signed(ACCOUNT), bondid));
        assert_eq!(Evercity::bond_packages(&bondid).is_empty(), true);
    });
}

#[test]
fn bond_try_create_with_overflow() {
    const ACCOUNT: u64 = 3;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        let mut bond = get_test_bond().inner;
        bond.bond_units_maxcap_amount = BondUnitAmount::MAX - 1;

        assert_noop!(
            Evercity::bond_add_new(Origin::signed(ACCOUNT), bondid, bond),
            RuntimeError::BondParamIncorrect
        );
    });
}

#[test]
fn bond_try_buy_unit_with_overflow() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        let amount = bond.bond_units_maxcap_amount;
        bond_release(bondid, ACCOUNT, bond);

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(100_000);
        assert_noop!(
            Evercity::bond_unit_package_buy(
                Origin::signed(INVESTOR1),
                bondid,
                2,
                BondUnitAmount::MAX - amount
            ),
            RuntimeError::BondParamIncorrect
        );
    });
}

#[test]
fn bond_calc_coupon_yield_basic() {
    const ACCOUNT: u64 = 3;
    let bondid: BondId = "BOND2".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, get_test_bond().inner);

        let mut chain_bond_item = Evercity::get_bond(&bondid);

        assert_eq!(chain_bond_item.active_start_date, 30000);
        // pass first (index=0) period
        let mut moment: Moment =
            30000_u64 + (chain_bond_item.inner.start_period) as u64 * 1000_u64 + 1_u64;
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(moment);

        assert_eq!(bond_current_period(&chain_bond_item, moment), 1);
        assert!(
            Evercity::calc_and_store_bond_coupon_yield(&bondid, &mut chain_bond_item, moment) > 0
        );
        // second call should return false
        assert!(
            !Evercity::calc_and_store_bond_coupon_yield(&bondid, &mut chain_bond_item, moment) > 0
        );

        // pass second (index=1) period
        moment += chain_bond_item.inner.payment_period as u64 * 1000_u64;
        assert_eq!(bond_current_period(&chain_bond_item, moment), 2);
        chain_bond_item.bond_debit = 2000;

        assert!(
            Evercity::calc_and_store_bond_coupon_yield(&bondid, &mut chain_bond_item, moment) > 0
        );

        let bond_yields = Evercity::get_coupon_yields(&bondid);

        assert_eq!(bond_yields.len(), 2);
        assert_eq!(
            bond_yields[0].interest_rate,
            chain_bond_item.inner.interest_rate_start_period_value
        );
        assert_eq!(bond_yields[0].total_yield, 29_983_561_643_520);

        assert_eq!(
            bond_yields[1].interest_rate,
            chain_bond_item.inner.interest_rate_start_period_value
                + chain_bond_item
                    .inner
                    .interest_rate_penalty_for_missed_report
        );
        assert_eq!(bond_yields[1].total_yield, 39_057_534_246_240);
    });
}

#[test]
fn bond_calc_coupon_yield_advanced() {
    const ACCOUNT1: u64 = 3;
    const ACCOUNT2: u64 = 7;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;
    let bondid1: BondId = "BOND1".into();
    let bondid2: BondId = "BOND2".into();

    //         |-  investor1 600 presale + 400 after 100 days ( during start period ),
    // bond1---|
    //         |-  investor2 600 presale

    //         |-  investor1 600 presale
    // bond2---|
    //         |-  investor2 600 presale + 400 after 140 days (second period )

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid1, ACCOUNT1, get_test_bond().inner);
        bond_activate(bondid2, ACCOUNT2, get_test_bond().inner);

        let chain_bond_item1 = Evercity::get_bond(&bondid1);
        let chain_bond_item2 = Evercity::get_bond(&bondid2);

        let start_moment = chain_bond_item1.active_start_date;
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            start_moment + (100 * DEFAULT_DAY_DURATION) as u64 * 1000,
        );

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid1,
            3,
            400
        ));

        assert_eq!(
            bond_current_period(
                &chain_bond_item1,
                start_moment + (100 * DEFAULT_DAY_DURATION) as u64 * 1000
            ),
            0
        );

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            start_moment + (140 * DEFAULT_DAY_DURATION) as u64 * 1000,
        );
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR2),
            bondid2,
            3,
            400
        ));

        assert_eq!(
            bond_current_period(
                &chain_bond_item2,
                start_moment + (140 * DEFAULT_DAY_DURATION) as u64 * 1000
            ),
            1
        );

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            start_moment + (160 * DEFAULT_DAY_DURATION) as u64 * 1000,
        );

        assert_eq!(
            bond_current_period(
                &chain_bond_item2,
                start_moment + (160 * DEFAULT_DAY_DURATION) as u64 * 1000
            ),
            2
        );

        let _investor1_balance = Evercity::balance_everusd(&INVESTOR1);
        let _investor2_balance = Evercity::balance_everusd(&INVESTOR2);
        // set impact data
        assert_ok!(Evercity::set_impact_data(
            &bondid1,
            0,
            chain_bond_item1.inner.impact_data_baseline[0]
        ));
        //Evercity::set_impact_data(&bondid1, 1, chain_bond_item1.inner.impact_data_baseline );
        assert_ok!(Evercity::set_impact_data(
            &bondid2,
            0,
            chain_bond_item2.inner.impact_data_baseline[0]
        ));

        // request coupon yield
    });
}

#[test]
fn bond_try_create_arbitrary_period() {
    let bondid: BondId = "BOND".into();
    const ACCOUNT: u64 = 3;

    new_test_ext().execute_with(|| {
        let mut bond = get_test_bond();
        bond.inner.start_period += 1;
        assert_noop!(
            Evercity::bond_add_new(Origin::signed(ACCOUNT), bondid, bond.inner),
            RuntimeError::BondParamIncorrect
        );

        bond = get_test_bond();
        bond.inner.payment_period += 1;
        assert_noop!(
            Evercity::bond_add_new(Origin::signed(ACCOUNT), bondid, bond.inner),
            RuntimeError::BondParamIncorrect
        );

        bond = get_test_bond();
        bond.inner.bond_finishing_period += 1;
        assert_noop!(
            Evercity::bond_add_new(Origin::signed(ACCOUNT), bondid, bond.inner),
            RuntimeError::BondParamIncorrect
        );

        bond = get_test_bond();
        bond.inner.interest_pay_period += 1;
        assert_noop!(
            Evercity::bond_add_new(Origin::signed(ACCOUNT), bondid, bond.inner),
            RuntimeError::BondParamIncorrect
        );

        bond = get_test_bond();
        bond.inner.impact_data_send_period += 1;
        assert_noop!(
            Evercity::bond_add_new(Origin::signed(ACCOUNT), bondid, bond.inner),
            RuntimeError::BondParamIncorrect
        );
    });
}

#[test]
fn bond_try_release_without_fundraising_period() {
    let bondid: BondId = "BOND".into();
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;

    new_test_ext().execute_with(|| {
        let mut bond = get_test_bond();
        bond.inner.mincap_deadline = 100000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(100000);
        assert_noop!(
            Evercity::bond_release(Origin::signed(MASTER), bondid, 0),
            RuntimeError::BondStateNotPermitAction
        );

        bond = get_test_bond();
        bond.inner.mincap_deadline = 200000;
        assert_ok!(Evercity::bond_update(
            Origin::signed(ACCOUNT),
            bondid,
            0,
            bond.inner
        ));
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 1));
    });
}

#[test]
fn bond_calc_redeemed_yield() {
    // YMT - yield to maturity - total coupon yield after bond redemption
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;

    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        assert!(Evercity::evercity_balance().is_ok());
        let initial_balace1 = Evercity::balance_everusd(&INVESTOR1);
        let initial_balace2 = Evercity::balance_everusd(&INVESTOR2);
        bond_activate(bondid, ACCOUNT, get_test_bond().inner);
        assert!(Evercity::evercity_balance().is_ok());

        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.active_start_date, 30000);
        assert_eq!(chain_bond_item.issued_amount, 1200);
        assert!(chain_bond_item
            .inner
            .impact_data_baseline
            .iter()
            .all(|&v| v == 20000_u64));

        let num_periods = chain_bond_item.get_periods();
        // all period except start period will have interest rate = interest_rate_base_value
        // for start period interest rate will be  interest_rate_start_period_value
        for period in 0..num_periods - 1 {
            assert_ok!(Evercity::set_impact_data(
                &bondid,
                period,
                chain_bond_item.inner.impact_data_baseline[period as usize]
            ));
        }
        // go to the last period
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date
                + days2timestamp(120 + chain_bond_item.inner.bond_duration * 30 + 1),
        );
        // add extra everusd to pay off coupon yield
        assert_ok!(add_token(ACCOUNT, 125_000_000_000_000));
        assert!(Evercity::evercity_balance().is_ok());

        assert_ok!(Evercity::bond_redeem(Origin::signed(ACCOUNT), bondid));
        assert!(Evercity::bond_check_invariant(&bondid));
        // withdraw coupon & principal value
        assert_ok!(Evercity::bond_withdraw_everusd(
            Origin::signed(INVESTOR1),
            bondid
        ));
        assert_ok!(Evercity::bond_withdraw_everusd(
            Origin::signed(INVESTOR2),
            bondid
        ));

        assert!(Evercity::evercity_balance().is_ok());

        let chain_bond_item = Evercity::get_bond(&bondid);
        let yield1 = Evercity::balance_everusd(&INVESTOR1) - initial_balace1;
        let yield2 = Evercity::balance_everusd(&INVESTOR2) - initial_balace2;

        assert_eq!(
            yield1 + yield2 + Evercity::balance_everusd(&ACCOUNT),
            125_000_000_000_000
        );
        assert_eq!(yield1, yield2);
        assert_eq!(yield1, 62_334_246_574_800);
        assert_eq!(Evercity::balance_everusd(&ACCOUNT), 331_506_850_400);

        assert_eq!(chain_bond_item.state, BondState::FINISHED);
        // @TODO descrees credit on redemption
        //assert_eq!(chain_bond_item.bond_credit, 0);
        //assert_eq!(chain_bond_item.bond_debit, 0);
    });
}

#[test]
fn bond_try_redeem_prior_maturity() {
    const MASTER: u64 = 1;
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;

    let bondid1: BondId = "BOND1".into();
    let bondid2: BondId = "BOND2".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        // bond before activation
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid1,
            get_test_bond().inner
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(days2timestamp(1));
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid1, 0));

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid1,
            1,
            600
        ));
        assert_noop!(
            Evercity::bond_redeem(Origin::signed(ACCOUNT), bondid1),
            RuntimeError::BondStateNotPermitAction
        );

        // active bond
        bond_activate(bondid2, ACCOUNT, get_test_bond().inner);

        // go to the end of the first period. n
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(days2timestamp(120 + 1));

        assert_ok!(add_token(ACCOUNT, 200_000_000_000_000));
        assert_noop!(
            Evercity::bond_redeem(Origin::signed(ACCOUNT), bondid2),
            RuntimeError::BondOutOfOrder
        );
    })
}

#[test]
fn bond_send_impact_reports() {
    const ACCOUNT: u64 = 3;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, get_test_bond().inner);
    });
}

#[test]
fn bond_periods() {
    let mut bond = get_test_bond();
    bond.state = BondState::ACTIVE;
    bond.active_start_date += 10;

    assert_eq!(bond.time_passed_after_activation(0), None);
    assert_eq!(
        bond.time_passed_after_activation(bond.active_start_date),
        Some((0, 0))
    );
    let start_period = bond.active_start_date + 120 * 1000 * DEFAULT_DAY_DURATION as u64;
    assert_eq!(bond.inner.start_period, 120 * DEFAULT_DAY_DURATION);

    assert_eq!(
        bond.time_passed_after_activation(start_period),
        Some((120 * DEFAULT_DAY_DURATION, 1))
    );
    assert_eq!(
        bond.time_passed_after_activation(start_period - 1),
        Some((120 * DEFAULT_DAY_DURATION - 1, 0))
    );

    assert_eq!(bond.inner.payment_period, 30 * DEFAULT_DAY_DURATION);
    assert_eq!(
        bond.time_passed_after_activation(start_period + 30 * 1000 * DEFAULT_DAY_DURATION as u64),
        Some(((120 + 30) * DEFAULT_DAY_DURATION, 2))
    );
    assert_eq!(
        bond.time_passed_after_activation(start_period + 29 * 1000 * DEFAULT_DAY_DURATION as u64),
        Some(((120 + 29) * DEFAULT_DAY_DURATION, 1))
    );
    assert_eq!(
        bond.time_passed_after_activation(start_period + 1000 * DEFAULT_DAY_DURATION as u64),
        Some(((120 + 1) * DEFAULT_DAY_DURATION, 1))
    );
    assert_eq!(
        bond.time_passed_after_activation(start_period + 31 * 1000 * DEFAULT_DAY_DURATION as u64),
        Some(((31 + 120) * DEFAULT_DAY_DURATION, 2))
    );
    assert_eq!(
        bond.time_passed_after_activation(start_period + 310 * 1000 * DEFAULT_DAY_DURATION as u64),
        Some(((120 + 310) * DEFAULT_DAY_DURATION, 11))
    );

    assert_eq!(
        bond.time_passed_after_activation(4294967295000),
        Some((4294967294, 13))
    );

    assert_eq!(bond.time_passed_after_activation(6300000000000), None);
}

#[test]
fn bond_try_create_with_same_id() {
    let bond = get_test_bond();
    let bondid: BondId = "TEST".into();
    const ACCOUNT: u64 = 3;

    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner.clone()
        ));
        assert_noop!(
            Evercity::bond_add_new(Origin::signed(ACCOUNT), bondid, bond.inner.clone()),
            RuntimeError::BondAlreadyExists
        );
        assert_ok!(Evercity::bond_revoke(Origin::signed(ACCOUNT), bondid));
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner.clone()
        ));
    });
}

#[test]
fn bond_create_delete() {
    let bond = get_test_bond();
    let bondid: BondId = "TEST".into();

    const ACCOUNT: u64 = 3;
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner.clone()
        ));
        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(bond.inner, chain_bond_item.inner);

        assert_ok!(Evercity::bond_revoke(Origin::signed(ACCOUNT), bondid));
        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_ne!(bond.inner, chain_bond_item.inner);
        assert_eq!(chain_bond_item.inner, Default::default());
    });
}

fn bond_grand_everusd() {
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;

    assert_ok!(add_token(INVESTOR1, 50_000_000_000_000_000));
    assert_ok!(add_token(INVESTOR2, 50_000_000_000_000_000));
}

fn bond_release(bondid: BondId, acc: u64, mut bond: BondInnerStruct) -> BondStruct {
    const MASTER: u64 = 1;
    const AUDITOR: u64 = 5;
    bond.mincap_deadline = 50000;
    assert_ok!(Evercity::bond_add_new(Origin::signed(acc), bondid, bond));
    <pallet_timestamp::Module<TestRuntime>>::set_timestamp(10_000);
    assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));
    assert_ok!(Evercity::bond_set_auditor(
        Origin::signed(MASTER),
        bondid,
        AUDITOR
    ));
    Evercity::get_bond(&bondid)
}

fn bond_activate(bondid: BondId, acc: u64, mut bond: BondInnerStruct) {
    const MASTER: u64 = 1;
    const AUDITOR: u64 = 5;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;

    let investor1_balance = Evercity::balance_everusd(&INVESTOR1);
    let investor2_balance = Evercity::balance_everusd(&INVESTOR2);

    bond.mincap_deadline = 50000;
    assert_ok!(Evercity::bond_add_new(Origin::signed(acc), bondid, bond));
    <pallet_timestamp::Module<TestRuntime>>::set_timestamp(10_000);
    assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));
    let chain_bond_item = Evercity::get_bond(&bondid);
    assert_eq!(chain_bond_item.issued_amount, 0);

    // Buy two packages
    assert_ok!(Evercity::bond_unit_package_buy(
        Origin::signed(INVESTOR1),
        bondid,
        1,
        600
    ));

    assert!(Evercity::bond_check_invariant(&bondid));

    <pallet_timestamp::Module<TestRuntime>>::set_timestamp(20_000);
    assert_ok!(Evercity::bond_unit_package_buy(
        Origin::signed(INVESTOR2),
        bondid,
        1,
        600
    ));

    assert!(Evercity::bond_check_invariant(&bondid));

    let chain_bond_item = Evercity::get_bond(&bondid);
    assert_eq!(chain_bond_item.issued_amount, 1200);
    assert_eq!(chain_bond_item.bond_debit, 1200 * 4_000_000_000_000);
    assert_eq!(chain_bond_item.bond_debit, chain_bond_item.bond_credit);

    assert_ok!(Evercity::bond_set_auditor(
        Origin::signed(MASTER),
        bondid,
        AUDITOR
    ));

    // Activate bond
    <pallet_timestamp::Module<TestRuntime>>::set_timestamp(30000);
    assert_ok!(Evercity::bond_activate(
        Origin::signed(MASTER),
        bondid,
        chain_bond_item.nonce + 1
    ));
    let chain_bond_item = Evercity::get_bond(&bondid);

    assert_eq!(chain_bond_item.issued_amount, 1200);
    assert_eq!(chain_bond_item.bond_debit, 0);
    assert_eq!(chain_bond_item.bond_credit, 0);

    assert_eq!(Evercity::balance_everusd(&acc), 1200 * 4_000_000_000_000);

    assert_eq!(
        investor1_balance - Evercity::balance_everusd(&INVESTOR1),
        600 * 4_000_000_000_000
    );
    assert_eq!(
        investor2_balance - Evercity::balance_everusd(&INVESTOR2),
        600 * 4_000_000_000_000
    );
    // Try revoke
    assert_noop!(
        Evercity::bond_revoke(Origin::signed(acc), bondid),
        RuntimeError::BondStateNotPermitAction
    );
    // Try give back
    assert_noop!(
        Evercity::bond_unit_package_return(Origin::signed(INVESTOR1), bondid, 600),
        RuntimeError::BondStateNotPermitAction
    );
}

#[test]
fn bond_create_release_update() {
    let bond = get_test_bond();
    let bondid: BondId = "TEST".into();

    const ACCOUNT: u64 = 3;
    const MASTER: u64 = 1;
    const MANAGER: u64 = 8;
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond.inner.clone()
        ));
        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.state, BondState::PREPARE);

        // set Manager
        assert_noop!(
            Evercity::bond_set_manager(Origin::signed(ACCOUNT), bondid, MANAGER),
            RuntimeError::AccountNotAuthorized
        );
        assert_ok!(Evercity::bond_set_manager(
            Origin::signed(MASTER),
            bondid,
            MANAGER
        ));
        // Manager can change bond_units_base_price
        let mut new_bond = bond.inner.clone();
        new_bond.bond_units_base_price = 100000;
        assert_ok!(Evercity::bond_update(
            Origin::signed(MANAGER),
            bondid,
            1,
            new_bond
        ));

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(10_000);

        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 2));
        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.state, BondState::BOOKING);
        assert_eq!(chain_bond_item.booking_start_date, 10_000);
        assert_eq!(chain_bond_item.manager, MANAGER);
        assert_eq!(chain_bond_item.inner.bond_units_base_price, 100_000);
    });
}

#[test]
fn bond_activate_bond_and_withdraw_bondfund() {
    const ACCOUNT: u64 = 3;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, get_test_bond().inner);
        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.state, BondState::ACTIVE);
        assert_eq!(chain_bond_item.active_start_date, 30000);
        assert_eq!(chain_bond_item.bond_debit, 0);
        assert_eq!(chain_bond_item.bond_credit, 0);

        assert_eq!(
            Evercity::balance_everusd(&ACCOUNT),
            1200 * 4_000_000_000_000
        );

        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.bond_debit, 0);
        assert_eq!(
            Evercity::balance_everusd(&ACCOUNT),
            1200 * 4_000_000_000_000
        );
        assert_eq!(Evercity::bond_packages(&bondid).is_empty(), false);
        let acquired_bond_units: BondUnitAmount = Evercity::bond_packages(&bondid)
            .iter()
            .map(|(_, packages)| {
                packages
                    .iter()
                    .map(|package| package.bond_units)
                    .sum::<BondUnitAmount>()
            })
            .sum::<BondUnitAmount>();

        assert_eq!(acquired_bond_units, 1200);
    });
}

#[test]
fn bond_buy_bond_units_after_activation() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, get_test_bond().inner);
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(600_000);
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            3,
            400
        ));

        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(
            Evercity::balance_everusd(&ACCOUNT),
            1600 * 4_000_000_000_000
        ); // (600 + 600 + 400) * 4000
        assert_eq!(chain_bond_item.bond_debit, 0);
        assert_eq!(bond_current_period(&chain_bond_item, 600_000), 0);
    });
}

#[test]
fn bond_try_return_foreign_bonds() {
    const ACCOUNT1: u64 = 3;
    const ACCOUNT2: u64 = 7;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;
    let bondid1: BondId = "BOND1".into();
    let bondid2: BondId = "BOND2".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_release(bondid1, ACCOUNT1, get_test_bond().inner);
        bond_release(bondid2, ACCOUNT2, get_test_bond().inner);

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(600_000);
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid1,
            2,
            400
        ));
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR2),
            bondid2,
            2,
            400
        ));

        // bond1 != bond2
        assert_noop!(
            Evercity::bond_unit_package_return(Origin::signed(INVESTOR1), bondid2, 400),
            RuntimeError::BondParamIncorrect
        );

        // make amend
        assert_ok!(Evercity::bond_unit_package_return(
            Origin::signed(INVESTOR1),
            bondid1,
            400
        ));
        assert_ok!(Evercity::bond_unit_package_return(
            Origin::signed(INVESTOR2),
            bondid2,
            400
        ));
    });
}

#[test]
fn bond_return_bondunit_package() {
    const ACCOUNT: u64 = 3;
    const MASTER: u64 = 1;

    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;

    let bondid: BondId = "BOND0".into();

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 6_000_000_000_000_000));
        assert_ok!(add_token(INVESTOR2, 6_000_000_000_000_000));

        let mut bond = get_test_bond().inner;
        bond.mincap_deadline = 50000;

        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(10000);
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));
        assert!(Evercity::evercity_balance().is_ok());

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            600
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(20000);
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR2),
            bondid,
            1,
            600
        ));
        assert!(Evercity::evercity_balance().is_ok());

        let packages1 = Evercity::bond_holder_packages(&bondid, &INVESTOR1);
        assert_eq!(packages1.len(), 1);
        assert_eq!(packages1[0].bond_units, 600);
        assert_ok!(Evercity::bond_unit_package_return(
            Origin::signed(INVESTOR1),
            bondid,
            600
        ));

        let packages1 = Evercity::bond_holder_packages(&bondid, &INVESTOR1);
        assert_eq!(packages1.len(), 0);
        // you cannot give back part of the package
        assert_noop!(
            Evercity::bond_unit_package_return(Origin::signed(INVESTOR2), bondid, 100),
            RuntimeError::BondParamIncorrect
        );
        let packages2 = Evercity::bond_holder_packages(&bondid, &INVESTOR2);
        assert_eq!(packages2.len(), 1);
        assert!(Evercity::evercity_balance().is_ok());
        assert!(Evercity::bond_check_invariant(&bondid));
    });
}

#[test]
fn bond_return_partial_bondunit_package() {
    const ACCOUNT: u64 = 3;
    const MASTER: u64 = 1;

    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;

    let bondid: BondId = "BOND0".into();
    // investor1 = 100 + 100 + 200
    //    return 200 + 200
    // investor2 = 200
    //    return 200

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 6_000_000_000_000_000));
        assert_ok!(add_token(INVESTOR2, 6_000_000_000_000_000));

        let mut bond = get_test_bond().inner;
        bond.mincap_deadline = 50000;

        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond
        ));

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(20000);
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));
        assert!(Evercity::evercity_balance().is_ok());

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            200
        ));
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            100
        ));
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            100
        ));

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR2),
            bondid,
            1,
            200
        ));
        assert!(Evercity::bond_check_invariant(&bondid));

        assert_ok!(Evercity::bond_unit_package_return(
            Origin::signed(INVESTOR1),
            bondid,
            200
        ));
        assert_ok!(Evercity::bond_unit_package_return(
            Origin::signed(INVESTOR1),
            bondid,
            200
        ));
        assert!(Evercity::bond_check_invariant(&bondid));
        assert_noop!(
            Evercity::bond_unit_package_return(Origin::signed(INVESTOR2), bondid, 100),
            RuntimeError::BondParamIncorrect
        );

        assert_ok!(Evercity::bond_unit_package_return(
            Origin::signed(INVESTOR2),
            bondid,
            200
        ));
        assert!(Evercity::bond_check_invariant(&bondid));
    });
}

#[test]
fn bond_iter_periods() {
    const ACCOUNT: u64 = 3;
    let bondid: BondId = "BOND1".into();

    let mut ext = new_test_ext();
    ext.execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, get_test_bond().inner);
        let chain_bond_item = Evercity::get_bond(&bondid);

        let mut start = 0;
        let mut count = 0;
        for period in chain_bond_item.iter_periods() {
            assert_eq!(period.start_period, start);
            start = period.payment_period;
            count += 1;
        }

        assert_eq!(count, 14);
        assert_eq!(chain_bond_item.get_periods(), count - 1);
    });
}

#[test]
fn bond_cancel_after_release() {
    const ACCOUNT: u64 = 3;
    const MASTER: u64 = 1;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(INVESTOR1, 10_000_000_000_000_000));
        assert_ok!(add_token(INVESTOR2, 10_000_000_000_000_000));

        let mut bond = get_test_bond().inner;
        bond.mincap_deadline = 50000;
        assert_ok!(Evercity::bond_add_new(
            Origin::signed(ACCOUNT),
            bondid,
            bond
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(10000);
        assert_ok!(Evercity::bond_release(Origin::signed(MASTER), bondid, 0));

        // Buy three packages
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            1,
            400
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(20_000);
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR2),
            bondid,
            1,
            200
        ));
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(30_000);
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR2),
            bondid,
            1,
            200
        ));

        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.issued_amount, 800);
        assert_eq!(chain_bond_item.bond_debit, 800 * 4_000_000_000_000);
        assert_eq!(chain_bond_item.bond_debit, chain_bond_item.bond_credit);

        assert_eq!(
            Evercity::balance_everusd(&INVESTOR1),
            10_000_000_000_000_000 - 400 * 4_000_000_000_000
        );
        assert_eq!(
            Evercity::balance_everusd(&INVESTOR2),
            10_000_000_000_000_000 - 400 * 4_000_000_000_000
        );

        // Bond unit packages

        let packages1 = Evercity::bond_holder_packages(&bondid, &INVESTOR1);
        let packages2 = Evercity::bond_holder_packages(&bondid, &INVESTOR2);

        assert_eq!(packages1.len(), 1);
        assert_eq!(packages2.len(), 2);

        assert_eq!(packages1[0].bond_units, 400);
        assert_eq!(packages2[0].bond_units, 200);
        assert_eq!(packages2[0].bond_units, 200);

        assert_eq!(packages1[0].acquisition, 0);
        assert_eq!(packages2[0].acquisition, 0);
        assert_eq!(packages2[1].acquisition, 0);

        // We raised up less than  bond_units_mincap_amount, so we should revoke the bond
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(60000);
        assert_ok!(Evercity::bond_withdraw(Origin::signed(MASTER), bondid));
        let chain_bond_item = Evercity::get_bond(&bondid);

        assert_eq!(chain_bond_item.issued_amount, 0);
        assert_eq!(chain_bond_item.state, BondState::PREPARE);
        assert_eq!(chain_bond_item.bond_debit, 0);
        assert_eq!(chain_bond_item.bond_credit, 0);

        assert_eq!(
            Evercity::balance_everusd(&INVESTOR1),
            10_000_000_000_000_000
        );
        assert_eq!(
            Evercity::balance_everusd(&INVESTOR2),
            10_000_000_000_000_000
        );

        let packages1 = Evercity::bond_holder_packages(&bondid, &INVESTOR1);
        let packages2 = Evercity::bond_holder_packages(&bondid, &INVESTOR2);

        assert_eq!(packages1.len(), 0);
        assert_eq!(packages2.len(), 0);
    });
}

#[test]
fn bond_impact_report_missing_data() {
    const ACCOUNT1: u64 = 3;
    const ACCOUNT2: u64 = 7;

    let bondid1: BondId = "BOND1".into();
    let bondid2: BondId = "BOND2".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        bond_activate(bondid1, ACCOUNT1, bond.clone());
        bond_activate(bondid2, ACCOUNT2, bond.clone());

        for &period in &[0, 1, 3, 5, 7, 9] {
            assert_ok!(Evercity::set_impact_data(
                &bondid1,
                period,
                bond.impact_data_baseline[period as usize]
            ));
        }
        let chain_bond_item = Evercity::get_bond(&bondid1);
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date
                + 1000_u64
                    * (bond.start_period + bond.bond_duration * bond.payment_period + 1) as u64,
        );
        assert_ok!(add_token(ACCOUNT1, 500_000_000_000_000));
        assert_ok!(add_token(ACCOUNT2, 500_000_000_000_000));
        //force interest rate calculation
        assert_ok!(Evercity::bond_redeem(Origin::signed(ACCOUNT1), bondid1));
        assert_ok!(Evercity::bond_redeem(Origin::signed(ACCOUNT2), bondid2));

        let ref_interest_rate1 = vec![
            1900, 2000, 2000, 2400, 2000, 2400, 2000, 2400, 2000, 2400, 2000, 2400, 2800,
        ];
        for (calc_interest_rate, ref_interest_rate) in Evercity::get_coupon_yields(&bondid1)
            .iter()
            .map(|coupon| coupon.interest_rate)
            .zip(ref_interest_rate1)
        {
            assert_eq!(calc_interest_rate, ref_interest_rate);
        }

        let ref_interest_rate2 = vec![
            1900, 2300, 2700, 3100, 3500, 3900, 4000, 4000, 4000, 4000, 4000, 4000, 4000,
        ];

        for (calc_interest_rate, ref_interest_rate) in Evercity::get_coupon_yields(&bondid2)
            .iter()
            .map(|coupon| coupon.interest_rate)
            .zip(ref_interest_rate2)
        {
            assert_eq!(calc_interest_rate, ref_interest_rate);
        }
    });
}

#[test]
fn bond_interest_rate_rnd() {
    use rand::{
        self,
        distributions::{Distribution, Uniform},
    };

    const ACCOUNT: u64 = 3;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        let mut rng = rand::thread_rng();

        let mut bond = get_test_bond().inner;
        let impact_data_range = Uniform::new_inclusive(
            bond.impact_data_max_deviation_floor,
            bond.impact_data_max_deviation_cap,
        );
        for period in 0..bond.bond_duration as usize {
            bond.impact_data_baseline[period] = impact_data_range.sample(&mut rng);
        }
        let periods: usize = bond.bond_duration as usize;
        //create bond
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, bond);

        for period in 0..periods {
            assert_ok!(Evercity::set_impact_data(
                &bondid,
                period as BondPeriodNumber,
                20000_u64
            ));
        }
        // force impact interesting rate calculation
        let chain_bond_item = Evercity::get_bond(&bondid);
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date
                + 1000_u64
                    * (chain_bond_item.inner.start_period
                        + chain_bond_item.inner.bond_duration
                            * chain_bond_item.inner.payment_period
                        + 1) as u64,
        );

        assert_ok!(add_token(ACCOUNT, 500_000_000_000_000));
        assert_ok!(Evercity::bond_redeem(Origin::signed(ACCOUNT), bondid));

        //
        let bond_coupon_yields = Evercity::bond_coupon_yield(&bondid);
        let impact_reports = Evercity::impact_reports(&bondid);

        assert_eq!(bond_coupon_yields.len(), periods + 1);
        assert_eq!(
            bond_coupon_yields[0].interest_rate,
            chain_bond_item.inner.interest_rate_start_period_value
        );
        for period in 0..periods {
            let interest_rate = bond_coupon_yields[period + 1].interest_rate;
            let impact_data = impact_reports[period].impact_data;
            assert_eq!(impact_data, 20000_u64);
            // if impact data is less than baseline value then  interest rate is more than base value
            assert_eq!(
                impact_data < chain_bond_item.inner.impact_data_baseline[period],
                interest_rate > chain_bond_item.inner.interest_rate_base_value
            );

            println!(
                "{}: impact_data={} <> baseline={}, base interest rate={} <> interest rate={}",
                period,
                impact_data,
                chain_bond_item.inner.impact_data_baseline[period],
                chain_bond_item.inner.interest_rate_base_value,
                interest_rate
            )
        }
    });
}

#[test]
fn bond_impact_report_interest_rate() {
    const ACCOUNT1: u64 = 3;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        bond_activate(bondid, ACCOUNT1, bond.clone());

        for (period, impact_data) in [
            bond.impact_data_baseline[0],
            0,
            60000,
            bond.impact_data_max_deviation_floor,
            bond.impact_data_max_deviation_cap,
            25000,
            16000,
        ]
        .iter()
        .enumerate()
        {
            assert_ok!(Evercity::set_impact_data(
                &bondid,
                period as BondPeriodNumber,
                *impact_data
            ));
        }
        let chain_bond_item = Evercity::get_bond(&bondid);
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date
                + 1000_u64
                    * (bond.start_period + bond.bond_duration * bond.payment_period + 1) as u64,
        );

        assert_ok!(add_token(ACCOUNT1, 500_000_000_000_000));

        //force interest rate calculation
        assert_ok!(Evercity::bond_redeem(Origin::signed(ACCOUNT1), bondid));

        let ref_interest_rate = vec![
            bond.interest_rate_start_period_value,
            bond.interest_rate_base_value,
            bond.interest_rate_margin_cap,
            bond.interest_rate_margin_floor,
            bond.interest_rate_margin_cap,
            bond.interest_rate_margin_floor,
            1500,
            3333,
        ];
        for (calc_interest_rate, ref_interest_rate) in Evercity::get_coupon_yields(&bondid)
            .iter()
            .map(|coupon| coupon.interest_rate)
            .zip(ref_interest_rate)
        {
            assert_eq!(calc_interest_rate, ref_interest_rate);
        }
    });
}

#[test]
fn bond_impact_report_send_approve() {
    const ACCOUNT1: u64 = 3;
    const AUDITOR: u64 = 5;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        bond_activate(bondid, ACCOUNT1, bond.clone());

        let chain_bond_item = Evercity::get_bond(&bondid);

        for period in 0..bond.bond_duration {
            // day before end of the period
            <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
                chain_bond_item.active_start_date
                    + 1000_u64 * (bond.start_period + period * bond.payment_period - 1) as u64,
            );
            assert_ok!(Evercity::bond_impact_report_send(
                Origin::signed(ACCOUNT1),
                bondid,
                period,
                bond.impact_data_baseline[period as usize]
            ));
            assert_ok!(Evercity::bond_impact_report_approve(
                Origin::signed(AUDITOR),
                bondid,
                period,
                bond.impact_data_baseline[period as usize]
            ));
        }
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date
                + 1000_u64
                    * (bond.start_period + bond.bond_duration * bond.payment_period + 1) as u64,
        );

        assert_ok!(add_token(ACCOUNT1, 500_000_000_000_000));
        //force interest rate calculation
        assert_ok!(Evercity::bond_redeem(Origin::signed(ACCOUNT1), bondid));

        let ref_interest_rate = vec![
            bond.interest_rate_start_period_value,
            bond.interest_rate_base_value,
            bond.interest_rate_base_value,
            bond.interest_rate_base_value,
            bond.interest_rate_base_value,
            bond.interest_rate_base_value,
            bond.interest_rate_base_value,
            bond.interest_rate_base_value,
            bond.interest_rate_base_value,
        ];
        for (calc_interest_rate, ref_interest_rate) in Evercity::get_coupon_yields(&bondid)
            .iter()
            .map(|coupon| coupon.interest_rate)
            .zip(ref_interest_rate)
        {
            assert_eq!(calc_interest_rate, ref_interest_rate);
        }
    });
}

#[test]
fn bond_impact_report_try_approve_unauthorized() {
    const ACCOUNT1: u64 = 3;
    const AUDITOR: u64 = 5;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        bond_activate(bondid, ACCOUNT1, bond.clone());

        let chain_bond_item = Evercity::get_bond(&bondid);
        // first period
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date + 1000_u64 * (bond.start_period - 1) as u64,
        );

        // send report
        assert_ok!(Evercity::bond_impact_report_send(
            Origin::signed(ACCOUNT1),
            bondid,
            0,
            1000
        ));

        for acc in iter_accounts().filter(|acc| *acc != AUDITOR) {
            assert_noop!(
                Evercity::bond_impact_report_approve(Origin::signed(acc), bondid, 0, 1000),
                RuntimeError::AccountNotAuthorized
            );
        }

        // make amend

        assert_ok!(Evercity::bond_impact_report_approve(
            Origin::signed(AUDITOR),
            bondid,
            0,
            1000
        ));
    });
}

#[test]
fn bond_impact_report_try_approve_unattended() {
    const ACCOUNT1: u64 = 3;
    const AUDITOR: u64 = 5;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        bond_activate(bondid, ACCOUNT1, bond.clone());

        let chain_bond_item = Evercity::get_bond(&bondid);
        // first period
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date + 1000_u64 * (bond.start_period - 1) as u64,
        );
        // try approve without report
        assert_noop!(
            Evercity::bond_impact_report_approve(Origin::signed(AUDITOR), bondid, 0, 0),
            RuntimeError::BondParamIncorrect
        );

        // make amend
        assert_ok!(Evercity::bond_impact_report_send(
            Origin::signed(ACCOUNT1),
            bondid,
            0,
            0
        ));

        assert_ok!(Evercity::bond_impact_report_approve(
            Origin::signed(AUDITOR),
            bondid,
            0,
            0
        ));
    });
}

#[test]
fn bond_impact_report_outof_order() {
    const ACCOUNT1: u64 = 3;
    const AUDITOR: u64 = 5;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        bond_activate(bondid, ACCOUNT1, bond.clone());

        let chain_bond_item = Evercity::get_bond(&bondid);

        for period in 0..bond.bond_duration {
            // before start of the report period
            <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
                chain_bond_item.active_start_date
                    + 1000_u64
                        * (bond.start_period + period * bond.payment_period
                            - bond.impact_data_send_period
                            - 1) as u64,
            );
            assert_noop!(
                Evercity::bond_impact_report_send(
                    Origin::signed(ACCOUNT1),
                    bondid,
                    period,
                    bond.impact_data_baseline[period as usize]
                ),
                RuntimeError::BondOutOfOrder
            );

            // after current period end
            <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
                chain_bond_item.active_start_date
                    + 1000_u64 * (bond.start_period + period * bond.payment_period + 1) as u64,
            );

            assert_noop!(
                Evercity::bond_impact_report_send(
                    Origin::signed(ACCOUNT1),
                    bondid,
                    period,
                    bond.impact_data_baseline[period as usize]
                ),
                RuntimeError::BondOutOfOrder
            );

            // between report period start and  current period end
            <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
                chain_bond_item.active_start_date
                    + 1000_u64 * (bond.start_period + period * bond.payment_period - 1000) as u64,
            );

            assert_ok!(Evercity::bond_impact_report_send(
                Origin::signed(ACCOUNT1),
                bondid,
                period,
                bond.impact_data_baseline[period as usize]
            ));

            assert_ok!(Evercity::bond_impact_report_approve(
                Origin::signed(AUDITOR),
                bondid,
                period,
                bond.impact_data_baseline[period as usize]
            ));
        }
    });
}

#[test]
fn bond_acquire_try_exceed_max() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;
    let bondid: BondId = "BOND1".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, get_test_bond().inner);

        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            3,
            599
        ));
        assert_noop!(
            Evercity::bond_unit_package_buy(Origin::signed(INVESTOR2), bondid, 3, 2),
            RuntimeError::BondParamIncorrect
        );

        // make amend
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            3,
            1
        ));
    });
}

#[test]
fn bond_acquire_try_own_bond() {
    const ACCOUNT1: u64 = 7;
    const ACCOUNT2: u64 = 3;
    let bondid1: BondId = "BOND1".into();
    let bondid2: BondId = "BOND2".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        bond_activate(bondid1, ACCOUNT1, get_test_bond().inner);
        bond_activate(bondid2, ACCOUNT2, get_test_bond().inner);
        let chain_bond_item = Evercity::get_bond(&bondid1);

        assert_eq!(chain_bond_item.issued_amount, 1200);

        assert_noop!(
            Evercity::bond_unit_package_buy(Origin::signed(ACCOUNT1), bondid1, 3, 1),
            RuntimeError::AccountNotAuthorized
        );

        let chain_bond_item = Evercity::get_bond(&bondid1);
        assert_eq!(chain_bond_item.issued_amount, 1200);

        // make amend by acquiring other bond
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(ACCOUNT1),
            bondid2,
            3,
            1
        ));
    });
}

#[test]
fn bond_acquire_try_after_redemption() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    let bondid: BondId = "BOND0000".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        bond_activate(bondid, ACCOUNT, bond.clone());
        let chain_bond_item = Evercity::get_bond(&bondid);

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date
                + 1000_u64
                    * (bond.start_period + bond.bond_duration * bond.payment_period + 1) as u64,
        );
        // add everusd to pay off bond yield
        assert_ok!(add_token(ACCOUNT, 500_000_000_000_000));
        assert_ok!(Evercity::bond_redeem(Origin::signed(ACCOUNT), bondid));

        assert_noop!(
            Evercity::bond_unit_package_buy(Origin::signed(INVESTOR1), bondid, 4, 2),
            RuntimeError::BondStateNotPermitAction
        );
    });
}

#[test]
fn bond_deposit_bond() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        assert!(Evercity::evercity_balance().is_ok());

        let bond = get_test_bond().inner;
        bond_activate(bondid, ACCOUNT, bond.clone());
        let chain_bond_item = Evercity::get_bond(&bondid);

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date + 1000_u64 * (bond.start_period + 1) as u64,
        );

        assert_eq!(chain_bond_item.bond_debit, 0);
        assert_eq!(chain_bond_item.coupon_yield, 0);
        assert_eq!(
            Evercity::balance_everusd(&ACCOUNT),
            1200 * 4_000_000_000_000
        );

        assert_ok!(Evercity::bond_deposit_everusd(
            Origin::signed(ACCOUNT),
            bondid,
            100_000_000_000_000
        ));

        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.bond_debit, 100_000_000_000_000);
        assert_eq!(chain_bond_item.coupon_yield, 0);
        assert!(Evercity::evercity_balance().is_ok());

        assert_eq!(
            Evercity::balance_everusd(&ACCOUNT),
            1200 * 4_000_000_000_000 - 100_000_000_000_000
        );

        assert_ok!(Evercity::bond_withdraw_everusd(
            Origin::signed(INVESTOR1),
            bondid
        ));
        let chain_bond_item = Evercity::get_bond(&bondid);
        assert_eq!(chain_bond_item.coupon_yield, 14_991_780_821_760);
        assert_eq!(chain_bond_item.get_debt(), 0);
        // 1.9 % - (600 + 600) x 4000 usd - 120 days
        assert_eq!(chain_bond_item.bond_credit, 29_983_561_643_520);
        assert_eq!(
            chain_bond_item.get_free_balance(),
            100_000_000_000_000 - 29_983_561_643_520
        );
        assert!(Evercity::evercity_balance().is_ok());
    });
}

#[test]
fn bond_deposit_return_after_redemption() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        let bond = get_test_bond().inner;
        bond_activate(bondid, ACCOUNT, bond.clone());
        let chain_bond_item = Evercity::get_bond(&bondid);

        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(
            chain_bond_item.active_start_date
                + 1000_u64
                    * (bond.start_period + bond.bond_duration * bond.payment_period + 1) as u64,
        );
        // add everusd to pay off bond yield
        assert_ok!(add_token(ACCOUNT, 500_000_000_000_000));

        assert_ok!(Evercity::bond_redeem(Origin::signed(ACCOUNT), bondid));

        assert_noop!(
            Evercity::bond_unit_package_buy(Origin::signed(INVESTOR1), bondid, 4, 2),
            RuntimeError::BondStateNotPermitAction
        );
    });
}

#[test]
fn bond_deposit_try_foreign() {
    const ACCOUNT1: u64 = 3;
    const ACCOUNT2: u64 = 7;

    let bondid1: BondId = "BOND1".into();
    let bondid2: BondId = "BOND2".into();

    new_test_ext().execute_with(|| {
        bond_grand_everusd();
        assert!(Evercity::evercity_balance().is_ok());

        let bond = get_test_bond().inner;
        bond_activate(bondid1, ACCOUNT1, bond.clone());
        bond_activate(bondid2, ACCOUNT2, bond);
        assert!(Evercity::evercity_balance().is_ok());

        assert_noop!(
            Evercity::bond_deposit_everusd(Origin::signed(ACCOUNT1), bondid2, 100_000_000_000_000),
            RuntimeError::BondAccessDenied
        );
    });
}

#[test]
fn bond_lot_bit_n_buy() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        let bond = get_test_bond().inner;
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, bond);

        assert!(Evercity::bond_check_invariant(&bondid));

        let lot = BondUnitSaleLotStruct {
            deadline: 100000,
            new_bondholder: Default::default(),
            bond_units: 600,
            amount: 600 * 3_000_000_000_000,
        };
        assert!(Evercity::evercity_balance().is_ok());
        assert_ok!(Evercity::bond_unit_lot_bid(
            Origin::signed(INVESTOR1),
            bondid,
            lot.clone()
        ));
        assert_ok!(Evercity::bond_unit_lot_settle(
            Origin::signed(INVESTOR2),
            bondid,
            INVESTOR1,
            lot
        ));
        assert!(Evercity::bond_check_invariant(&bondid));
        assert!(Evercity::evercity_balance().is_ok());
        let packages1 = Evercity::bond_holder_packages(&bondid, &INVESTOR1);
        let bond_units1: BondUnitAmount = packages1.iter().map(|p| p.bond_units).sum();
        let packages2 = Evercity::bond_holder_packages(&bondid, &INVESTOR2);
        let bond_units2: BondUnitAmount = packages2.iter().map(|p| p.bond_units).sum();

        assert_eq!(bond_units1, 0);
        assert_eq!(bond_units2, 1200);
    });
}

#[test]
fn bond_lot_paid_coupon() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        let bond = get_test_bond().inner;
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, bond.clone());
        let chain_bond_item = Evercity::get_bond(&bondid);

        // buy additional 200 + 100
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            3,
            100
        ));
        assert_ok!(Evercity::bond_unit_package_buy(
            Origin::signed(INVESTOR1),
            bondid,
            3,
            200
        ));
        assert!(Evercity::bond_check_invariant(&bondid));
        // first period
        let moment = chain_bond_item.active_start_date + 1000_u64 * (bond.start_period + 1) as u64;
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(moment);

        let (_, period) = chain_bond_item
            .time_passed_after_activation(moment)
            .unwrap();
        assert_eq!(period, 1);

        let lot = BondUnitSaleLotStruct {
            deadline: moment + 1,
            new_bondholder: Default::default(),
            bond_units: 400,
            amount: 400 * 3_000_000_000_000,
        };

        // deposit will be used to pay coupon
        assert_ok!(Evercity::bond_deposit_everusd(
            Origin::signed(ACCOUNT),
            bondid,
            100_000_000_000_000
        ));

        let balance1 = Evercity::balance_everusd(&INVESTOR1);

        assert!(Evercity::evercity_balance().is_ok());
        assert_ok!(Evercity::bond_unit_lot_bid(
            Origin::signed(INVESTOR1),
            bondid,
            lot.clone()
        ));
        assert_ok!(Evercity::bond_unit_lot_settle(
            Origin::signed(INVESTOR2),
            bondid,
            INVESTOR1,
            lot
        ));
        assert!(Evercity::evercity_balance().is_ok());

        let packages1 = Evercity::bond_holder_packages(&bondid, &INVESTOR1);
        let bond_units1: BondUnitAmount = packages1.iter().map(|p| p.bond_units).sum();

        let packages2 = Evercity::bond_holder_packages(&bondid, &INVESTOR2);
        let bond_units2: BondUnitAmount = packages2.iter().map(|p| p.bond_units).sum();

        assert_eq!(bond_units1, 500);
        assert_eq!(bond_units2, 1000);

        let bond_units1: Vec<_> = packages1.iter().map(|p| p.bond_units).collect();
        let bond_units2: Vec<_> = packages2.iter().map(|p| p.bond_units).collect();

        assert_eq!(bond_units1, vec![500]);
        assert_eq!(bond_units2, vec![600, 100, 200, 100]);
        // 1.9% - 120 days - (600 + 200 + 100) units x 4000 usd =22487.671 usd
        // @TODO calc coupon yield
        assert_eq!(
            Evercity::balance_everusd(&INVESTOR1) - balance1,
            400 * 3_000_000_000_000 + 22_487_671_232_640
        );
    });
}

#[test]
fn bond_lot_try_buy_foreign() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        let bond = get_test_bond().inner;
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, bond);

        let lot = BondUnitSaleLotStruct {
            deadline: 100000,
            new_bondholder: 7,
            bond_units: 600,
            amount: 600 * 3_000_000_000_000,
        };
        assert!(Evercity::evercity_balance().is_ok());
        assert_ok!(Evercity::bond_unit_lot_bid(
            Origin::signed(INVESTOR1),
            bondid,
            lot.clone()
        ));
        assert_noop!(
            Evercity::bond_unit_lot_settle(Origin::signed(INVESTOR2), bondid, INVESTOR1, lot),
            RuntimeError::LotNotFound
        );
    });
}

#[test]
fn bond_lot_try_create_expired() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        let bond = get_test_bond().inner;
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, bond);

        let lot = BondUnitSaleLotStruct {
            deadline: 100000,
            new_bondholder: Default::default(),
            bond_units: 600,
            amount: 600 * 3_000_000_000_000,
        };
        // move forward
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(1000000 + 1);
        assert_noop!(
            Evercity::bond_unit_lot_bid(Origin::signed(INVESTOR1), bondid, lot),
            RuntimeError::LotParamIncorrect
        );
    });
}

#[test]
fn bond_lot_try_buy_expired() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    const INVESTOR2: u64 = 6;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        let bond = get_test_bond().inner;
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, bond);

        let lot = BondUnitSaleLotStruct {
            deadline: 100000,
            new_bondholder: Default::default(),
            bond_units: 600,
            amount: 600 * 3_000_000_000_000,
        };
        assert!(Evercity::evercity_balance().is_ok());
        assert_ok!(Evercity::bond_unit_lot_bid(
            Origin::signed(INVESTOR1),
            bondid,
            lot.clone()
        ));

        // move forward
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(1000000 + 1);

        assert_noop!(
            Evercity::bond_unit_lot_settle(Origin::signed(INVESTOR2), bondid, INVESTOR1, lot),
            RuntimeError::LotObsolete
        );
    });
}

#[test]
fn bond_lot_try_exceed_portfolio() {
    const ACCOUNT: u64 = 3;
    const INVESTOR1: u64 = 4;
    let bondid: BondId = "BOND".into();

    new_test_ext().execute_with(|| {
        let bond = get_test_bond().inner;
        bond_grand_everusd();
        bond_activate(bondid, ACCOUNT, bond);

        let lot = BondUnitSaleLotStruct {
            deadline: 100000,
            new_bondholder: Default::default(),
            bond_units: 500,
            amount: 600 * 3_000_000_000_000,
        };

        assert_ok!(Evercity::bond_unit_lot_bid(
            Origin::signed(INVESTOR1),
            bondid,
            lot.clone()
        ));
        assert_noop!(
            Evercity::bond_unit_lot_bid(Origin::signed(INVESTOR1), bondid, lot.clone()),
            RuntimeError::BalanceOverdraft
        );
        // make amend. make prior lots expired
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(100000 + 1);
        let mut lot = lot;
        lot.deadline = 100000 + 2;
        assert_ok!(Evercity::bond_unit_lot_bid(
            Origin::signed(INVESTOR1),
            bondid,
            lot
        ));
    });
}
