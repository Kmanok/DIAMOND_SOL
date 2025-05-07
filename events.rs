use anchor_lang::prelude::*;

#[event]
pub struct TokenStateInitialized {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub initial_supply: u64,
    pub max_supply: u64,
    pub multisig: Pubkey,
}

#[event]
pub struct TokenMinted {
    pub minter: Pubkey,
    pub amount: u64,
    pub payment_token: Pubkey,
    pub payment_amount: u64,
}

#[event]
pub struct TokenBurned {
    pub amount: u64,
    pub refund_amount: u64,
    pub authority: Pubkey,
}

#[event]
pub struct TokensPaused {
    pub timestamp: i64,
    pub authority: Pubkey,
}

#[event]
pub struct TokensUnpaused {
    pub timestamp: i64,
    pub authority: Pubkey,
}

#[event]
pub struct MaxSupplyUpdated {
    pub old_supply: u64,
    pub new_supply: u64,
    pub authority: Pubkey,
}

#[event]
pub struct BlacklistUpdated {
    pub address: Pubkey,
    pub is_blacklisted: bool,
    pub authority: Pubkey,
}

#[event]
pub struct ItemPurchased {
    pub buyer: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
    pub vault_balance: u64,
}

#[event]
pub struct TransferHookExecuted {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ReserveVerified {
    pub total_supply: u64,
    pub expected_usdt: u64,
    pub actual_usdt: u64,
    pub timestamp: i64,
} 