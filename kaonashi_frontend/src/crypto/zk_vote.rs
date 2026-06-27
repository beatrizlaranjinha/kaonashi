use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use rand_core::OsRng;
use sha2::{Digest, Sha512};

use solana_zk_sdk::encryption::{elgamal::ElGamalPubkey, pedersen::PedersenOpening};

use crate::api::client::{RistrettoVoteProof, RistrettoVoteSumProof};

// Helpers
//converter um ponto ristreto para bytes
fn vec_from_point(point: &RistrettoPoint) -> Vec<u8> {
    point.compress().to_bytes().to_vec()
}
//converter um scalar para bytes (usado para enviar a proof por json para a api)
fn vec_from_scalar(scalar: &Scalar) -> Vec<u8> {
    scalar.to_bytes().to_vec()
}

//divide o ciphertext solana em duas partes -> commitment e handle (as provas trabalham sobre estes dois pontos )
fn split_ciphertext(ciphertext: &[u8; 64]) -> Result<(RistrettoPoint, RistrettoPoint), String> {
    let mut commitment_bytes = [0u8; 32];
    let mut handle_bytes = [0u8; 32];

    commitment_bytes.copy_from_slice(&ciphertext[0..32]);
    handle_bytes.copy_from_slice(&ciphertext[32..64]);

    let commitment = CompressedRistretto(commitment_bytes)
        .decompress()
        .ok_or_else(|| "Invalid ciphertext commitment point".to_string())?;

    let handle = CompressedRistretto(handle_bytes)
        .decompress()
        .ok_or_else(|| "Invalid ciphertext handle point".to_string())?;

    Ok((commitment, handle))
}

// Deriva os pontos usados pela cifragem Solana:
//
// G = commitment de Enc(1; 0) permite obter G
// H = commitment de Enc(0; 1) premite obter H e P
// P = handle de Enc(0; 1), ou seja, a ElGamal public key como ponto
fn derive_bases(
    public_key: &ElGamalPubkey,
) -> Result<(RistrettoPoint, RistrettoPoint, RistrettoPoint), String> {
    let zero_opening = PedersenOpening::new(Scalar::ZERO);
    let one_opening = PedersenOpening::new(Scalar::ONE);

    let enc_one_zero = public_key.encrypt_with_u64(1, &zero_opening).to_bytes();

    let enc_zero_one = public_key.encrypt_with_u64(0, &one_opening).to_bytes();

    let (g_base, _) = split_ciphertext(&enc_one_zero)?;
    let (h_base, public_key_point) = split_ciphertext(&enc_zero_one)?;

    Ok((g_base, h_base, public_key_point))
}

//calcula o challange fiat-shamir da voteProof
fn challenge_vote_proof(
    public_key: &ElGamalPubkey,
    ciphertext: &[u8; 64],
    a0: &RistrettoPoint,
    b0: &RistrettoPoint,
    a1: &RistrettoPoint,
    b1: &RistrettoPoint,
) -> Scalar {
    let mut hasher = Sha512::new();

    hasher.update(b"kaonashi-vote-proof");
    hasher.update(public_key.to_bytes());
    hasher.update(ciphertext);
    hasher.update(a0.compress().as_bytes());
    hasher.update(b0.compress().as_bytes());
    hasher.update(a1.compress().as_bytes());
    hasher.update(b1.compress().as_bytes());

    let hash = hasher.finalize();

    let mut wide = [0u8; 64];
    wide.copy_from_slice(&hash);

    Scalar::from_bytes_mod_order_wide(&wide)
}
//calcula o challange fiat shamir da voteSumProof
fn challenge_sum_proof(
    public_key: &ElGamalPubkey,
    aggregate_commitment: &RistrettoPoint,
    aggregate_handle: &RistrettoPoint,
    a: &RistrettoPoint,
    b: &RistrettoPoint,
) -> Scalar {
    let mut hasher = Sha512::new();

    hasher.update(b"kaonashi-vote-sum-proof");
    hasher.update(public_key.to_bytes());
    hasher.update(aggregate_commitment.compress().as_bytes());
    hasher.update(aggregate_handle.compress().as_bytes());
    hasher.update(a.compress().as_bytes());
    hasher.update(b.compress().as_bytes());

    let hash = hasher.finalize();

    let mut wide = [0u8; 64];
    wide.copy_from_slice(&hash);

    Scalar::from_bytes_mod_order_wide(&wide)
}

// VoteProof: ciphertext encrypts 0 OR 1, prova sem revelar!!

pub fn generate_vote_proof(
    public_key: &ElGamalPubkey,
    ciphertext: &[u8; 64],
    vote_value: u64,
    opening: &Scalar,
) -> Result<RistrettoVoteProof, String> {
    if vote_value != 0 && vote_value != 1 {
        return Err("VoteProof can only be generated for values 0 or 1".to_string());
    }

    let (g_base, h_base, public_key_point) = derive_bases(public_key)?;
    let (commitment, handle) = split_ciphertext(ciphertext)?;

    let commitment_minus_0 = commitment;
    let commitment_minus_1 = commitment - g_base;

    let w = Scalar::random(&mut OsRng);
    let simulated_c = Scalar::random(&mut OsRng);
    let simulated_s = Scalar::random(&mut OsRng);

    if vote_value == 0 {
        // Real branch: ciphertext encrypts 0
        let a0 = h_base * w;
        let b0 = public_key_point * w;

        // Simulated branch: ciphertext encrypts 1
        let c1 = simulated_c;
        let s1 = simulated_s;

        let a1 = h_base * s1 - commitment_minus_1 * c1;
        let b1 = public_key_point * s1 - handle * c1;

        let challenge = challenge_vote_proof(public_key, ciphertext, &a0, &b0, &a1, &b1);

        let c0 = challenge - c1;
        let s0 = w + c0 * opening;

        Ok(RistrettoVoteProof {
            a0: vec_from_point(&a0),
            b0: vec_from_point(&b0),
            c0: vec_from_scalar(&c0),
            s0: vec_from_scalar(&s0),

            a1: vec_from_point(&a1),
            b1: vec_from_point(&b1),
            c1: vec_from_scalar(&c1),
            s1: vec_from_scalar(&s1),
        })
    } else {
        // Simulated branch: ciphertext encrypts 0
        let c0 = simulated_c;
        let s0 = simulated_s;

        let a0 = h_base * s0 - commitment_minus_0 * c0;
        let b0 = public_key_point * s0 - handle * c0;

        // Real branch: ciphertext encrypts 1
        let a1 = h_base * w;
        let b1 = public_key_point * w;

        let challenge = challenge_vote_proof(public_key, ciphertext, &a0, &b0, &a1, &b1);

        let c1 = challenge - c0;
        let s1 = w + c1 * opening;

        Ok(RistrettoVoteProof {
            a0: vec_from_point(&a0),
            b0: vec_from_point(&b0),
            c0: vec_from_scalar(&c0),
            s0: vec_from_scalar(&s0),

            a1: vec_from_point(&a1),
            b1: vec_from_point(&b1),
            c1: vec_from_scalar(&c1),
            s1: vec_from_scalar(&s1),
        })
    }
}
//gera uma voteproof para cada posição do vetor, chama a vote_Proof
pub fn generate_vote_proofs(
    public_key: &ElGamalPubkey,
    vote_vector: &[u64],
    encrypted_vote: &[[u8; 64]],
    openings: &[Scalar],
) -> Result<Vec<RistrettoVoteProof>, String> {
    if vote_vector.len() != encrypted_vote.len() {
        return Err("Vote vector and encrypted vote have different lengths".to_string());
    }

    if encrypted_vote.len() != openings.len() {
        return Err("Encrypted vote and openings have different lengths".to_string());
    }

    vote_vector
        .iter()
        .zip(encrypted_vote.iter())
        .zip(openings.iter())
        .map(|((value, ciphertext), opening)| {
            generate_vote_proof(public_key, ciphertext, *value, opening)
        })
        .collect()
}

// VoteSumProof: encrypted vector sums to 1

pub fn generate_vote_sum_proof(
    public_key: &ElGamalPubkey,
    encrypted_vote: &[[u8; 64]],
    openings: &[Scalar],
) -> Result<RistrettoVoteSumProof, String> {
    if encrypted_vote.is_empty() {
        return Err("Cannot generate VoteSumProof for an empty vote".to_string());
    }

    if encrypted_vote.len() != openings.len() {
        return Err("Encrypted vote and openings have different lengths".to_string());
    }

    let (g_base, h_base, public_key_point) = derive_bases(public_key)?;

    let mut aggregate_commitment: Option<RistrettoPoint> = None;
    let mut aggregate_handle: Option<RistrettoPoint> = None;

    for ciphertext in encrypted_vote {
        let (commitment, handle) = split_ciphertext(ciphertext)?;

        aggregate_commitment = Some(match aggregate_commitment {
            Some(current) => current + commitment,
            None => commitment,
        });

        aggregate_handle = Some(match aggregate_handle {
            Some(current) => current + handle,
            None => handle,
        });
    }

    let aggregate_commitment =
        aggregate_commitment.ok_or_else(|| "Missing aggregate commitment".to_string())?;

    let aggregate_handle =
        aggregate_handle.ok_or_else(|| "Missing aggregate handle".to_string())?;

    let total_opening = openings
        .iter()
        .fold(Scalar::ZERO, |acc, opening| acc + opening);

    // If the encrypted vector sums to 1:
    // aggregate_commitment - G = total_opening * H
    let commitment_minus_one = aggregate_commitment - g_base;

    let w = Scalar::random(&mut OsRng);

    let a = h_base * w;
    let b = public_key_point * w;

    let c = challenge_sum_proof(public_key, &aggregate_commitment, &aggregate_handle, &a, &b);

    let s = w + c * total_opening;

    // The verifier will check:
    // sH = a + c(aggregate_commitment - G)
    // sP = b + c(aggregate_handle)
    let _check_a = h_base * s - commitment_minus_one * c;
    let _check_b = public_key_point * s - aggregate_handle * c;

    Ok(RistrettoVoteSumProof {
        a: vec_from_point(&a),
        b: vec_from_point(&b),
        c: vec_from_scalar(&c),
        s: vec_from_scalar(&s),
    })
}
