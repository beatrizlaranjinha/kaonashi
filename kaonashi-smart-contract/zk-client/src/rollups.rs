use anyhow::Result;
use sha2::{Digest, Sha256};
use solana_zk_sdk::encryption::elgamal::ElGamalPubkey;

use crate::crypto::{encrypt_values, encrypt_vote};

#[derive(Debug, Clone)]
pub struct PreparedRollupBatch {
    pub encrypted_votes: Vec<Vec<[u8; 64]>>,
    pub merkle_root: [u8; 32],
    pub plain_tally: Vec<u64>,
    pub encrypted_tally: Vec<[u8; 64]>,
    pub batch_size: u64,
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn vote_leaf(ciphertexts: &[[u8; 64]]) -> [u8; 32] {
    let bytes = ciphertexts
        .iter()
        .flat_map(|ciphertext| ciphertext.iter().copied())
        .collect::<Vec<u8>>();

    sha256(&bytes)
}

pub fn merkle_root(leaves: Vec<[u8; 32]>) -> [u8; 32] {
    match leaves.len() {
        0 => [0u8; 32],
        1 => leaves[0],
        _ => {
            let next_level = leaves
                .chunks(2)
                .map(|pair| {
                    let left = pair[0];
                    let right = pair.get(1).copied().unwrap_or(left);

                    let mut data = Vec::with_capacity(64);
                    data.extend_from_slice(&left);
                    data.extend_from_slice(&right);
                    sha256(&data)
                })
                .collect::<Vec<[u8; 32]>>();

            merkle_root(next_level)
        }
    }
}

pub fn aggregate_votes(
    plain_votes: &[Vec<u64>],
    proposal_count: usize,
) -> Result<Vec<u64>> {
    if proposal_count == 0 {
        anyhow::bail!("A votação deve ter pelo menos uma proposta");
    }

    if plain_votes.is_empty() {
        anyhow::bail!("O batch deve conter pelo menos um voto");
    }

    if plain_votes
        .iter()
        .any(|vote| vote.len() != proposal_count)
    {
        anyhow::bail!("Todos os votos devem ter o mesmo número de propostas");
    }

    Ok(plain_votes
        .iter()
        .fold(vec![0u64; proposal_count], |acc, vote| {
            acc.iter()
                .zip(vote.iter())
                .map(|(&acc_value, &vote_value)| acc_value + vote_value)
                .collect()
        }))
}

pub fn prepare_rollup_batch(
    plain_votes: &[Vec<u64>],
    proposal_count: usize,
    public_key: &ElGamalPubkey,
) -> Result<PreparedRollupBatch> {
    let encrypted_votes = plain_votes
        .iter()
        .map(|vote| {
            if vote.len() != proposal_count {
                anyhow::bail!("O tamanho do voto não corresponde ao número de propostas");
            }

            encrypt_vote(vote, public_key)
        })
        .collect::<Result<Vec<Vec<[u8; 64]>>>>()?;

    let leaves = encrypted_votes
        .iter()
        .map(|vote| vote_leaf(vote))
        .collect::<Vec<[u8; 32]>>();

    let plain_tally = aggregate_votes(plain_votes, proposal_count)?;
    let encrypted_tally = encrypt_values(&plain_tally, public_key);

    Ok(PreparedRollupBatch {
        encrypted_votes,
        merkle_root: merkle_root(leaves),
        plain_tally,
        encrypted_tally,
        batch_size: plain_votes.len() as u64,
    })
}
