//! Program fees

use crate::error::AuctionError;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
use std::convert::TryFrom;

/// Encapsulates all fee information and calculations for swap operations
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AuctionFees {
    /// At farm creation, if the LP token is not a B2B_token-CRP composition, user B will be subject to an additional fee of 500 CRP;
    pub auction_fee_numerator: u64,
    /// Additional fee denominator
    pub auction_fee_denominator: u64,

}

/// Helper function for calculating swap fee
pub fn calculate_fee(
    token_amount: u128,
    fee_numerator: u128,
    fee_denominator: u128,
) -> Option<u128> {
    if fee_numerator == 0 || token_amount == 0 {
        Some(0)
    } else {
        let fee = token_amount
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        if fee == 0 {
            Some(1) // minimum fee of one token
        } else {
            Some(fee)
        }
    }
}

fn validate_fraction(numerator: u64, denominator: u64) -> Result<(), AuctionError> {
    if denominator == 0 && numerator == 0 {
        Ok(())
    } else if numerator >= denominator {
        Err(AuctionError::InvalidFee)
    } else {
        Ok(())
    }
}

impl AuctionFees {
    /// Calculate the withdraw fee in pool tokens
    pub fn auction_fee(&self, pool_tokens: u128) -> Option<u128> {
        calculate_fee(
            pool_tokens,
            u128::try_from(self.auction_fee_numerator).ok()?,
            u128::try_from(self.auction_fee_denominator).ok()?,
        )
    }

    /// Validate that the fees are reasonable
    pub fn validate(&self) -> Result<(), AuctionError> {
        validate_fraction(
            self.auction_fee_numerator,
            self.auction_fee_denominator,
        )?;
        Ok(())
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for AuctionFees {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Sealed for AuctionFees {}
impl Pack for AuctionFees {
    const LEN: usize = 64;
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 16];
        let (
            auction_fee_numerator,
            auction_fee_denominator,
        ) = mut_array_refs![output, 8, 8];
        *auction_fee_numerator = self.auction_fee_numerator.to_le_bytes();
        *auction_fee_denominator = self.auction_fee_denominator.to_le_bytes();
    }

    fn unpack_from_slice(input: &[u8]) -> Result<AuctionFees, ProgramError> {
        let input = array_ref![input, 0, 16];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            auction_fee_numerator,
            auction_fee_denominator,
        ) = array_refs![input, 8, 8];
        Ok(Self {
            auction_fee_numerator: u64::from_le_bytes(*auction_fee_numerator),
            auction_fee_denominator: u64::from_le_bytes(*auction_fee_denominator),
        })
    }
}
