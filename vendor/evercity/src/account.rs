use frame_support::{
    codec::{Decode, Encode},
    sp_runtime::RuntimeDebug,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use crate::{EverUSDBalance, Expired};

pub const MASTER_ROLE_MASK: u8 = 1u8;
pub const CUSTODIAN_ROLE_MASK: u8 = 2u8;
pub const ISSUER_ROLE_MASK: u8 = 4u8;
pub const INVESTOR_ROLE_MASK: u8 = 8u8;
pub const AUDITOR_ROLE_MASK: u8 = 16u8;
pub const MANAGER_ROLE_MASK: u8 = 32u8;
pub const IMPACT_REPORTER_ROLE_MASK: u8 = 64u8;

pub const ALL_ROLES_MASK: u8 = MASTER_ROLE_MASK
    | CUSTODIAN_ROLE_MASK
    | ISSUER_ROLE_MASK
    | INVESTOR_ROLE_MASK
    | AUDITOR_ROLE_MASK
    | MANAGER_ROLE_MASK
    | IMPACT_REPORTER_ROLE_MASK;

#[inline]
pub const fn is_roles_correct(roles: u8) -> bool {
    // max value of any roles combinations
    roles <= ALL_ROLES_MASK && roles > 0
}

/// Main structure, containing account data: roles(bit mask), identity(external id), creation_time.
/// This structure is used to check and assign account roles
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct EvercityAccountStructT<Moment> {
    pub roles: u8,
    #[codec(compact)]
    pub identity: u64,
    #[codec(compact)]
    pub create_time: Moment,
}

pub type EvercityAccountStructOf<T> =
    EvercityAccountStructT<<T as pallet_timestamp::Config>::Moment>;

/// Structure, created by Issuer or Investor to receive EverUSD on her balance
/// by paying USD to Custodian. Then Custodian confirms request, adding corresponding
/// amount to mint request creator's balance
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct TokenMintRequestStruct<Moment> {
    #[codec(compact)]
    pub amount: EverUSDBalance,
    #[codec(compact)]
    pub deadline: Moment,
}

impl<Moment: core::cmp::PartialOrd> Expired<Moment> for TokenMintRequestStruct<Moment> {
    fn is_expired(&self, now: Moment) -> bool {
        self.deadline <= now
    }
}

pub type TokenMintRequestStructOf<T> =
    TokenMintRequestStruct<<T as pallet_timestamp::Config>::Moment>;

/// Structure, created by Issuer or Investor to burn EverUSD on her balance
/// and receive corresponding amount of USD from Custodian.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct TokenBurnRequestStruct<Moment> {
    #[codec(compact)]
    pub amount: EverUSDBalance,
    #[codec(compact)]
    pub deadline: Moment,
}

impl<Moment: core::cmp::PartialOrd> Expired<Moment> for TokenBurnRequestStruct<Moment> {
    fn is_expired(&self, now: Moment) -> bool {
        self.deadline <= now
    }
}

pub type TokenBurnRequestStructOf<T> =
    TokenBurnRequestStruct<<T as pallet_timestamp::Config>::Moment>;

#[impl_trait_for_tuples::impl_for_tuples(30)]
pub trait OnAddAccount<AccountId, Moment> {
    fn on_add_account(account: &AccountId, data: &EvercityAccountStructT<Moment>);
}
