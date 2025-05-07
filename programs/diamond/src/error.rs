use anchor_lang::prelude::*;

#[error_code]
pub enum DiamondTokenError {
    #[msg("Invalid multisig threshold")]
    InvalidMultisigThreshold,

    #[msg("Invalid token state PDA")]
    InvalidTokenState,

    #[msg("Invalid blacklist PDA")]
    InvalidBlacklist,

    #[msg("Address is blacklisted")]
    AddressBlacklisted,

    #[msg("Source address is blacklisted")]
    SourceAddressBlacklisted,

    #[msg("Destination address is blacklisted")]
    DestinationAddressBlacklisted,

    #[msg("Address is already blacklisted")]
    AddressAlreadyBlacklisted,

    #[msg("Address is not blacklisted")]
    AddressNotBlacklisted,

    #[msg("Blacklist is full")]
    BlacklistFull,

    #[msg("Insufficient balance")]
    InsufficientBalance,

    #[msg("Insufficient reserve")]
    InsufficientReserve,

    #[msg("Invalid amount")]
    InvalidAmount,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Purchase amount is too small")]
    PurchaseAmountTooSmall,

    #[msg("Max supply would be exceeded")]
    MaxSupplyExceeded,

    #[msg("Cannot increase max supply")]
    CannotIncreaseMaxSupply,

    #[msg("Invalid max supply")]
    InvalidMaxSupply,

    #[msg("Max supply reduction too large")]
    MaxSupplyReductionTooLarge,

    #[msg("Token operations are paused")]
    Paused,

    #[msg("Pause cooldown has not elapsed")]
    PauseCooldownNotElapsed,

    #[msg("Invalid token decimals")]
    InvalidDecimals,

    #[msg("Math operation overflow")]
    MathOverflow,

    #[msg("Invalid price feed")]
    InvalidPriceFeed,

    #[msg("Price feed is stale")]
    StalePrice,

    #[msg("Token is already paused")]
    AlreadyPaused,

    #[msg("Token is not paused")]
    NotPaused,
}
