use anchor_lang::prelude::*;

use crate::{Ballot, ErrorCode, NO_FINAL_WINNER};

pub const ELECTION_OPEN: u8 = 0;
pub const ELECTION_CLOSED: u8 = 1;
pub const ELECTION_FINALIZED: u8 = 2;

pub fn ensure_open(ballot: &Ballot) -> Result<()> {
    require!(ballot.status == ELECTION_OPEN, ErrorCode::ElectionClosed);

    Ok(())
}

pub fn close(ballot: &mut Ballot) -> Result<()> {
    require!(ballot.status == ELECTION_OPEN, ErrorCode::ElectionNotOpen);

    ballot.status = ELECTION_CLOSED;

    Ok(())
}

pub fn finalize(ballot: &mut Ballot, winner_index: u8) -> Result<()> {
    require!(
        ballot.status == ELECTION_CLOSED,
        ErrorCode::ElectionNotClosed
    );

    require!(
        ballot.final_winner_index == NO_FINAL_WINNER,
        ErrorCode::WinnerAlreadySet
    );

    require!(
        ballot.is_valid_proposal_index(winner_index),
        ErrorCode::InvalidProposalIndex
    );

    ballot.final_winner_index = winner_index;
    ballot.status = ELECTION_FINALIZED;

    Ok(())
}
