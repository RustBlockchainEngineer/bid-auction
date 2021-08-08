//! Pool Instructin types
//! by hongbo
#![allow(clippy::too_many_arguments)]

use crate::{
    fees::AuctionFees,
    error::AuctionError,
};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    clock::UnixTimestamp,
    sysvar,
};
use std::convert::TryInto;
use std::mem::size_of;

#[cfg(feature = "fuzz")]
use arbitrary::Arbitrary;


/// Defines which validator vote account is set during the SetPreferredValidator instruction
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum PreferredValidatorType {
    /// set preffered validator for deposits
    PlaceBid,

    /// set preffered validator for withdraws
    Withdraw,
}

/// Initialize instruction data
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct InitializeData {
    /// auction fees
    pub fees: AuctionFees,

    pub nonce: u8,

    pub start_timestamp: UnixTimestamp,

    pub end_timestamp: UnixTimestamp
}
/// PlaceBid instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceBid {
    
    /// Bid amount to deposit, prevents excessive slippage
    pub bid_amount: u64,
}

/// Withdraw instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Withdraw {
    /// withdraw winning bid amount
    pub bid_amount: u64,
}
/// Withdraw instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Cancel {
    /// withdraw winning bid amount
    pub canceled: u8,
}

/// Instructions supported by the auction program
#[repr(C)]
#[derive( Debug, PartialEq)]
pub enum AuctionInstruction {
    ///   Initializes a new Auction
    ///
    ///   0. `[writable, signer]` New Auction to create.
    ///   1. `[]` owner token Account. Must be non zero
    ///   2. `[]` Fee Token Account to deposit and withdraw fees.
    ///   3. `[]` Pool Token Account to deposit bids
    ///   4. '[]` Token program id
    Initialize(InitializeData),

    ///   deposit bid amount
    ///
    ///   0. `[]` Auction
    ///   1. `[writable]` token Base Account to deposit into.
    ///   2. `[writable]` Pool Account to deposit the tokens
    ///   3. '[]` Token program id
    ///   4. '[]' user_transfer_authority
    PlaceBid(PlaceBid),

    ///   Withdraw winning bid
    ///
    ///   0. `[]` Auction
    ///   1. `[writable]` SOURCE Pool token account
    ///   3. `[writable]` token user Account to credit.
    ///   4. `[writable]` Fee account, to receive withdrawal fees
    ///   5 '[]` Token program id
    ///   6. '[]' user_transfer_authority
    Withdraw(Withdraw),

    /// Cancel auction
    /// 
    /// 0.'[]' Auction
    Cancel(Cancel)
}


impl AuctionInstruction {
    //  unpack
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, _rest) = input.split_first().ok_or(AuctionError::InvalidInstruction)?;
        Ok(match tag {
            0 => {
                
                if _rest.len() >= AuctionFees::LEN {
                    let (fees, _rest) = _rest.split_at(AuctionFees::LEN);
                    let fees = AuctionFees::unpack_unchecked(fees)?;

                    let (&nonce, _rest) = _rest.split_first().ok_or(AuctionError::InvalidInstruction)?;

                    let (start_timestamp, _rest) = Self::unpack_i64(_rest)?;
                    let (end_timestamp, _rest) = Self::unpack_i64(_rest)?;

                    Self::Initialize(InitializeData {
                        fees,
                        nonce,
                        start_timestamp,
                        end_timestamp,
                    })
                } else {
                    return Err(AuctionError::InvalidInstruction.into());
                }
            }
            1 => {
                let (bid_amount, _rest) = Self::unpack_u64(_rest)?;
                Self::PlaceBid(PlaceBid {
                    bid_amount,
                })
            }
            2 => {
                let (bid_amount, _rest) = Self::unpack_u64(_rest)?;
                Self::Withdraw(Withdraw {
                    bid_amount,
                })
            }
            3 => {
                let (&canceled, _rest) = _rest.split_first().ok_or(AuctionError::InvalidInstruction)?;
                Self::Cancel(Cancel {
                    canceled,
                })
            }
            
            _ => return Err(AuctionError::InvalidInstruction.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, _rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(AuctionError::InvalidInstruction)?;
            Ok((amount, _rest))
        } else {
            Err(AuctionError::InvalidInstruction.into())
        }
    }
    fn unpack_i64(input: &[u8]) -> Result<(i64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, _rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(i64::from_le_bytes)
                .ok_or(AuctionError::InvalidInstruction)?;
            Ok((amount, _rest))
        } else {
            Err(AuctionError::InvalidInstruction.into())
        }
    }

    /// Packs a [AuctionInstruction](enum.AuctionInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match &*self {
            Self::Initialize(InitializeData {
                fees,
                nonce,
                start_timestamp,
                end_timestamp,
            }) => {
                buf.push(0);
                let mut fees_slice = [0u8; AuctionFees::LEN];
                Pack::pack_into_slice(fees, &mut fees_slice[..]);
                buf.extend_from_slice(&fees_slice);

                buf.push(*nonce);
                buf.extend_from_slice(&start_timestamp.to_le_bytes());
                buf.extend_from_slice(&end_timestamp.to_le_bytes());

            }
            Self::PlaceBid(PlaceBid {
                bid_amount,
            }) => {
                buf.push(1);
                buf.extend_from_slice(&bid_amount.to_le_bytes());
            }
            Self::Withdraw(Withdraw {
                bid_amount,
            }) => {
                buf.push(2);
                buf.extend_from_slice(&bid_amount.to_le_bytes());
            }
            Self::Cancel(Cancel {
                canceled,
            }) => {
                buf.push(3);
                buf.extend_from_slice(&canceled.to_le_bytes());
            }
        }
        buf
    }
}

/// Creates an 'initialize' instruction.
pub fn initialize(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    auction_pubkey: &Pubkey,
    owner_token_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    fee_pubkey: &Pubkey,
    fees: AuctionFees,
    nonce: u8,
    start_timestamp: UnixTimestamp,
    end_timestamp: UnixTimestamp,
) -> Result<Instruction, ProgramError> {
    let init_data = AuctionInstruction::Initialize(InitializeData {
        fees,
        nonce,
        start_timestamp,
        end_timestamp,
    });
    let data = init_data.pack();

    let accounts = vec![
        AccountMeta::new(*auction_pubkey, true),
        AccountMeta::new_readonly(*owner_token_pubkey, false),
        AccountMeta::new(*pool_pubkey, false),
        AccountMeta::new_readonly(*fee_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a 'place_bid' instruction.
pub fn place_bid(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    auction_pubkey: &Pubkey,
    deposit_token_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    user_transfer_authority_pubkey: &Pubkey,
    instruction: PlaceBid,
) -> Result<Instruction, ProgramError> {
    let data = AuctionInstruction::PlaceBid(instruction).pack();

    let accounts = vec![
        AccountMeta::new_readonly(*auction_pubkey, false),
        AccountMeta::new(*deposit_token_pubkey, false),
        AccountMeta::new(*pool_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a 'withdraw' instruction.
pub fn withdraw(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    auction_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    fee_account_pubkey: &Pubkey,
    destination_token_pubkey: &Pubkey,
    user_transfer_authority_pubkey: &Pubkey,
    instruction: Withdraw,
) -> Result<Instruction, ProgramError> {
    let data = AuctionInstruction::Withdraw(instruction).pack();

    let accounts = vec![
        AccountMeta::new_readonly(*auction_pubkey, false),
        AccountMeta::new(*pool_pubkey, false),
        AccountMeta::new(*destination_token_pubkey, false),
        AccountMeta::new(*fee_account_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a 'cancel' instruction.
pub fn cancel(
    program_id: &Pubkey,
    auction_pubkey: &Pubkey,
    instruction: Cancel,
) -> Result<Instruction, ProgramError> {
    let data = AuctionInstruction::Cancel(instruction).pack();

    let accounts = vec![
        AccountMeta::new_readonly(*auction_pubkey, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}
