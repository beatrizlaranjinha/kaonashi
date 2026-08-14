use crate::models::PendingEncryptedVote;

use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{prepare_verifying_key, Groth16, ProvingKey};
use ark_relations::{
    lc,
    r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable},
};
use ark_std::test_rng;

use std::time::{Duration, Instant};

/// Output produced by the Groth16 batch prover.
///
/// For now we keep the Arkworks proof internally.
/// Serialization to the 256-byte Solana representation comes next.
pub struct BatchProofResult {
    pub proof: ark_groth16::Proof<Bn254>,
    pub batch_commitment: Fr,
    pub batch_size: Fr,

    pub setup_time: Duration,
    pub proof_generation_time: Duration,
    pub local_verification_time: Duration,
}

/// Minimal first Kaonashi Groth16 batch circuit.
///
/// PRIVATE:
///     one field-element commitment for each encrypted vote
///
/// PUBLIC:
///     batch_commitment
///     batch_size
///
/// PROVES:
///     1. The private commitments sum to batch_commitment.
///     2. The circuit contains exactly batch_size commitments.
///
/// IMPORTANT:
/// This is the first integration circuit.
/// It does NOT yet prove the SHA Merkle root or the ElGamal tally.
#[derive(Clone)]
pub struct BatchCircuit {
    pub vote_commitments: Vec<Option<Fr>>,
    pub batch_commitment: Option<Fr>,
    pub batch_size: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for BatchCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public input #1
        let batch_commitment_var = cs.new_input_variable(|| {
            self.batch_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Public input #2
        let batch_size_var =
            cs.new_input_variable(|| self.batch_size.ok_or(SynthesisError::AssignmentMissing))?;

        // ---------------------------------------------------------
        // Constraint 1:
        //
        // vote_commitment_0 + ... + vote_commitment_n
        //     == batch_commitment
        // ---------------------------------------------------------

        let mut commitment_sum = lc!();

        for commitment in &self.vote_commitments {
            let commitment_var =
                cs.new_witness_variable(|| commitment.ok_or(SynthesisError::AssignmentMissing))?;

            commitment_sum = commitment_sum + commitment_var;
        }

        cs.enforce_constraint(
            commitment_sum,
            lc!() + Variable::One,
            lc!() + batch_commitment_var,
        )?;

        // ---------------------------------------------------------
        // Constraint 2:
        //
        // number of private vote commitments == batch_size
        //
        // The circuit shape determines how many witness slots exist.
        // We constrain the public batch_size to that constant.
        // ---------------------------------------------------------

        let expected_batch_size = Fr::from(self.vote_commitments.len() as u64);

        cs.enforce_constraint(
            lc!() + (expected_batch_size, Variable::One),
            lc!() + Variable::One,
            lc!() + batch_size_var,
        )?;

        Ok(())
    }
}

/// Converts one real Kaonashi encrypted vote into a field element.
///
/// The vote already contains the ElGamal ciphertext bytes:
///
///     Vec<[u8; 64]>
///
/// We flatten those bytes and deterministically map them into Fr.
///
/// This gives us a ZK-friendly scalar representation of the vote
/// for this first circuit.
///
/// IMPORTANT:
/// This is NOT the existing Merkle leaf hash.
fn encrypted_vote_to_field(vote: &PendingEncryptedVote) -> Fr {
    let mut bytes = Vec::new();

    // Bind the commitment to the same metadata that is relevant
    // to the batch.
    bytes.extend_from_slice(vote.wallet_id.as_bytes());
    bytes.extend_from_slice(vote.public_key.as_bytes());
    bytes.push(vote.decade_id);
    bytes.extend_from_slice(vote.encrypted_vote_hash.as_bytes());

    for ciphertext in &vote.encrypted_vote {
        bytes.extend_from_slice(ciphertext);
    }

    Fr::from_le_bytes_mod_order(&bytes)
}

/// Creates the public commitment corresponding to the private
/// vote commitments.
fn calculate_batch_commitment(commitments: &[Fr]) -> Fr {
    commitments
        .iter()
        .copied()
        .fold(Fr::from(0u64), |acc, value| acc + value)
}

/// Creates a setup for a circuit containing exactly `batch_size` votes.
///
/// For the moment this is called during proof generation.
/// Later we will move setup OUT of the hot path and reuse a fixed
/// proving/verifying key for benchmark correctness.
fn create_parameters(batch_size: usize) -> Result<ProvingKey<Bn254>, String> {
    let mut rng = test_rng();

    let circuit = BatchCircuit {
        vote_commitments: vec![None; batch_size],
        batch_commitment: None,
        batch_size: None,
    };

    Groth16::<Bn254>::generate_random_parameters_with_reduction(circuit, &mut rng)
        .map_err(|error| format!("Groth16 setup failed: {error}"))
}

/// Generates and locally verifies a Groth16 proof for a REAL
/// Kaonashi batch.
pub fn generate_batch_proof(votes: &[PendingEncryptedVote]) -> Result<BatchProofResult, String> {
    if votes.is_empty() {
        return Err("Cannot generate Groth16 proof for empty batch".to_string());
    }

    // ---------------------------------------------------------
    // Convert the real encrypted votes into private field values.
    // ---------------------------------------------------------

    let vote_commitments = votes
        .iter()
        .map(encrypted_vote_to_field)
        .collect::<Vec<Fr>>();

    let batch_commitment = calculate_batch_commitment(&vote_commitments);

    let batch_size = Fr::from(votes.len() as u64);

    println!(
        "Generating Groth16 proof for Kaonashi batch of {} votes",
        votes.len()
    );

    // ---------------------------------------------------------
    // SETUP
    // ---------------------------------------------------------

    let setup_start = Instant::now();

    let proving_key = create_parameters(votes.len())?;

    let setup_time = setup_start.elapsed();

    println!("Groth16 setup time: {:?}", setup_time);

    let prepared_verifying_key = prepare_verifying_key(&proving_key.vk);

    // ---------------------------------------------------------
    // PROOF GENERATION
    // ---------------------------------------------------------

    let proof_circuit = BatchCircuit {
        vote_commitments: vote_commitments.iter().copied().map(Some).collect(),

        batch_commitment: Some(batch_commitment),

        batch_size: Some(batch_size),
    };

    let mut rng = test_rng();

    let proof_start = Instant::now();

    let proof =
        Groth16::<Bn254>::create_random_proof_with_reduction(proof_circuit, &proving_key, &mut rng)
            .map_err(|error| format!("Groth16 batch proof generation failed: {error}"))?;

    let proof_generation_time = proof_start.elapsed();

    println!("Groth16 proof generation time: {:?}", proof_generation_time);

    // ---------------------------------------------------------
    // LOCAL VERIFICATION
    // ---------------------------------------------------------

    let public_inputs = [batch_commitment, batch_size];

    let verification_start = Instant::now();

    let valid = Groth16::<Bn254>::verify_proof(&prepared_verifying_key, &proof, &public_inputs)
        .map_err(|error| format!("Groth16 local verification failed: {error}"))?;

    let local_verification_time = verification_start.elapsed();

    println!(
        "Groth16 local verification time: {:?}",
        local_verification_time
    );

    if !valid {
        return Err("Generated Groth16 batch proof is invalid".to_string());
    }

    println!("Kaonashi Groth16 batch proof valid: true");

    Ok(BatchProofResult {
        proof,
        batch_commitment,
        batch_size,
        setup_time,
        proof_generation_time,
        local_verification_time,
    })
}
