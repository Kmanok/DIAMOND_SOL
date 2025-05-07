use anchor_lang::prelude::*;

// Token configuration
pub const DECIMALS: u8 = 9;
pub const USDT_DECIMALS: u8 = 6;
pub const USDC_DECIMALS: u8 = 6;
pub const SOL_DECIMALS: u8 = 9;
pub const INITIAL_SUPPLY: u64 = 8_000_000 * 10u64.pow(DECIMALS as u32); // 8 million tokens
pub const MAX_SUPPLY: u64 = 100_000_000 * 10u64.pow(DECIMALS as u32); // 100 million tokens

// Pricing
pub const TOKEN_PRICE_USDT: u64 = 1_000_000; // 1 USDT
pub const TOKEN_PRICE_USDC: u64 = 800_000; // 0.8 USDC
pub const TOKEN_PRICE_SOL: u64 = 0; // To be set based on oracle price
pub const MIN_PURCHASE_AMOUNT: u64 = 1_000_000; // 1 USDT
pub const MAX_PURCHASE_AMOUNT: u64 = 1_000_000_000; // 1000 USDT
pub const MIN_PURCHASE_USDC: u64 = 1_000_000; // 1 USDC
pub const MIN_PURCHASE_SOL: u64 = 1_000_000; // 0.001 SOL

// Price Oracle
pub const PYTH_SOL_USD_PRICE_FEED: &str = "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG";
pub const PRICE_CONFIDENCE_THRESHOLD: u64 = 100; // 1%
pub const MAX_PRICE_AGE: i64 = 60; // 60 seconds

// Time constants
pub const PAUSE_COOLDOWN: i64 = 900; // 15 minutes in seconds

// PDA seeds
pub const TOKEN_STATE_SEED: &[u8] = b"token_state";
pub const BLACKLIST_SEED: &[u8] = b"blacklist";
pub const VAULT_SEED: &[u8] = b"vault";
pub const MULTISIG_SEED: &[u8] = b"multisig"; 