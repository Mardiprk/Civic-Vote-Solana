use anchor_lang::prelude::*;

declare_id!("8YfCp7xf5XnFh6aAXdMskLs1cya4puZtFWcaUSzsbPnY");

#[program]
pub mod civic_vote {
    use super::*;

    pub fn initialize_election(ctx: Context<InitializeElection>, start_ts: i64, end_ts: i64, vote_fee_lamports: u64) -> Result<()> {
        let clock = Clock::get()?;

        require!(end_ts > start_ts, VoteError::InvalidTimeRange);
        require!(start_ts >= clock.unix_timestamp, VoteError::StartInPast);

        let election = &mut ctx.accounts.election;

        election.authority = ctx.accounts.authority.key();
        election.start_ts = start_ts;
        election.end_ts = end_ts;
        election.vote_fee_lamports = vote_fee_lamports;
        election.total_votes = 0;
        election.bump = ctx.bumps.election;
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(start_ts: i64, end_ts: i64)]
pub struct InitializeElection<'info>{
    #[account(
        init,
        payer = authority,
        space = 8 + ElectionConfig::INIT_SPACE,
        seeds = [b"election", authority.key().as_ref()],
        bump
    )]
    pub election: Account<'info, ElectionConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>
}

#[account]
#[derive(InitSpace)]
pub struct ElectionConfig{
    pub authority: Pubkey, // who can add parties
    pub start_ts: i64,   // prevents early voting
    pub end_ts: i64,    // hard stop
    pub vote_fee_lamports: u64, // voting fees
    pub total_votes: u64,
    pub bump: u8 // PDA validation
}

#[error_code]
pub enum VoteError {
    #[msg("End time must be after start time")]
    InvalidTimeRange,
    #[msg("Start time cannot be in the past")]
    StartInPast,
}
