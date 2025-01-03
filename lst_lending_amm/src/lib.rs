use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Mint, Token, TokenAccount, Transfer},
};
use pyth_sdk_solana::{PriceFeed, Price};

declare_id!("2FYzABhg8gz1RBnxiqngxJt2D6Z5ooafLtiV2EsNzpEK");

/// LST Lending AMM with Collateralized Swaps
#[program]
pub mod lst_lending_amm {
    use super::*;

    /// Deposit LSTs as collateral
    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        require!(!ctx.accounts.global_state.paused, LendingError::ProtocolPaused);

        // Transfer LST tokens to the collateral vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_lst_account.to_account_info(),
            to: ctx.accounts.collateral_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Update user's collateral balance
        ctx.accounts.user_collateral_account.collateral_amount += amount;

        // Update protocol stats
        ctx.accounts.protocol_stats.total_collateral += amount;

        emit!(CollateralDeposited {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }

    /// Borrow assets using LST collateral
    pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64) -> Result<()> {
        require!(!ctx.accounts.global_state.paused, LendingError::ProtocolPaused);

        let price = get_price(&ctx.accounts.price_feed, &ctx.accounts.switchboard_feed)?;
        let lst_value_in_usd = ctx.accounts.user_collateral_account.collateral_amount * price as u64;

        let required_collateral = borrow_amount * 2; // 200% collateral ratio

        require!(
            lst_value_in_usd >= required_collateral,
            LendingError::InsufficientCollateral
        );

        // Calculate interest rate based on pool utilization
        let utilization = (ctx.accounts.protocol_stats.total_borrowed * 100) / ctx.accounts.protocol_stats.total_collateral;
        let interest_rate = calculate_interest_rate(utilization);

        // Transfer borrowed tokens to user
        let cpi_accounts = Transfer {
            from: ctx.accounts.lending_pool.to_account_info(),
            to: ctx.accounts.user_borrow_account.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, borrow_amount)?;

        ctx.accounts.user_debt_account.debt_amount += borrow_amount + (borrow_amount * interest_rate / 100);

        // Update protocol stats
        ctx.accounts.protocol_stats.total_borrowed += borrow_amount;

        emit!(AssetBorrowed {
            user: ctx.accounts.user.key(),
            borrow_amount,
        });

        Ok(())
    }

    /// Liquidate under-collateralized positions (supports partial liquidation)
    pub fn liquidate(ctx: Context<Liquidate>, repay_amount: u64) -> Result<()> {
        require!(!ctx.accounts.global_state.paused, LendingError::ProtocolPaused);

        let price = get_price(&ctx.accounts.price_feed, &ctx.accounts.switchboard_feed)?;
        let lst_value_in_usd = ctx.accounts.borrower_collateral_account.collateral_amount * price as u64;

        let debt_value = ctx.accounts.borrower_debt_account.debt_amount;

        let health_factor = calculate_health_factor(
            ctx.accounts.borrower_collateral_account.collateral_amount,
            price as u64,
            debt_value,
            200,
        );

        require!(health_factor < 1, LendingError::PositionStillSafe);

        let liquidation_amount = repay_amount.min(ctx.accounts.borrower_debt_account.debt_amount);

        // Swap borrowed assets for LST collateral using AMM logic
        let cpi_accounts = Transfer {
            from: ctx.accounts.collateral_vault.to_account_info(),
            to: ctx.accounts.amm_pool.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, liquidation_amount)?;

        ctx.accounts.borrower_debt_account.debt_amount -= liquidation_amount;
        ctx.accounts.borrower_collateral_account.collateral_amount -= liquidation_amount;

        // Provide liquidation bonus
        let liquidation_bonus = liquidation_amount / 100; // 1% bonus
        let cpi_bonus_accounts = Transfer {
            from: ctx.accounts.collateral_vault.to_account_info(),
            to: ctx.accounts.liquidator.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_bonus_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_bonus_accounts);
        token::transfer(cpi_bonus_ctx, liquidation_bonus)?;

        emit!(PositionLiquidated {
            borrower: ctx.accounts.borrower.key(),
            liquidator: ctx.accounts.liquidator.key(),
            swapped_amount: liquidation_amount,
        });

        Ok(())
    }
}

/// --- Utility Functions ---
/*fn get_price(pyth_feed: &AccountInfo, switchboard_feed: &AccountInfo) -> Result<u64> {
    let pyth_price_feed = pyth_sdk_solana::load_price_feed_from_account_info(pyth_feed)
        .map_err(|_| error!(LendingError::InvalidOracle))?;
    
    let mut price = pyth_price_feed.get_price_no_older_than(60, Clock::get()?.unix_timestamp)
        .map_err(|_| error!(LendingError::InvalidOracle))?;
    
    if price.price == 0 {
        let switchboard_price_feed = pyth_sdk_solana::load_price_feed_from_account_info(switchboard_feed)
            .map_err(|_| error!(LendingError::InvalidOracle))?;
        price = switchboard_price_feed.get_price_no_older_than(60, Clock::get()?.unix_timestamp)
            .map_err(|_| error!(LendingError::InvalidOracle))?;
    }
    Ok(price.price as u64)
}*/

fn get_price(pyth_feed: &AccountInfo, switchboard_feed: &AccountInfo) -> Result<u64> {
    let pyth_price_feed = pyth_sdk_solana::load_price_feed_from_account_info(pyth_feed)
        .map_err(|_| LendingError::InvalidOracle)?;
    
    // Convert unix_timestamp to u64 using try_into()
    let timestamp = Clock::get()
        .map_err(|_| LendingError::InvalidOracle)?
        .unix_timestamp
        .try_into()
        .map_err(|_| LendingError::InvalidOracle)?;

    // First try Pyth price feed
    let price = match pyth_price_feed.get_price_no_older_than(60, timestamp) {
        Some(price) => price,
        None => {
            // If Pyth price is not available, try Switchboard
            let switchboard_price_feed = pyth_sdk_solana::load_price_feed_from_account_info(switchboard_feed)
                .map_err(|_| LendingError::InvalidOracle)?;
            
            switchboard_price_feed
                .get_price_no_older_than(60, timestamp)
                .ok_or(LendingError::InvalidOracle)?
        }
    };

    Ok(price.price as u64)
}

pub fn get_price_instruction(ctx: Context<GetPrice>) -> Result<u64> {
    let price = get_price(&ctx.accounts.price_feed, &ctx.accounts.switchboard_feed)?;
    msg!("Oracle Price Fetched: {}", price);
    Ok(price)
}


fn calculate_health_factor(collateral: u64, collateral_price: u64, debt: u64, liquidation_threshold: u64) -> u64 {
    if debt == 0 {
        return u64::MAX;
    }
    (collateral * collateral_price) / (debt * liquidation_threshold)
}

fn calculate_interest_rate(utilization: u64) -> u64 {
    if utilization < 80 {
        5 // Base interest rate of 5%
    } else {
        10 + (utilization - 80) // Increase interest dynamically
    }
}

/// --- Error Handling ---
#[error_code]
pub enum LendingError {
    #[msg("Insufficient collateral to borrow the requested amount.")]
    InsufficientCollateral,
    #[msg("Position is still safe, cannot liquidate.")]
    PositionStillSafe,
    #[msg("The protocol is currently paused.")]
    ProtocolPaused,
  #[msg("Invalid oracle price feed")]
    InvalidOracle,
}


/// --- Events ---
#[event]
pub struct CollateralDeposited {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct AssetBorrowed {
    pub user: Pubkey,
    pub borrow_amount: u64,
}

#[event]
pub struct PositionLiquidated {
    pub borrower: Pubkey,
    pub liquidator: Pubkey,
    pub swapped_amount: u64,
}

/// --- Account Structures ---
#[account]
pub struct CollateralAccount {
    pub collateral_amount: u64,
}

#[account]
pub struct DebtAccount {
    pub debt_amount: u64,
}

#[account]
pub struct GlobalState {
    pub paused: bool,
    pub admin: Pubkey,
}

#[account]
pub struct ProtocolStats {
    pub total_collateral: u64,
    pub total_borrowed: u64,
    pub total_liquidations: u64,
}
#[derive(Accounts)]
pub struct GetPrice<'info> {
    pub price_feed: AccountInfo<'info>,
    pub switchboard_feed: AccountInfo<'info>,
}

/// --- Accounts and Contexts ---
#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_lst_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub collateral_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_collateral_account: Account<'info, CollateralAccount>,
    /// Added for protocol stats tracking
    #[account(mut)]
    pub protocol_stats: Account<'info, ProtocolStats>,
    /// Added for pause check
    pub global_state: Account<'info, GlobalState>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_borrow_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub lending_pool: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_debt_account: Account<'info, DebtAccount>,
    #[account(mut)]
    pub user_collateral_account: Account<'info, CollateralAccount>,
    #[account(mut)]
    pub pool_authority: Signer<'info>,
    /// Added Switchboard feed as fallback
   //  pub switchboard_feed: Account<'info, PriceFeed>,
    // pub price_feed: Account<'info, PriceFeed>,
    pub switchboard_feed: AccountInfo<'info>,
    pub price_feed: AccountInfo<'info>,
    /// Added for protocol stats tracking
    #[account(mut)]
    pub protocol_stats: Account<'info, ProtocolStats>,
    /// Added for pause check
    pub global_state: Account<'info, GlobalState>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,
    #[account(mut)]
    pub borrower: AccountInfo<'info>,
    #[account(mut)]
    pub borrower_collateral_account: Account<'info, CollateralAccount>,
    #[account(mut)]
    pub borrower_debt_account: Account<'info, DebtAccount>,
    #[account(mut)]
    pub collateral_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub amm_pool: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_authority: Signer<'info>,
    /// Added Switchboard feed as fallback
  //  pub switchboard_feed: Account<'info, PriceFeed>,
   // pub price_feed: Account<'info, PriceFeed>,
    pub switchboard_feed: AccountInfo<'info>,
    pub price_feed: AccountInfo<'info>,
    /// Added for pause check
    pub global_state: Account<'info, GlobalState>,
    pub token_program: Program<'info, Token>,
}

