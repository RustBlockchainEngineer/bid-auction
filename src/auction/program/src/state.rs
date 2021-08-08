//! State transition types
//! by hongbo
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use enum_dispatch::enum_dispatch;
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
    clock::UnixTimestamp,
};
use crate::fees::AuctionFees;

/// Trait representing access to program state across all versions
#[enum_dispatch]
pub trait AuctionState {
    /// Is the auction initialized, with data written to it
    fn is_initialized(&self) -> bool;
    /// Token program ID associated with the auction
    fn token_program_id(&self) -> &Pubkey;
    /// Address of owner token account
    fn token_account(&self) -> &Pubkey;
    /// Address of pool token account
    fn pool(&self) -> &Pubkey;

    /// Address of pool fee account
    fn fee_account(&self) -> &Pubkey;

    /// Fees associated with auction
    fn fees(&self) -> &AuctionFees;

    fn nonce(&self) -> u8;

    fn start_timestamp(&self) -> UnixTimestamp; 
    fn end_timestamp(&self) -> UnixTimestamp; 

    fn canceled(&self) -> u8;
}

/// All versions of AuctionState
#[enum_dispatch(AuctionState)]
pub enum AuctionVersion {
    /// Latest version, used for all new auctions
    AuctionV1,
}

/// AuctionVersion does not implement program_pack::Pack because there are size
/// checks on pack and unpack that would break backwards compatibility, so
/// special implementations are provided here
impl AuctionVersion {
    /// Size of the latest version of the AuctionState
    pub const LATEST_LEN: usize = 1 + AuctionV1::LEN; // add one for the version enum

    /// Pack a auction into a byte array, based on its version
    pub fn pack(src: Self, dst: &mut [u8]) -> Result<(), ProgramError> {
        match src {
            Self::AuctionV1(auction_info) => {
                dst[0] = 1;
                AuctionV1::pack(auction_info, &mut dst[1..])
            }
        }
    }

    /// Unpack the auction account based on its version, returning the result as a
    /// AuctionState trait object
    pub fn unpack(input: &[u8]) -> Result<Box<dyn AuctionState>, ProgramError> {
        let (&version, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidAccountData)?;
        match version {
            1 => Ok(Box::new(AuctionV1::unpack(rest)?)),
            _ => Err(ProgramError::UninitializedAccount),
        }
    }

    /// Special check to be done before any instruction processing, works for
    /// all versions
    pub fn is_initialized(input: &[u8]) -> bool {
        match Self::unpack(input) {
            Ok(auction) => auction.is_initialized(),
            Err(_) => false,
        }
    }
}

/// Program states.
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct AuctionV1 {
    /// Initialized state.
    pub is_initialized: bool,

    /// Program ID of the tokens being exchanged.
    pub token_program_id: Pubkey,

    /// owner Token account
    pub token: Pubkey,

    /// Pool token account
    pub pool: Pubkey,

    /// Pool token account to receive trading and / or withdrawal fees
    pub fee_account: Pubkey,

    // All auction fee information
    pub fees: AuctionFees,

    /// owner Token account
    pub nonce: u8,

    pub start_timestamp: UnixTimestamp,
    pub end_timestamp: UnixTimestamp,

    pub canceled: u8,
}

impl AuctionState for AuctionV1 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    fn token_program_id(&self) -> &Pubkey {
        &self.token_program_id
    }

    fn token_account(&self) -> &Pubkey {
        &self.token
    }

    fn pool(&self) -> &Pubkey {
        &self.pool
    }

    fn fee_account(&self) -> &Pubkey {
        &self.fee_account
    }

    fn fees(&self) -> &AuctionFees {
        &self.fees
    }

    fn nonce(&self) -> u8 {
        self.nonce
    }

    fn start_timestamp(&self) -> UnixTimestamp {
        self.start_timestamp
    }

    fn end_timestamp(&self) -> UnixTimestamp {
        self.end_timestamp
    }

    fn canceled(&self) -> u8 {
        self.canceled
    }

}

impl Sealed for AuctionV1 {}
impl IsInitialized for AuctionV1 {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for AuctionV1 {
    const LEN: usize = 323;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 289];
        let (
            is_initialized,
            token_program_id,
            token,
            pool,
            fee_account,
            fees,
            nonce,
            start_timestamp,
            end_timestamp,
            canceled,
        ) = mut_array_refs![output, 1, 32, 32, 32, 32, 16, 8, 64, 64,8];
        is_initialized[0] = self.is_initialized as u8;
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
        token.copy_from_slice(self.token.as_ref());
        pool.copy_from_slice(self.pool.as_ref());
        fee_account.copy_from_slice(self.fee_account.as_ref());
        self.fees.pack_into_slice(&mut fees[..]);
        nonce[0] = self.nonce as u8;
        start_timestamp.copy_from_slice(&self.start_timestamp.to_le_bytes());
        end_timestamp.copy_from_slice(&self.end_timestamp.to_le_bytes());
        canceled[0] = self.canceled as u8;
    }

    /// Unpacks a byte buffer into a [SwapV1](struct.SwapV1.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 289];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            token_program_id,
            token,
            pool,
            fee_account,
            fees,
            nonce,
            start_timestamp,
            end_timestamp,
            canceled,
        ) = array_refs![input, 1, 32, 32, 32, 32, 16, 8, 64,64,8];
        Ok(Self {
            is_initialized: match is_initialized {
                [0] => false,
                [1] => true,
                _ => return Err(ProgramError::InvalidAccountData),
            },
            token_program_id: Pubkey::new_from_array(*token_program_id),
            token: Pubkey::new_from_array(*token),
            pool: Pubkey::new_from_array(*pool),
            fee_account: Pubkey::new_from_array(*fee_account),
            fees: AuctionFees::unpack_from_slice(fees)?,
            nonce : nonce[0],
            start_timestamp: start_timestamp[0] as i64,
            end_timestamp: end_timestamp[0] as i64,
            canceled: canceled[0],
        })
    }
}