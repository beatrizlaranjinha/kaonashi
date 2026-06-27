use anyhow::{bail, Result};
use solana_zk_sdk::encryption::elgamal::{
    ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey, ElGamalSecretKey,
};

// ---------------------------------------------------
// ElGamal key generation
// ---------------------------------------------------

pub fn generate_elgamal_keypair() -> ElGamalKeypair {
    ElGamalKeypair::new_rand()
}

// ---------------------------------------------------
// Vote vector creation
// ---------------------------------------------------

pub fn create_vote_vector(selected_index: usize, proposal_count: usize) -> Result<Vec<u64>> {
    if proposal_count == 0 {
        bail!("The ballot must contain at least one proposal");
    }

    if selected_index >= proposal_count {
        bail!(
            "Invalid proposal index {}. The ballot contains {} proposals",
            selected_index,
            proposal_count
        );
    }

    let mut vote = vec![0u64; proposal_count];
    vote[selected_index] = 1;

    Ok(vote)
}

// ---------------------------------------------------
// Vote vector validation
// ---------------------------------------------------

pub fn validate_vote_vector(vote: &[u64]) -> Result<()> {
    if vote.is_empty() {
        bail!("The vote vector cannot be empty");
    }

    if vote.iter().any(|value| *value != 0 && *value != 1) {
        bail!("VoteProof failed: each value must be 0 or 1");
    }

    if vote.iter().sum::<u64>() != 1 {
        bail!("VoteSumProof failed: the vector must contain exactly one vote");
    }

    Ok(())
}

// ---------------------------------------------------
// Encrypt one complete vote vector
// ---------------------------------------------------

pub fn encrypt_vote(vote: &[u64], public_key: &ElGamalPubkey) -> Result<Vec<[u8; 64]>> {
    validate_vote_vector(vote)?;

    let encrypted_vote = vote
        .iter()
        .map(|value| public_key.encrypt(*value).to_bytes())
        .collect::<Vec<[u8; 64]>>();

    Ok(encrypted_vote)
}

// ---------------------------------------------------
// Encrypt a collection of values
//
// Used for:
// - initial encrypted tally;
// - aggregated rollup tally.
// ---------------------------------------------------

pub fn encrypt_values(values: &[u64], public_key: &ElGamalPubkey) -> Vec<[u8; 64]> {
    values
        .iter()
        .map(|value| public_key.encrypt(*value).to_bytes())
        .collect()
}

// ---------------------------------------------------
// Decrypt one ciphertext
// ---------------------------------------------------

pub fn decrypt_value(ciphertext_bytes: &[u8; 64], secret_key: &ElGamalSecretKey) -> Result<u32> {
    let ciphertext = ElGamalCiphertext::from_bytes(ciphertext_bytes)
        .ok_or_else(|| anyhow::anyhow!("Invalid ElGamal ciphertext"))?;

    let decrypted_value = ciphertext
        .decrypt_u32(secret_key)
        .ok_or_else(|| anyhow::anyhow!("Could not decrypt ElGamal ciphertext"))?;

    let value = u32::try_from(decrypted_value)
        .map_err(|_| anyhow::anyhow!("Decrypted value does not fit in u32"))?;

    Ok(value)
}

// ---------------------------------------------------
// Decrypt the complete tally
// ---------------------------------------------------

pub fn decrypt_tally(
    encrypted_tally: &[[u8; 64]],
    secret_key: &ElGamalSecretKey,
) -> Result<Vec<u32>> {
    if encrypted_tally.is_empty() {
        bail!("The encrypted tally cannot be empty");
    }

    encrypted_tally
        .iter()
        .map(|ciphertext| decrypt_value(ciphertext, secret_key))
        .collect()
}

// ---------------------------------------------------
// Determine the winning proposal
// ---------------------------------------------------

pub fn winner_index(tally: &[u32]) -> Result<usize> {
    if tally.is_empty() {
        bail!("Cannot determine a winner from an empty tally");
    }

    tally
        .iter()
        .enumerate()
        .max_by_key(|(_, votes)| *votes)
        .map(|(index, _)| index)
        .ok_or_else(|| anyhow::anyhow!("Could not determine the winner"))
}
