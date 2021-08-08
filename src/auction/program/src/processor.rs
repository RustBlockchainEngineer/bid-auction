//! Program state processor
//! by hongbo

use crate::{
    error::AuctionError,
    instruction::{
        AuctionInstruction,
        InitializeData,
        PlaceBid,
        Withdraw,
        Cancel
    },
    state::{AuctionV1, AuctionVersion},
    fees::AuctionFees
};
use num_traits::FromPrimitive;
use std::convert::TryInto;

use solana_program::{
    account_info::{next_account_info, AccountInfo}, 
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::{PrintProgramError,ProgramError},
    program_pack::Pack,
    pubkey::Pubkey,
    clock::Clock,
    clock::UnixTimestamp,
    sysvar::Sysvar,
};
pub struct Processor {
}

impl Processor {
    /// Unpacks a spl_token `Account`.
    pub fn unpack_token_account(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Account, AuctionError> {
        if account_info.owner != token_program_id {
            Err(AuctionError::IncorrectTokenProgramId)
        } else {
            spl_token::state::Account::unpack(&account_info.data.borrow())
                .map_err(|_| AuctionError::ExpectedAccount)
        }
    }
    /// Unpacks a spl_token `Mint`.
    pub fn unpack_mint(
        account_info: &AccountInfo,
        token_program_id: &Pubkey,
    ) -> Result<spl_token::state::Mint, AuctionError> {
        if account_info.owner != token_program_id {
            Err(AuctionError::IncorrectTokenProgramId)
        } else {
            spl_token::state::Mint::unpack(&account_info.data.borrow())
                .map_err(|_| AuctionError::ExpectedMint)
        }
    }
    /// Processes an [Initialize](enum.Instruction.html).
    pub fn process_initialize(
        program_id: &Pubkey,
        fees: AuctionFees,
        nonce: u8,
        start_timestamp: UnixTimestamp,
        end_timestamp: UnixTimestamp,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let auction_info = next_account_info(account_info_iter)?;
        let token_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_program_id = *token_program_info.key;
        if AuctionVersion::is_initialized(&auction_info.data.borrow()) {
            return Err(AuctionError::AlreadyInUse.into());
        }


        fees.validate()?;

        let canceled = 0;
        
        let obj = AuctionVersion::AuctionV1(AuctionV1 {
            is_initialized: true,
            token_program_id,
            token: *token_info.key,
            pool: *pool_info.key,
            fee_account: *fee_account_info.key,
            fees,
            nonce,
            start_timestamp,
            end_timestamp,
            canceled,
        });
        AuctionVersion::pack(obj, &mut auction_info.data.borrow_mut())?;
        Ok(())
    }
    /// Issue a spl_token `Transfer` instruction.
    pub fn token_transfer<'a>(
        auction: &Pubkey,
        token_program: AccountInfo<'a>,
        source: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let auction_bytes = auction.to_bytes();
        let authority_signature_seeds = [&auction_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];

        let ix = spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke_signed(
            &ix,
            &[source, destination, authority, token_program],
            signers,
        )
    }
    pub fn process_place_bid(
        program_id: &Pubkey,
        bid_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {

        let account_info_iter = &mut accounts.iter();
        let auction_info = next_account_info(account_info_iter)?;
        let token_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;

        let clock_sysvar_info = next_account_info(account_info_iter)?;
        let clock = &Clock::from_account_info(clock_sysvar_info)?;

        
        let auction = &mut AuctionVersion::unpack(&auction_info.data.borrow())?;

        if clock.unix_timestamp > auction.end_timestamp() {
            msg!("This auction was ended!");
        }
        else if clock.unix_timestamp < auction.start_timestamp() {
            msg!("This auction was not started yet!");
        }
        else if auction.canceled() == 1 {
            msg!("This auction was canceled!");
        }
        else {
            Self::token_transfer(
                auction_info.key,
                token_program_info.clone(),
                token_info.clone(),
                pool_info.clone(),
                user_transfer_authority_info.clone(),
                auction.nonce(),
                bid_amount,
            )?;
        }
        Ok(())
    }
    pub fn process_withdraw(
        program_id: &Pubkey,
        bid_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let token_program_info = next_account_info(account_info_iter)?;
        let auction_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let fee_account_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;

        let auction = AuctionVersion::unpack(&auction_info.data.borrow())?;

        let withdraw_fee: u64 = to_u64(auction
            .fees()
            .auction_fee(to_u128(bid_amount)?)
            .ok_or(AuctionError::FeeCalculationFailure)?)?;

        
        Self::token_transfer(
            auction_info.key,
            token_program_info.clone(),
            destination_info.clone(),
            pool_info.clone(),
            user_transfer_authority_info.clone(),
            auction.nonce(),
            bid_amount,
        )?;

        //fee
        Self::token_transfer(
            auction_info.key,
            token_program_info.clone(),
            fee_account_info.clone(),
            pool_info.clone(),
            user_transfer_authority_info.clone(),
            auction.nonce(),
            withdraw_fee,
        )?;

        Ok(())
    }
    pub fn process_cancel(
        program_id: &Pubkey,
        canceled: u8,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let auction_info = next_account_info(account_info_iter)?;

        let auction = AuctionVersion::unpack(&auction_info.data.borrow())?;
        
        let obj = AuctionVersion::AuctionV1(AuctionV1 {
            is_initialized:auction.is_initialized(),
            token_program_id: *auction.token_program_id(),
            token: *auction.token_account(),
            pool: *auction.pool(),
            fee_account: *auction.fee_account(),
            fees: auction.fees().clone(),
            nonce: auction.nonce(),
            start_timestamp:auction.start_timestamp(),
            end_timestamp: auction.end_timestamp(),
            canceled: canceled,
        });
        AuctionVersion::pack(obj, &mut auction_info.data.borrow_mut())?;
        Ok(())
    }
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult{
        
        let instruction = AuctionInstruction::unpack(input)?;
        match instruction {
            AuctionInstruction::Initialize(InitializeData {
                fees,
                nonce,
                start_timestamp,
                end_timestamp,
            }) => {
                msg!("Instruction: Init");
                Self::process_initialize(
                    program_id,
                    fees,
                    nonce,
                    start_timestamp,
                    end_timestamp,
                    accounts,
                )?;
            }
            AuctionInstruction::PlaceBid(PlaceBid {
                bid_amount,
            }) => {
                msg!("Instruction: PlaceBid");
                
                Self::process_place_bid(
                    program_id,
                    bid_amount,
                    accounts,
                )?;
                
            }
            AuctionInstruction::Withdraw(Withdraw {
                bid_amount,
            }) => {
                msg!("Instruction: Withdraw");
                
                
                Self::process_withdraw(
                    program_id,
                    bid_amount,
                    accounts,
                )?;
            }
            AuctionInstruction::Cancel(Cancel {
                canceled,
            }) => {
                msg!("Instruction: Cancel");
                
                
                Self::process_cancel(
                    program_id,
                    canceled,
                    accounts,
                )?;
            }
            
        }
        Ok(())
    }
}


impl PrintProgramError for AuctionError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            AuctionError::AlreadyInUse => msg!("Error: Farm account already in use"),
            AuctionError::InvalidProgramAddress => {
                msg!("Error: Invalid program address generated from nonce and key")
            }
            AuctionError::InvalidOwner => {
                msg!("Error: The input account owner is not the program address")
            }
            AuctionError::InvalidOutputOwner => {
                msg!("Error: Output pool account owner cannot be the program address")
            }
            AuctionError::ExpectedMint => msg!("Error: Deserialized account is not an SPL Token mint"),
            AuctionError::ExpectedAccount => {
                msg!("Error: Deserialized account is not an SPL Token account")
            }
            AuctionError::InvalidOutput => msg!("Error: InvalidOutput"),
            AuctionError::CalculationFailure => msg!("Error: CalculationFailure"),
            AuctionError::InvalidInstruction => msg!("Error: InvalidInstruction"),
            AuctionError::IncorrectFeeAccount => msg!("Error: Pool fee token account incorrect"),
            AuctionError::FeeCalculationFailure => msg!(
                "Error: The fee calculation failed due to overflow, underflow, or unexpected 0"
            ),
            AuctionError::ConversionFailure => msg!("Error: Conversion to or from u64 failed."),
            AuctionError::InvalidFee => {
                msg!("Error: The provided fee does not match the program owner's constraints")
            }
            AuctionError::IncorrectTokenProgramId => {
                msg!("Error: The provided token program does not match the token program expected by the auction")
            }
            AuctionError::Ended => {
                msg!("The auction was ended")
            }
            AuctionError::Canceled => {
                msg!("The auction was canceled")
            }
        }
    }
}


fn to_u128(val: u64) -> Result<u128, AuctionError> {
    val.try_into().map_err(|_| AuctionError::ConversionFailure)
}

fn to_u64(val: u128) -> Result<u64, AuctionError> {
    val.try_into().map_err(|_| AuctionError::ConversionFailure)
}
