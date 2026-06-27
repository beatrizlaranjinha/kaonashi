use anchor_lang::prelude::*;
use solana_curve25519::ristretto::{add_ristretto, validate_ristretto, PodRistrettoPoint};

use crate::ErrorCode;

// Valida se a chave pública é um ponto Ristretto válido.
pub(crate) fn validate_public_key(public_key: &[u8; 32]) -> Result<()> {
    let public_key_point = PodRistrettoPoint(*public_key);

    require!(
        validate_ristretto(&public_key_point),
        ErrorCode::InvalidPublicKey
    );

    Ok(())
}

// Valida todos os ciphertexts recebidos.
pub(crate) fn validate_ciphertexts(ciphertexts: &[[u8; 64]]) -> Result<()> {
    ciphertexts.iter().try_for_each(validate_ciphertext)
}

// Soma um voto cifrado ao tally cifrado atual.
pub(crate) fn encrypted_tally_after_vote(
    current_tally: &[[u8; 64]],
    encrypted_vote: &[[u8; 64]],
) -> Result<Vec<[u8; 64]>> {
    require!(
        current_tally.len() == encrypted_vote.len(),
        ErrorCode::InvalidTallySize
    );

    current_tally
        .iter()
        .copied()
        .zip(encrypted_vote.iter().copied())
        .map(|(current, vote)| sum_ciphertexts(current, vote))
        .collect()
}

// Soma homomorficamente dois ciphertexts ElGamal.
fn sum_ciphertexts(a: [u8; 64], b: [u8; 64]) -> Result<[u8; 64]> {
    let (a_commitment, a_handle) = ciphertext_to_points(a);
    let (b_commitment, b_handle) = ciphertext_to_points(b);

    let summed_commitment = add_points(a_commitment, b_commitment)?;
    let summed_handle = add_points(a_handle, b_handle)?;

    Ok(points_to_ciphertext(summed_commitment, summed_handle))
}

// Divide um ciphertext nos seus dois pontos: commitment e handle.
fn ciphertext_to_points(ciphertext: [u8; 64]) -> (PodRistrettoPoint, PodRistrettoPoint) {
    let mut commitment = [0u8; 32];
    let mut handle = [0u8; 32];

    commitment.copy_from_slice(&ciphertext[..32]);
    handle.copy_from_slice(&ciphertext[32..]);

    (PodRistrettoPoint(commitment), PodRistrettoPoint(handle))
}

// Converte os dois pontos novamente num ciphertext de 64 bytes.
fn points_to_ciphertext(commitment: PodRistrettoPoint, handle: PodRistrettoPoint) -> [u8; 64] {
    let mut ciphertext = [0u8; 64];

    ciphertext[..32].copy_from_slice(&commitment.0);
    ciphertext[32..].copy_from_slice(&handle.0);

    ciphertext
}

// Soma dois pontos Ristretto.
fn add_points(a: PodRistrettoPoint, b: PodRistrettoPoint) -> Result<PodRistrettoPoint> {
    add_ristretto(&a, &b).ok_or(error!(ErrorCode::InvalidCiphertext))
}

// Verifica se os dois pontos de um ciphertext são válidos.
fn validate_ciphertext(ciphertext: &[u8; 64]) -> Result<()> {
    let (commitment, handle) = ciphertext_to_points(*ciphertext);

    require!(
        validate_ristretto(&commitment),
        ErrorCode::InvalidCiphertext
    );

    require!(validate_ristretto(&handle), ErrorCode::InvalidCiphertext);

    Ok(())
}
