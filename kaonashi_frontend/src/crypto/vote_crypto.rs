use curve25519_dalek::scalar::Scalar;
use rand_core::OsRng;
use sha2::{Digest, Sha256};

use solana_zk_sdk::encryption::{elgamal::ElGamalPubkey, pedersen::PedersenOpening};

pub struct EncryptedVoteWitness {
    pub encrypted_vote: Vec<[u8; 64]>,

    // Used only locally to generate the ZK proofs.
    // Never sent to the API.
    pub opening_scalars: Vec<Scalar>,
}

pub fn create_vote_vector(
    selected_index: usize,
    proposal_count: usize,
) -> Result<Vec<u64>, String> {
    if proposal_count == 0 {
        return Err("The ballot must contain at least one proposal".to_string());
    }

    if selected_index >= proposal_count {
        return Err(format!(
            "Invalid proposal index {}. The ballot contains {} proposals",
            selected_index, proposal_count
        ));
    }

    let mut vote = vec![0u64; proposal_count];
    vote[selected_index] = 1;

    Ok(vote)
}

//cifra cada calor do voto com elgamal
// guarda a randomness/opening usada em cada cifra
pub fn encrypt_vote_with_witness(
    vote: &[u64],
    public_key: &ElGamalPubkey,
) -> Result<EncryptedVoteWitness, String> {
    let mut encrypted_vote = Vec::with_capacity(vote.len());
    let mut opening_scalars = Vec::with_capacity(vote.len());

    for value in vote {
        let r = Scalar::random(&mut OsRng);
        let opening = PedersenOpening::new(r);

        let ciphertext = public_key.encrypt_with_u64(*value, &opening).to_bytes();

        encrypted_vote.push(ciphertext);
        opening_scalars.push(r);
    }

    Ok(EncryptedVoteWitness {
        encrypted_vote,  //vai para a api
        opening_scalars, //fica no frontend para gerar as zk proofs
    })
}

pub fn hash_encrypted_vote(encrypted_vote: &[[u8; 64]]) -> String {
    let mut hasher = Sha256::new();

    for ciphertext in encrypted_vote {
        hasher.update(ciphertext);
    }

    hex::encode(hasher.finalize())
}
