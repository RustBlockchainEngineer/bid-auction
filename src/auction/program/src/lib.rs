//! A program for creating and managing farms
//! by hongbo
pub mod error;
pub mod instruction;
pub mod processor;
pub mod fees;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("AuctionuN9wGpxNjGnPNbRPtpQ7mHgKM8d9BeFC549Jy");

// test code
#[cfg(test)]
mod test {
    #[test]
    fn test_sanity() {
        assert_eq!(1,1);
    }
}
