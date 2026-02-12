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
//! Unit tests for the Robonomics Claim pallet.

#[cfg(test)]
use super::*;
use crate::Call as ClaimsCall;
use crate::{self as claims, mock::*};
use hex_literal::hex;
use secp_utils::*;

use parity_scale_codec::Encode;
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use frame_support::{assert_err, assert_noop, assert_ok};
use sp_runtime::transaction_validity::TransactionLongevity;

#[test]
fn basic_setup_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(claims::Claims::<Test>::get(&eth(&alice())), Some(100));
        assert_eq!(claims::Claims::<Test>::get(&eth(&dave())), Some(200));
        assert_eq!(claims::Claims::<Test>::get(&eth(&eve())), Some(300));
        assert_eq!(claims::Claims::<Test>::get(&eth(&frank())), Some(400));
        assert_eq!(
            claims::Claims::<Test>::get(&EthereumAddress::default()),
            None
        );
    });
}

#[test]
fn serde_works() {
    let x = EthereumAddress(hex!["0123456789abcdef0123456789abcdef01234567"]);
    let y = serde_json::to_string(&x).unwrap();
    assert_eq!(y, "\"0x0123456789abcdef0123456789abcdef01234567\"");
    let z: EthereumAddress = serde_json::from_str(&y).unwrap();
    assert_eq!(x, z);
}

#[test]
fn claiming_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::free_balance(42), 0);
        assert_ok!(claims::mock::Claims::claim(
            RuntimeOrigin::none(),
            42,
            sig::<Test>(&alice(), &42u64.encode())
        ));
        assert_eq!(Balances::free_balance(&42), 100);
    });
}

#[test]
fn claiming_does_not_bypass_signing() {
    new_test_ext().execute_with(|| {
        assert_ok!(claims::mock::Claims::claim(
            RuntimeOrigin::none(),
            42,
            sig::<Test>(&alice(), &42u64.encode())
        ));
        assert_ok!(claims::mock::Claims::claim(
            RuntimeOrigin::none(),
            42,
            sig::<Test>(&frank(), &42u64.encode())
        ));
    });
}

#[test]
fn add_claim_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            claims::mock::Claims::add_claim(RuntimeOrigin::signed(42), eth(&bob()), 200,),
            sp_runtime::traits::BadOrigin,
        );
        assert_eq!(Balances::free_balance(42), 0);
        assert_noop!(
            claims::mock::Claims::claim(
                RuntimeOrigin::none(),
                69,
                sig::<Test>(&bob(), &69u64.encode())
            ),
            Error::<Test>::SignerHasNoClaim,
        );
        assert_ok!(claims::mock::Claims::add_claim(
            RuntimeOrigin::root(),
            eth(&bob()),
            200,
        ));
        assert_ok!(claims::mock::Claims::claim(
            RuntimeOrigin::none(),
            69,
            sig::<Test>(&bob(), &69u64.encode())
        ));
        assert_eq!(Balances::free_balance(&69), 200);
    });
}

#[test]
fn origin_signed_claiming_fail() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::free_balance(42), 0);
        assert_err!(
            claims::mock::Claims::claim(
                RuntimeOrigin::signed(42),
                42,
                sig::<Test>(&alice(), &42u64.encode())
            ),
            sp_runtime::traits::BadOrigin,
        );
    });
}

#[test]
fn double_claiming_doesnt_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::free_balance(42), 0);
        assert_ok!(claims::mock::Claims::claim(
            RuntimeOrigin::none(),
            42,
            sig::<Test>(&alice(), &42u64.encode())
        ));
        assert_noop!(
            claims::mock::Claims::claim(
                RuntimeOrigin::none(),
                42,
                sig::<Test>(&alice(), &42u64.encode())
            ),
            Error::<Test>::SignerHasNoClaim
        );
    });
}

#[test]
fn non_sender_sig_doesnt_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::free_balance(42), 0);
        assert_noop!(
            claims::mock::Claims::claim(
                RuntimeOrigin::none(),
                42,
                sig::<Test>(&alice(), &69u64.encode())
            ),
            Error::<Test>::SignerHasNoClaim
        );
    });
}

#[test]
fn non_claimant_doesnt_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::free_balance(42), 0);
        assert_noop!(
            claims::mock::Claims::claim(
                RuntimeOrigin::none(),
                42,
                sig::<Test>(&bob(), &69u64.encode())
            ),
            Error::<Test>::SignerHasNoClaim
        );
    });
}

#[test]
fn real_eth_sig_works() {
    new_test_ext().execute_with(|| {
			// "Pay RUSTs to the TEST account:2a00000000000000"
			let sig = hex!["444023e89b67e67c0562ed0305d252a5dd12b2af5ac51d6d3cb69a0b486bc4b3191401802dc29d26d586221f7256cd3329fe82174bdf659baea149a40e1c495d1c"];
			let sig = EcdsaSignature(sig);
			let who = 42u64.using_encoded(to_ascii_hex);
			let signer = claims::mock::Claims::eth_recover(&sig, &who).unwrap();
			assert_eq!(signer.0, hex!["6d31165d5d932d571f3b44695653b46dcc327e84"]);
		});
}

#[test]
fn validate_unsigned_works() {
    use sp_runtime::traits::ValidateUnsigned;
    let source = sp_runtime::transaction_validity::TransactionSource::External;

    new_test_ext().execute_with(|| {
        assert_eq!(
            Pallet::<Test>::validate_unsigned(
                source,
                &ClaimsCall::claim {
                    dest: 1,
                    ethereum_signature: sig::<Test>(&alice(), &1u64.encode())
                }
            ),
            Ok(ValidTransaction {
                priority: 100,
                requires: vec![],
                provides: vec![("claims", eth(&alice())).encode()],
                longevity: TransactionLongevity::max_value(),
                propagate: true,
            })
        );
        assert_eq!(
            Pallet::<Test>::validate_unsigned(
                source,
                &ClaimsCall::claim {
                    dest: 0,
                    ethereum_signature: EcdsaSignature([0; 65])
                }
            ),
            InvalidTransaction::Custom(0).into(),
        );
        assert_eq!(
            Pallet::<Test>::validate_unsigned(
                source,
                &ClaimsCall::claim {
                    dest: 1,
                    ethereum_signature: sig::<Test>(&bob(), &1u64.encode())
                }
            ),
            InvalidTransaction::Custom(1).into(),
        );
    });
}
