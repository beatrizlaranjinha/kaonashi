use anchor_lang::prelude::*;

use crate::{
    crypto::{encrypted_tally_after_vote, validate_ciphertexts},
    ErrorCode, SubmitRollupBatchAccounts,
};

// Applies an off-chain rollup batch to the on-chain ballot state.
pub fn submit_rollup_batch(
    ctx: Context<SubmitRollupBatchAccounts>,
    new_merkle_root: [u8; 32],
    encrypted_batch_tally: Vec<[u8; 64]>,
    batch_size: u64,
) -> Result<()> {
    let ballot = &mut ctx.accounts.ballot;

    require!(batch_size > 0, ErrorCode::InvalidBatchSize);

    require!(
        encrypted_batch_tally.len() == ballot.proposal_count as usize,
        ErrorCode::InvalidTallySize
    );

    validate_ciphertexts(&encrypted_batch_tally)?;

    ballot.encrypted_tally =
        encrypted_tally_after_vote(&ballot.encrypted_tally, &encrypted_batch_tally)?;

    ballot.merkle_root = new_merkle_root;

    ballot.total_votes = ballot
        .total_votes
        .checked_add(batch_size)
        .ok_or(error!(ErrorCode::MathOverflow))?;

    ballot.batch_count = ballot
        .batch_count
        .checked_add(1)
        .ok_or(error!(ErrorCode::MathOverflow))?;

    Ok(())
}
