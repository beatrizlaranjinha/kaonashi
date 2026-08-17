use anchor_lang::prelude::*;

use groth16_solana::groth16::Groth16Verifier;

use crate::{
    crypto::{encrypted_tally_after_vote, validate_ciphertexts},
    groth16_verifying_key::VERIFYING_KEY,
    ErrorCode, SubmitRollupBatchAccounts,
};

const GROTH16_PUBLIC_INPUTS: usize = 2;

// Applies an off-chain rollup batch to the on-chain ballot state.
//
// The coordinator supplies:
//
// - the new Merkle root
// - the encrypted batch tally
// - the number of votes in the batch
// - a 256-byte Groth16 proof
// - the two public inputs used by the Groth16 circuit
//
// The ballot state is changed only after the Groth16 proof has
// successfully been verified on-chain.
pub fn submit_rollup_batch(
    ctx: Context<SubmitRollupBatchAccounts>,
    new_merkle_root: [u8; 32],
    encrypted_batch_tally: Vec<[u8; 64]>,
    batch_size: u64,
    proof: [u8; 256],
    public_inputs: [[u8; 32]; GROTH16_PUBLIC_INPUTS],
) -> Result<()> {
    // ---------------------------------------------------------
    // Basic batch validation
    // ---------------------------------------------------------

    require!(batch_size > 0, ErrorCode::InvalidBatchSize);

    let proposal_count = ctx.accounts.ballot.proposal_count as usize;

    require!(
        encrypted_batch_tally.len() == proposal_count,
        ErrorCode::InvalidTallySize
    );

    validate_ciphertexts(&encrypted_batch_tally)?;

    // ---------------------------------------------------------
    // Bind public input #2 to the instruction batch_size
    //
    // BatchCircuit public inputs:
    //
    // public_inputs[0] = batch_commitment
    // public_inputs[1] = batch_size
    //
    // Arkworks / groth16-solana use big-endian field bytes here.
    // Since batch_size is a small integer, its Fr representation
    // is simply the integer encoded in the final 8 bytes.
    // ---------------------------------------------------------

    let mut expected_batch_size = [0u8; 32];

    expected_batch_size[24..].copy_from_slice(&batch_size.to_be_bytes());

    require!(
        public_inputs[1] == expected_batch_size,
        ErrorCode::InvalidGroth16PublicInputs
    );

    // ---------------------------------------------------------
    // Split the 256-byte proof into the representation expected
    // by groth16-solana:
    //
    // A = 64 bytes
    // B = 128 bytes
    // C = 64 bytes
    //
    // total = 256 bytes
    // ---------------------------------------------------------

    let proof_a: &[u8; 64] = proof[0..64]
        .try_into()
        .map_err(|_| error!(ErrorCode::InvalidGroth16Proof))?;

    let proof_b: &[u8; 128] = proof[64..192]
        .try_into()
        .map_err(|_| error!(ErrorCode::InvalidGroth16Proof))?;

    let proof_c: &[u8; 64] = proof[192..256]
        .try_into()
        .map_err(|_| error!(ErrorCode::InvalidGroth16Proof))?;

    // ---------------------------------------------------------
    // Groth16 verification
    // ---------------------------------------------------------

    msg!("Verifying Kaonashi Groth16 batch proof");

    let mut verifier = Groth16Verifier::<GROTH16_PUBLIC_INPUTS>::new(
        proof_a,
        proof_b,
        proof_c,
        &public_inputs,
        &VERIFYING_KEY,
    )
    .map_err(|verification_error| {
        msg!(
            "Failed to construct Groth16 verifier: {:?}",
            verification_error
        );

        error!(ErrorCode::InvalidGroth16Proof)
    })?;

    verifier.verify().map_err(|verification_error| {
        msg!(
            "Groth16 batch proof verification failed: {:?}",
            verification_error
        );

        error!(ErrorCode::InvalidGroth16Proof)
    })?;

    msg!("Kaonashi Groth16 batch proof verified");

    // ---------------------------------------------------------
    // IMPORTANT:
    //
    // Nothing above this point changes the ballot.
    //
    // Only a batch with a valid Groth16 proof reaches here.
    // ---------------------------------------------------------

    let ballot = &mut ctx.accounts.ballot;

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

    msg!(
        "Rollup batch accepted. Batch size: {}. Total votes: {}",
        batch_size,
        ballot.total_votes
    );

    Ok(())
}
