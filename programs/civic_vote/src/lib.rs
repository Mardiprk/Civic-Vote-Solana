use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction::{transfer};
use anchor_lang::solana_program::program::invoke;

declare_id!("8YfCp7xf5XnFh6aAXdMskLs1cya4puZtFWcaUSzsbPnY");

const MAX_PARTIES: usize = 10;

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

    pub fn add_party(ctx: Context<AddParty>, party_id: u8, name: String) -> Result<()>{
        let election = &ctx.accounts.election;
        let party = &mut ctx.accounts.party;
        let clock = Clock::get()?;

        //prevent initilization after voting starts
        require!(
            clock.unix_timestamp < election.start_ts,
            VoteError::ElectionAlreadyStarted
        );

        require!(!name.is_empty(), VoteError::InvalidPartyName);
        require!(name.len() <= 50, VoteError::InvalidPartyName);

        party.election = election.key();
        party.party_id = party_id;
        party.name = name;
        party.bump = ctx.bumps.party;

        Ok(())
    }

    pub fn init_state_votes(ctx: Context<InitStateVotes>, state_id: u8) -> Result<()>{
        let election = &ctx.accounts.election;
        let state_votes = &mut ctx.accounts.state_votes;
        let clock = Clock::get()?;

        // valid state ID
        require!(state_id <= 35, VoteError::InvalidStateId);

        //prevent initilization after voting starts
        require!(
            clock.unix_timestamp < election.start_ts,
            VoteError::ElectionAlreadyStarted
        );

        state_votes.election = election.key();
        state_votes.state_id = state_id;
        state_votes.total_votes = 0;
        state_votes.bump = ctx.bumps.state_votes;

        
        for i in 0..MAX_PARTIES{
            state_votes.votes[i] = 0;
        }

        Ok(())
    }

    pub fn cast_vote(ctx: Context<CasteVote>, state_id: u8, party_id: u8) -> Result<()>{
        let election = &mut ctx.accounts.election;
        let state_votes = &mut ctx.accounts.state_votes;
        let vote_record = &mut ctx.accounts.voter_record;
        let voter = &ctx.accounts.voter;
        let clock = Clock::get()?;

        // time window checks
        require!(clock.unix_timestamp <= election.start_ts, VoteError::VotingNotStarted);
        require!(clock.unix_timestamp <= election.end_ts, VoteError::VotingEnded);

        require!(state_votes.state_id == state_id, VoteError::InvalidStateId);
        require!((party_id as usize ) < MAX_PARTIES, VoteError::InvalidPartyId);

        // transfer vote fee
        let ix = transfer(&voter.key(), &election.authority, election.vote_fee_lamports);
        
        invoke(&ix, &[
            voter.to_account_info(),
            ctx.accounts.system_program.to_account_info()   
        ])?;

        state_votes.votes[party_id as usize] = state_votes.votes[party_id as usize]
            .checked_add(1)
            .ok_or(VoteError::VoteOverflow)?;
        state_votes.total_votes = state_votes.total_votes
            .checked_add(1)
            .ok_or(VoteError::VoteOverflow)?;
        election.total_votes = election.total_votes
            .checked_add(1)
            .ok_or(VoteError::VoteOverflow)?;

        vote_record.election = election.key();
        vote_record.state_id = state_id;
        vote_record.voter = voter.key();
        vote_record.voted_at = clock.unix_timestamp;
        vote_record.bump = ctx.bumps.voter_record;

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

#[derive(Accounts)]
#[instruction(party_id: u8)]
pub struct AddParty<'info>{
    #[account(
        init,
        payer = authority,
        space = 8 + Party::INIT_SPACE,
        seeds = [b"party", election.key().as_ref(), &[party_id]],
        bump
    )]
    pub party: Account<'info, Party>,

    #[account(
        mut,
        seeds = [b"election", authority.key().as_ref()],
        bump = election.bump,
        has_one = authority        
    )]
    pub election: Account<'info, ElectionConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(state_id: u8)]
pub struct InitStateVotes<'info>{  
    #[account(
        init,
        payer = authority,
        space = 8 + StateVotes::INIT_SPACE,
        seeds = [
            b"state_votes",
            election.key().as_ref(),
            &[state_id]
        ],
        bump

    )]
    pub state_votes: Account<'info, StateVotes>,

    #[account(
        mut,
        seeds = [b"election", authority.key().as_ref()],
        bump = election.bump,
        has_one = authority
    )]
    pub election: Account<'info, ElectionConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(state_id: u8)]
pub struct CasteVote<'info>{
    #[account(
        mut,
        seeds = [b"election", election.key().as_ref()],
        bump = election.bump,
    )]
    pub election: Account<'info, ElectionConfig>,

    #[account(
        mut,
        seeds = [
            b"state_votes",
            election.key().as_ref(),
            &[state_id]
        ],
        bump = state_votes.bump
    )]
    pub state_votes: Account<'info, StateVotes>,

    #[account(
        init,
        payer = voter,
        space = 8 + VoteRecord::INIT_SPACE,
        seeds = [ 
            b"vote_record", election.key().as_ref(), &[state_id], voter.key().as_ref()
        ],
        bump
    )]
    pub voter_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,

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

#[account]
#[derive(InitSpace)]
pub struct Party{
    pub election: Pubkey,
    pub party_id: u8,
    #[max_len(50)]
    pub name: String,
    pub bump: u8
}

#[account]
#[derive(InitSpace)]
pub struct StateVotes{
    pub election: Pubkey,
    pub state_id: u8, // 0..35
    pub votes: [u64; MAX_PARTIES], // votes per partyy
    pub total_votes: u64, // total votes in this state
    pub bump: u8 // PDA Bump
}

#[account]
#[derive(InitSpace)]
pub struct VoteRecord{
    pub election: Pubkey,
    pub state_id: u8, // 0..35
    pub voter: Pubkey,
    pub voted_at: i64,
    pub bump: u8
}

#[error_code]
pub enum VoteError {
    #[msg("End time must be after start time")]
    InvalidTimeRange,
    #[msg("Start time cannot be in the past")]
    StartInPast,
    #[msg("Election has already started")]
    ElectionAlreadyStarted,
    #[msg("Invalid party name")]
    InvalidPartyName,
    #[msg("Invalid state ID")]
    InvalidStateId,
    #[msg("Voting has not started")]
    VotingNotStarted,
    #[msg("Voting has ended")]
    VotingEnded,
    #[msg("Invalid party ID")]
    InvalidPartyId,
    #[msg("Vote counter overflow")]
    VoteOverflow,
}
