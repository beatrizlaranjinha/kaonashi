use anchor_lang::prelude::*;

use groth16_solana::groth16::Groth16Verifier;

use crate::{
    crypto::{encrypted_tally_after_vote, validate_ciphertexts},
    groth16_verifying_key::VERIFYING_KEY as VERIFYING_KEY_10,
    groth16_verifying_key_100::VERIFYING_KEY as VERIFYING_KEY_100,
    groth16_verifying_key_50::VERIFYING_KEY as VERIFYING_KEY_50,
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
// The Groth16 public inputs are now:
//
// public_inputs[0] = high 128 bits of the exact Merkle root
// public_inputs[1] = low 128 bits of the exact Merkle root
//
// The ballot state is changed only after:
//
// 1. the public Groth16 root is confirmed to match `new_merkle_root`; and
// 2. the Groth16 proof is successfully verified on-chain.
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

    require!(
        batch_size == 10 || batch_size == 50 || batch_size == 100,
        ErrorCode::InvalidBatchSize
    );

    // ---------------------------------------------------------
    // Select the verifying key corresponding to this circuit
    // shape.
    //
    // Each batch size uses its own Groth16 setup:
    //
    // 10 votes  -> VK10
    // 50 votes  -> VK50
    // 100 votes -> VK100
    //
    // The circuit shape itself therefore binds the number of
    // private Merkle leaves represented by the proof.
    // ---------------------------------------------------------

    let verifying_key = match batch_size {
        10 => &VERIFYING_KEY_10,
        50 => &VERIFYING_KEY_50,
        100 => &VERIFYING_KEY_100,
        _ => return err!(ErrorCode::InvalidBatchSize),
    };

    // ---------------------------------------------------------
    // Validate encrypted tally
    // ---------------------------------------------------------

    let proposal_count = ctx.accounts.ballot.proposal_count as usize;

    require!(
        encrypted_batch_tally.len() == proposal_count,
        ErrorCode::InvalidTallySize
    );

    validate_ciphertexts(&encrypted_batch_tally)?;

    // ---------------------------------------------------------
    // Bind the Groth16 public inputs to the exact Merkle root
    // supplied in this Solana instruction.
    //
    // BatchCircuit public inputs:
    //
    // public_inputs[0] = first 16 bytes of new_merkle_root
    // public_inputs[1] = last 16 bytes of new_merkle_root
    //
    // Each 128-bit half is represented as a BN254 Fr value and
    // serialized to 32-byte big-endian form.
    //
    // Because each half is only 128 bits, this conversion is
    // lossless: there is no field reduction / ambiguity.
    // ---------------------------------------------------------

    let mut expected_root_hi = [0u8; 32];
    expected_root_hi[16..].copy_from_slice(&new_merkle_root[..16]);

    let mut expected_root_lo = [0u8; 32];
    expected_root_lo[16..].copy_from_slice(&new_merkle_root[16..]);

    require!(
        public_inputs[0] == expected_root_hi,
        ErrorCode::InvalidGroth16PublicInputs
    );

    require!(
        public_inputs[1] == expected_root_lo,
        ErrorCode::InvalidGroth16PublicInputs
    );

    msg!("Groth16 public inputs match submitted Merkle root");

    // ---------------------------------------------------------
    // Split the 256-byte Groth16 proof into the representation
    // expected by groth16-solana:
    //
    // A = 64 bytes
    // B = 128 bytes
    // C = 64 bytes
    //
    // Total = 256 bytes
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

    msg!(
        "Verifying Merkle-bound Kaonashi Groth16 batch proof. Batch size: {}",
        batch_size
    );

    let mut verifier = Groth16Verifier::<GROTH16_PUBLIC_INPUTS>::new(
        proof_a,
        proof_b,
        proof_c,
        &public_inputs,
        verifying_key,
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

    msg!(
        "Merkle-bound Kaonashi Groth16 batch proof verified. Batch size: {}",
        batch_size
    );

    // ---------------------------------------------------------
    // IMPORTANT:
    //
    // Nothing above this point changes the ballot.
    //
    // Reaching this point means:
    //
    // - the proof is valid for the selected circuit shape;
    // - the Merkle root proved inside Groth16 is exactly the same
    //   root supplied as `new_merkle_root`.
    //
    // Only then is the ballot state updated.
    // ---------------------------------------------------------

    let ballot = &mut ctx.accounts.ballot;

    // ---------------------------------------------------------
    // Add encrypted batch tally to existing encrypted tally
    // ---------------------------------------------------------

    ballot.encrypted_tally =
        encrypted_tally_after_vote(&ballot.encrypted_tally, &encrypted_batch_tally)?;

    // ---------------------------------------------------------
    // Store the Merkle root already bound to the Groth16 proof
    // ---------------------------------------------------------

    ballot.merkle_root = new_merkle_root;

    // ---------------------------------------------------------
    // Update number of votes
    // ---------------------------------------------------------

    ballot.total_votes = ballot
        .total_votes
        .checked_add(batch_size)
        .ok_or(error!(ErrorCode::MathOverflow))?;

    // ---------------------------------------------------------
    // Update number of rollup batches
    // ---------------------------------------------------------

    ballot.batch_count = ballot
        .batch_count
        .checked_add(1)
        .ok_or(error!(ErrorCode::MathOverflow))?;

    msg!(
        "Rollup batch accepted. Batch size: {}. Total votes: {}. Batch count: {}",
        batch_size,
        ballot.total_votes,
        ballot.batch_count
    );

    Ok(())
}
