use anchor_lang::prelude::*;
use solana_program::program_pack::Pack;

#[account]
pub struct TokenState {
    pub authority: Pubkey,      // 32 bytes
    pub mint: Pubkey,          // 32 bytes
    pub total_supply: u64,     // 8 bytes
    pub max_supply: u64,       // 8 bytes
    pub is_paused: bool,       // 1 byte
    pub last_pause_timestamp: i64, // 8 bytes
    pub multisig: Pubkey,      // 32 bytes
    pub vault: Pubkey,         // 32 bytes
    pub bump: u8,              // 1 byte
}

impl TokenState {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        32 + // mint
        8 + // total_supply
        8 + // max_supply
        1 + // is_paused
        8 + // last_pause_timestamp
        32 + // multisig
        32 + // vault
        1; // bump

    pub fn is_admin(&self, admin: &Pubkey) -> bool {
        self.authority == *admin
    }
}

#[account]
pub struct Blacklist {
    pub addresses: Vec<Pubkey>, // Vector of blacklisted addresses
    pub bump: u8,              // 1 byte
}

impl Blacklist {
    pub fn space(max_addresses: usize) -> usize {
        8 + // discriminator
        4 + // vec length
        max_addresses * 32 + // addresses
        1 // bump
    }
} 