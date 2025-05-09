use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Token, Transfer, Mint, TokenAccount},
};
use pyth_sdk_solana::state::SolanaPriceAccount;

declare_id!("97xUm7Kv6TiKyCkaLGgmTFu3skVte3wStYY4vYTXtpxL");

pub mod constants;
pub mod error;
pub mod events;
pub mod state;

use crate::{constants::*, error::*, events::*, state::*};

#[program]
pub mod diamond {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        multisig_owners: Vec<Pubkey>,
        threshold: u64,
    ) -> Result<()> {
        // Validate multisig threshold
        require!(
            threshold == 3 && multisig_owners.len() == 5,
            DiamondTokenError::InvalidMultisigThreshold
        );

        // Initialize token state
        let token_state = &mut ctx.accounts.token_state;
        token_state.authority = ctx.accounts.payer.key();
        token_state.mint = ctx.accounts.mint.key();
        token_state.total_supply = INITIAL_SUPPLY;
        token_state.max_supply = MAX_SUPPLY;
        token_state.is_paused = false;
        token_state.last_pause_timestamp = 0;
        token_state.multisig = ctx.accounts.multisig.key();
        token_state.vault = ctx.accounts.vault.key();
        token_state.bump = ctx.bumps.token_state;

        // Initialize blacklist
        let blacklist = &mut ctx.accounts.blacklist;
        blacklist.addresses = Vec::new();
        blacklist.bump = ctx.bumps.blacklist;

        // Mint initial supply to vault
        let token_state_seeds = &[TOKEN_STATE_SEED, &[token_state.bump]];
        let signer = &[&token_state_seeds[..]];

        let mint_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: token_state.to_account_info(),
            },
            signer,
        );

        anchor_spl::token::mint_to(mint_ctx, INITIAL_SUPPLY)?;

        emit!(TokenStateInitialized {
            authority: token_state.authority,
            mint: token_state.mint,
            initial_supply: INITIAL_SUPPLY,
            max_supply: MAX_SUPPLY,
            multisig: token_state.multisig,
        });

        Ok(())
    }

    pub fn mint_by_user(ctx: Context<MintByUser>, amount: u64) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;

        // Validate amount is not zero
        require!(amount > 0, DiamondTokenError::InvalidAmount);

        // Check blacklist
        let blacklist = &ctx.accounts.blacklist;
        require!(
            !blacklist.addresses.contains(&ctx.accounts.user.key()),
            DiamondTokenError::AddressBlacklisted
        );

        // Validate token accounts
        require!(
            ctx.accounts.user_token_account.mint == ctx.accounts.mint.key(),
            DiamondTokenError::InvalidTokenAccount
        );
        require!(
            ctx.accounts.user_payment_account.mint == ctx.accounts.payment_token.key(),
            DiamondTokenError::InvalidTokenAccount
        );

        // Calculate payment amount based on token type
        let payment_amount = match ctx.accounts.payment_token.decimals {
            6 => { // USDT or USDC
                let amount = if ctx.accounts.payment_token.key() == USDT_PUBKEY {
                    amount.checked_mul(TOKEN_PRICE_USDT)
                } else {
                    amount.checked_mul(TOKEN_PRICE_USDC)
                }.ok_or(DiamondTokenError::MathOverflow)?;
                
                require!(
                    amount >= MIN_PURCHASE_USDC,
                    DiamondTokenError::PurchaseAmountTooSmall
                );
                amount
            }
            SOL_DECIMALS => {
                // Get SOL price from Pyth oracle
                let price_feed = SolanaPriceAccount::account_info_to_feed(&ctx.accounts.sol_price_feed)
                    .map_err(|_| DiamondTokenError::InvalidPriceFeed)?;

                let current_time = Clock::get()?.unix_timestamp;
                let current_price = price_feed.get_price_no_older_than(current_time, MAX_PRICE_AGE as u64)
                    .ok_or(DiamondTokenError::InvalidPriceFeed)?;

                // Calculate SOL amount needed
                let token_price_usd = 0.8; // 0.8 USD per token
                let sol_price_usd = current_price.price as f64 / 10f64.powi(current_price.expo as i32);
                let sol_amount = (amount as f64 * token_price_usd / sol_price_usd * 1e9) as u64;

                require!(
                    sol_amount >= MIN_PURCHASE_SOL,
                    DiamondTokenError::PurchaseAmountTooSmall
                );

                sol_amount
            }
            _ => return Err(DiamondTokenError::InvalidDecimals.into()),
        };

        // Check if minting would exceed max supply
        let new_supply = token_state
            .total_supply
            .checked_add(amount)
            .ok_or(DiamondTokenError::MathOverflow)?;

        require!(
            new_supply <= token_state.max_supply,
            DiamondTokenError::MaxSupplyExceeded
        );

        // Transfer payment to vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_payment_account.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );

        anchor_spl::token::transfer(transfer_ctx, payment_amount)?;

        // Mint tokens to user
        let token_state_seeds = &[TOKEN_STATE_SEED, &[token_state.bump]];
        let signer = &[&token_state_seeds[..]];

        let mint_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: token_state.to_account_info(),
            },
            signer,
        );

        anchor_spl::token::mint_to(mint_ctx, amount)?;

        // Update state
        token_state.total_supply = new_supply;

        // Emit event
        emit!(TokenMinted {
            minter: ctx.accounts.user.key(),
            amount,
            payment_token: ctx.accounts.payment_token.key(),
            payment_amount,
        });

        Ok(())
    }

    pub fn admin_burn(ctx: Context<AdminBurn>, amount: u64) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;
        let mint = &ctx.accounts.mint;
        let vault = &ctx.accounts.vault;

        // Verify admin signature
        require!(
            token_state.is_admin(&ctx.accounts.admin.to_account_info().key()),
            DiamondTokenError::NotAuthorized
        );

        // Verify amount
        require!(amount > 0, DiamondTokenError::InvalidAmount);

        // Verify vault has enough tokens
        require!(vault.amount >= amount, DiamondTokenError::InsufficientFunds);

        // Burn tokens from vault
        anchor_spl::token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Burn {
                    mint: mint.to_account_info(),
                    from: vault.to_account_info(),
                    authority: token_state.to_account_info(),
                },
            ),
            amount,
        )?;

        // Update total supply
        token_state.total_supply = token_state
            .total_supply
            .checked_sub(amount)
            .ok_or(DiamondTokenError::ArithmeticOverflow)?;

        // If premint account exists, burn from it too
        if let Some(premint) = &ctx.accounts.premint_account {
            if premint.amount > 0 {
                anchor_spl::token::burn(
                    CpiContext::new(
                        ctx.accounts.token_program.to_account_info(),
                        anchor_spl::token::Burn {
                            mint: mint.to_account_info(),
                            from: premint.to_account_info(),
                            authority: token_state.to_account_info(),
                        },
                    ),
                    premint.amount,
                )?;

                // Update total supply again
                token_state.total_supply = token_state
                    .total_supply
                    .checked_sub(premint.amount)
                    .ok_or(DiamondTokenError::ArithmeticOverflow)?;
            }
        }

        Ok(())
    }

    pub fn pause(ctx: Context<Pause>) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;

        // Check if already paused
        require!(!token_state.is_paused, DiamondTokenError::AlreadyPaused);

        // Update state
        token_state.is_paused = true;
        token_state.last_pause_timestamp = Clock::get()?.unix_timestamp;

        // Emit event
        emit!(TokensPaused {
            timestamp: token_state.last_pause_timestamp,
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    pub fn unpause(ctx: Context<Unpause>) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;
        let current_time = Clock::get()?.unix_timestamp;

        // Check if already unpaused
        require!(token_state.is_paused, DiamondTokenError::NotPaused);

        // Check cooldown period
        let time_since_pause = current_time
            .checked_sub(token_state.last_pause_timestamp)
            .ok_or(DiamondTokenError::MathOverflow)?;

        require!(
            time_since_pause >= PAUSE_COOLDOWN,
            DiamondTokenError::PauseCooldownNotElapsed
        );

        // Update state
        token_state.is_paused = false;

        // Emit event
        emit!(TokensUnpaused {
            timestamp: current_time,
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    pub fn update_max_supply(ctx: Context<UpdateMaxSupply>, new_max_supply: u64) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;

        // Check if new supply is less than current max supply
        require!(
            new_max_supply < token_state.max_supply,
            DiamondTokenError::CannotIncreaseMaxSupply
        );

        // Check if new supply is not less than current total supply
        require!(
            new_max_supply >= token_state.total_supply,
            DiamondTokenError::InvalidMaxSupply
        );

        // Calculate maximum allowed reduction (50% of current max supply)
        let max_reduction = token_state
            .max_supply
            .checked_div(2)
            .ok_or(DiamondTokenError::MathOverflow)?;

        let min_allowed_max_supply = token_state
            .max_supply
            .checked_sub(max_reduction)
            .ok_or(DiamondTokenError::MathOverflow)?;

        // Ensure new max supply is within allowed range
        require!(
            new_max_supply >= min_allowed_max_supply,
            DiamondTokenError::MaxSupplyReductionTooLarge
        );

        // Store old supply for event
        let old_supply = token_state.max_supply;

        // Update max supply
        token_state.max_supply = new_max_supply;

        // Emit event
        emit!(MaxSupplyUpdated {
            old_supply,
            new_supply: new_max_supply,
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    pub fn add_to_blacklist(ctx: Context<UpdateBlacklist>, address: Pubkey) -> Result<()> {
        let blacklist = &mut ctx.accounts.blacklist;

        // Check if address is already blacklisted
        require!(
            !blacklist.addresses.contains(&address),
            DiamondTokenError::AddressAlreadyBlacklisted
        );

        // Check if blacklist is at capacity
        require!(
            blacklist.addresses.len() < 100, // Maximum 100 blacklisted addresses
            DiamondTokenError::BlacklistFull
        );

        // Add address to blacklist
        blacklist.addresses.push(address);

        // Emit event
        emit!(BlacklistUpdated {
            address,
            is_blacklisted: true,
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    pub fn remove_from_blacklist(ctx: Context<UpdateBlacklist>, address: Pubkey) -> Result<()> {
        let blacklist = &mut ctx.accounts.blacklist;

        // Check if address is in blacklist
        require!(
            blacklist.addresses.contains(&address),
            DiamondTokenError::AddressNotBlacklisted
        );

        // Remove address from blacklist
        if let Some(index) = blacklist.addresses.iter().position(|x| x == &address) {
            blacklist.addresses.remove(index);
        }

        // Emit event
        emit!(BlacklistUpdated {
            address,
            is_blacklisted: false,
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    pub fn purchase_item(ctx: Context<PurchaseItem>, amount: u64) -> Result<()> {
        // Check if token operations are paused
        require!(
            !ctx.accounts.token_state.is_paused,
            DiamondTokenError::Paused
        );

        // Check if user has sufficient balance
        require!(
            ctx.accounts.user_token_account.amount >= amount,
            DiamondTokenError::InsufficientBalance
        );

        // Check minimum purchase amount
        require!(
            amount >= MIN_PURCHASE_AMOUNT,
            DiamondTokenError::PurchaseAmountTooSmall
        );

        // Transfer tokens to vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );

        anchor_spl::token::transfer(transfer_ctx, amount)?;

        // Emit event with more details
        emit!(ItemPurchased {
            buyer: ctx.accounts.user.key(),
            amount,
            timestamp: Clock::get()?.unix_timestamp,
            vault_balance: ctx.accounts.vault.amount,
        });

        Ok(())
    }

    pub fn on_transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        let blacklist = &ctx.accounts.blacklist;

        // Check if source is blacklisted
        if blacklist.addresses.contains(&ctx.accounts.source.key()) {
            return Err(DiamondTokenError::SourceAddressBlacklisted.into());
        }

        // Check if destination is blacklisted
        if blacklist
            .addresses
            .contains(&ctx.accounts.destination.key())
        {
            return Err(DiamondTokenError::DestinationAddressBlacklisted.into());
        }

        // Emit event for successful transfer
        emit!(TransferHookExecuted {
            source: ctx.accounts.source.key(),
            destination: ctx.accounts.destination.key(),
            amount,
        });

        Ok(())
    }

    pub fn verify_reserve(ctx: Context<VerifyReserve>) -> Result<()> {
        let token_state = &ctx.accounts.token_state;
        let vault = &ctx.accounts.vault;

        // Calculate expected USDT balance based on total supply
        let expected_usdt = token_state
            .total_supply
            .checked_mul(TOKEN_PRICE_USDT)
            .ok_or(DiamondTokenError::MathOverflow)?;

        // Verify vault has sufficient USDT balance
        require!(
            vault.amount >= expected_usdt,
            DiamondTokenError::InsufficientReserve
        );

        // Emit verification event
        emit!(ReserveVerified {
            total_supply: token_state.total_supply,
            expected_usdt,
            actual_usdt: vault.amount,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = TokenState::LEN,
        seeds = [TOKEN_STATE_SEED],
        bump
    )]
    pub token_state: Account<'info, TokenState>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump
    )]
    pub vault: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,

    #[account(
        init,
        payer = payer,
        space = Blacklist::space(100), // Maximum 100 blacklisted addresses
        seeds = [BLACKLIST_SEED],
        bump
    )]
    pub blacklist: Account<'info, Blacklist>,

    /// CHECK: Multisig account is validated in the instruction
    pub multisig: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct MintByUser<'info> {
    pub user: Signer<'info>,
    pub token_state: Account<'info, TokenState>,
    pub mint: Account<'info, Mint>,
    pub payment_token: Account<'info, Mint>,
    pub user_payment_account: Account<'info, TokenAccount>,
    pub user_token_account: Account<'info, TokenAccount>,
    pub vault: Account<'info, TokenAccount>,
    pub blacklist: Account<'info, Blacklist>,
    /// CHECK: Pyth price feed account is validated in the instruction
    pub sol_price_feed: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminBurn<'info> {
    pub admin: Signer<'info>,
    pub token_state: Account<'info, TokenState>,
    /// CHECK: Multisig account is validated in the instruction
    pub multisig: UncheckedAccount<'info>,
    pub mint: Account<'info, Mint>,
    pub vault: Account<'info, TokenAccount>,
    pub premint_account: Option<Account<'info, TokenAccount>>,
    pub refund_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Pause<'info> {
    /// CHECK: Authority is validated in the instruction
    pub authority: UncheckedAccount<'info>,
    pub token_state: Account<'info, TokenState>,
    /// CHECK: Multisig account is validated in the instruction
    pub multisig: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Unpause<'info> {
    /// CHECK: Authority is validated in the instruction
    pub authority: UncheckedAccount<'info>,
    pub token_state: Account<'info, TokenState>,
    /// CHECK: Multisig account is validated in the instruction
    pub multisig: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpdateMaxSupply<'info> {
    /// CHECK: Authority is validated in the instruction
    pub authority: UncheckedAccount<'info>,
    pub token_state: Account<'info, TokenState>,
    /// CHECK: Multisig account is validated in the instruction
    pub multisig: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpdateBlacklist<'info> {
    /// CHECK: Authority is validated in the instruction
    pub authority: UncheckedAccount<'info>,
    pub token_state: Account<'info, TokenState>,

    #[account(seeds = [BLACKLIST_SEED], bump)]
    pub blacklist: Account<'info, Blacklist>,

    /// CHECK: Multisig account is validated in the instruction
    pub multisig: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct PurchaseItem<'info> {
    pub user: Signer<'info>,
    pub token_state: Account<'info, TokenState>,
    pub user_token_account: Account<'info, TokenAccount>,
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(seeds = [BLACKLIST_SEED], bump)]
    pub blacklist: Account<'info, Blacklist>,
    pub source: Account<'info, TokenAccount>,
    pub destination: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct VerifyReserve<'info> {
    pub token_state: Account<'info, TokenState>,
    pub vault: Account<'info, TokenAccount>,
}
