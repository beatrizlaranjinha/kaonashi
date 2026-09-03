use crate::merkle::merkle_root;
use crate::zk_merkle::merkle_root_from_leaf_hashes;

use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G2Affine};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{prepare_verifying_key, Groth16, ProvingKey};
use ark_r1cs_std::{
    boolean::Boolean,
    fields::fp::FpVar,
    prelude::{AllocVar, EqGadget, ToBitsGadget},
    uint8::UInt8,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::CanonicalDeserialize;
use ark_std::test_rng;

use solana_sdk::hash::Hash;

use std::{
    fs,
    str::FromStr,
    time::{Duration, Instant},
};

// ============================================================================
// Fixed Groth16 configuration
// ============================================================================

pub const SUPPORTED_GROTH16_BATCH_SIZES: &[usize] = &[10, 50, 100];

fn proving_key_path(batch_size: usize) -> Result<String, String> {
    match batch_size {
        10 | 50 | 100 => Ok(format!("data/groth16/batch{}_proving_key.bin", batch_size)),
        _ => Err(format!(
            "Unsupported Groth16 batch size: {}. Supported sizes: {:?}",
            batch_size, SUPPORTED_GROTH16_BATCH_SIZES
        )),
    }
}

// ============================================================================
// Proof result
// ============================================================================

/// Result produced by the Groth16 batch prover.
///
/// PUBLIC INPUTS:
///
/// [0] = high 128 bits of the exact 32-byte Merkle root
/// [1] = low 128 bits of the exact 32-byte Merkle root
///
/// Batch size is not a public input anymore. The circuit shape is fixed for
/// each setup (10 / 50 / 100 leaves), and the Solana program selects the
/// corresponding verifying key from the submitted batch size.
pub struct BatchProofResult {
    pub proof: ark_groth16::Proof<Bn254>,

    /// Serialized Groth16 proof:
    ///
    /// -A = 64 bytes
    ///  B = 128 bytes
    ///  C = 64 bytes
    ///
    /// Total = 256 bytes.
    pub proof_bytes: [u8; 256],

    pub merkle_root_hi: Fr,
    pub merkle_root_lo: Fr,

    /// Public inputs serialized in big-endian representation.
    ///
    /// [0] = Merkle root bytes 0..16, represented as an Fr
    /// [1] = Merkle root bytes 16..32, represented as an Fr
    pub public_inputs_bytes: [[u8; 32]; 2],

    /// Time required to load and deserialize the proving key.
    ///
    /// This is NOT Groth16 setup time.
    pub proving_key_load_time: Duration,

    pub proof_generation_time: Duration,
    pub local_verification_time: Duration,
}

// ============================================================================
// Batch circuit
// ============================================================================

/// Kaonashi Groth16 Merkle-bound batch circuit.
///
/// PRIVATE INPUTS:
///     exactly N private 32-byte Merkle leaf hashes
///
/// PUBLIC INPUTS:
///     merkle_root_hi
///     merkle_root_lo
///
/// PROVES:
///
///     SHA256 MerkleTree(private_leaf_0, ..., private_leaf_N)
///         == public Merkle root
///
/// The internal Merkle hashing rule is exactly the same as src/merkle.rs:
///
///     parent = SHA256("kaonashi-node" || left[32] || right[32])
///
/// and an odd final node is duplicated.
///
/// IMPORTANT:
///
/// This circuit binds the Groth16 proof to the exact Merkle root submitted
/// for the batch. It does NOT yet prove the homomorphic ElGamal batch tally.
#[derive(Clone)]
pub struct BatchCircuit {
    pub leaf_hashes: Vec<Option<[u8; 32]>>,
    pub merkle_root_hi: Option<Fr>,
    pub merkle_root_lo: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for BatchCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        if self.leaf_hashes.is_empty() {
            return Err(SynthesisError::Unsatisfiable);
        }

        // ---------------------------------------------------------
        // Compute the Merkle root from the private leaf hashes.
        // ---------------------------------------------------------

        let computed_root = merkle_root_from_leaf_hashes(cs.clone(), &self.leaf_hashes)?;

        // ---------------------------------------------------------
        // Public input #1: high 128 bits of the Merkle root.
        // ---------------------------------------------------------

        let merkle_root_hi_var = FpVar::<Fr>::new_input(cs.clone(), || {
            self.merkle_root_hi.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ---------------------------------------------------------
        // Public input #2: low 128 bits of the Merkle root.
        // ---------------------------------------------------------

        let merkle_root_lo_var = FpVar::<Fr>::new_input(cs, || {
            self.merkle_root_lo.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Convert each 16-byte big-endian half of the in-circuit SHA-256
        // digest into an Fr without loss. 128 bits fit safely inside BN254 Fr.
        let computed_hi = bytes_be_to_fp_var(&computed_root.0[..16])?;
        let computed_lo = bytes_be_to_fp_var(&computed_root.0[16..32])?;

        computed_hi.enforce_equal(&merkle_root_hi_var)?;
        computed_lo.enforce_equal(&merkle_root_lo_var)?;

        Ok(())
    }
}

/// Convert a big-endian byte slice into an Fr variable.
///
/// This helper is currently used with exactly 16 bytes (128 bits), so the
/// conversion is injective and does not perform field reduction.
fn bytes_be_to_fp_var(bytes: &[UInt8<Fr>]) -> Result<FpVar<Fr>, SynthesisError> {
    let mut bits_le = Vec::with_capacity(bytes.len() * 8);

    // `Boolean::le_bits_to_fp` expects the least-significant bit first.
    // The digest bytes are big-endian, so visit the bytes in reverse order,
    // while preserving little-endian bit order inside each byte.
    for byte in bytes.iter().rev() {
        bits_le.extend(byte.to_bits_le()?);
    }

    Boolean::le_bits_to_fp(&bits_le)
}

// ============================================================================
// Merkle root encoding helpers
// ============================================================================

/// Split an exact 32-byte Merkle root into two lossless 128-bit BN254 scalar
/// public inputs.
pub fn merkle_root_to_public_inputs(root: [u8; 32]) -> [Fr; 2] {
    [
        Fr::from_be_bytes_mod_order(&root[..16]),
        Fr::from_be_bytes_mod_order(&root[16..32]),
    ]
}

fn parse_hash_bytes(value: &str) -> Result<[u8; 32], String> {
    Hash::from_str(value)
        .map(|hash| hash.to_bytes())
        .map_err(|error| format!("Invalid Merkle hash '{}': {}", value, error))
}

// ============================================================================
// Serialization helpers
// ============================================================================

/// Convert a BN254 base-field value into 32-byte big endian.
fn fq_to_be(value: &Fq) -> [u8; 32] {
    let bytes = value.into_bigint().to_bytes_be();

    let mut out = [0u8; 32];

    out[32 - bytes.len()..].copy_from_slice(&bytes);

    out
}

/// Convert a BN254 scalar-field value into 32-byte big endian.
///
/// groth16-solana expects public inputs in this representation.
fn fr_to_be(value: &Fr) -> [u8; 32] {
    let bytes = value.into_bigint().to_bytes_be();

    let mut out = [0u8; 32];

    out[32 - bytes.len()..].copy_from_slice(&bytes);

    out
}

/// Serialize a G1 point as x || y (64 bytes).
fn g1_to_bytes(point: &G1Affine) -> [u8; 64] {
    let mut out = [0u8; 64];

    out[..32].copy_from_slice(&fq_to_be(&point.x));
    out[32..].copy_from_slice(&fq_to_be(&point.y));

    out
}

/// Serialize Fq2 using the ordering expected by groth16-solana: c1 || c0.
fn fq2_to_bytes(value: &Fq2) -> [u8; 64] {
    let mut out = [0u8; 64];

    out[..32].copy_from_slice(&fq_to_be(&value.c1));
    out[32..].copy_from_slice(&fq_to_be(&value.c0));

    out
}

/// Serialize a G2 point (128 bytes).
fn g2_to_bytes(point: &G2Affine) -> [u8; 128] {
    let mut out = [0u8; 128];

    out[..64].copy_from_slice(&fq2_to_bytes(&point.x));
    out[64..].copy_from_slice(&fq2_to_bytes(&point.y));

    out
}

/// Convert an Arkworks Groth16 proof into the exact representation expected by
/// groth16-solana.
///
/// groth16-solana's pairing equation expects -A.
fn serialize_proof(proof: &ark_groth16::Proof<Bn254>) -> [u8; 256] {
    let proof_a_neg = -proof.a;

    let proof_a = g1_to_bytes(&proof_a_neg);
    let proof_b = g2_to_bytes(&proof.b);
    let proof_c = g1_to_bytes(&proof.c);

    let mut out = [0u8; 256];

    out[0..64].copy_from_slice(&proof_a);
    out[64..192].copy_from_slice(&proof_b);
    out[192..256].copy_from_slice(&proof_c);

    out
}

// ============================================================================
// Fixed proving key
// ============================================================================

/// Loads the proving key generated once for the corresponding fixed circuit
/// shape. This function does NOT perform trusted setup.
fn load_proving_key(batch_size: usize) -> Result<ProvingKey<Bn254>, String> {
    let path = proving_key_path(batch_size)?;

    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "Failed to read Groth16 proving key from {}: {}",
            path, error
        )
    })?;

    ProvingKey::<Bn254>::deserialize_compressed(bytes.as_slice()).map_err(|error| {
        format!(
            "Failed to deserialize Groth16 proving key for batch size {}: {}",
            batch_size, error
        )
    })
}

// ============================================================================
// Batch proof generation
// ============================================================================

/// Generate and locally verify a Groth16 proof for the exact Merkle leaves and
/// root already constructed by the Kaonashi batch coordinator.
pub fn generate_batch_proof(
    leaves: &[String],
    expected_merkle_root: &str,
) -> Result<BatchProofResult, String> {
    // ---------------------------------------------------------
    // Basic validation
    // ---------------------------------------------------------

    if leaves.is_empty() {
        return Err("Cannot generate Groth16 proof for empty batch".to_string());
    }

    if !SUPPORTED_GROTH16_BATCH_SIZES.contains(&leaves.len()) {
        return Err(format!(
            "Unsupported Groth16 batch size: {}. Supported sizes: {:?}",
            leaves.len(),
            SUPPORTED_GROTH16_BATCH_SIZES
        ));
    }

    // Make sure the root supplied by batches.rs is exactly the root produced by
    // these same leaves before we spend time generating a proof.
    let computed_merkle_root = merkle_root(leaves)?;

    if computed_merkle_root != expected_merkle_root {
        return Err(format!(
            "Merkle root mismatch before Groth16 proving: computed {}, expected {}",
            computed_merkle_root, expected_merkle_root
        ));
    }

    println!(
        "Generating Merkle-bound Groth16 proof for Kaonashi batch of {} votes",
        leaves.len()
    );

    // ---------------------------------------------------------
    // Build private Merkle leaf witnesses.
    // ---------------------------------------------------------

    let leaf_hashes = leaves
        .iter()
        .map(|leaf| parse_hash_bytes(leaf))
        .collect::<Result<Vec<[u8; 32]>, String>>()?;

    // ---------------------------------------------------------
    // Build the two exact public Merkle-root inputs.
    // ---------------------------------------------------------

    let merkle_root_bytes = parse_hash_bytes(expected_merkle_root)?;
    let [merkle_root_hi, merkle_root_lo] = merkle_root_to_public_inputs(merkle_root_bytes);

    // ---------------------------------------------------------
    // Load fixed proving key for this batch size.
    // ---------------------------------------------------------

    let key_load_start = Instant::now();

    let proving_key = load_proving_key(leaves.len())?;

    let proving_key_load_time = key_load_start.elapsed();

    println!("Groth16 proving key load time: {:?}", proving_key_load_time);

    let prepared_verifying_key = prepare_verifying_key(&proving_key.vk);

    // ---------------------------------------------------------
    // Build circuit witness.
    // ---------------------------------------------------------

    let proof_circuit = BatchCircuit {
        leaf_hashes: leaf_hashes.iter().copied().map(Some).collect(),
        merkle_root_hi: Some(merkle_root_hi),
        merkle_root_lo: Some(merkle_root_lo),
    };

    // ---------------------------------------------------------
    // Generate proof.
    // ---------------------------------------------------------

    let mut rng = test_rng();

    let proof_start = Instant::now();

    let proof =
        Groth16::<Bn254>::create_random_proof_with_reduction(proof_circuit, &proving_key, &mut rng)
            .map_err(|error| format!("Groth16 batch proof generation failed: {}", error))?;

    let proof_generation_time = proof_start.elapsed();

    println!("Groth16 proof generation time: {:?}", proof_generation_time);

    // ---------------------------------------------------------
    // Local verification.
    // ---------------------------------------------------------

    let public_inputs = [merkle_root_hi, merkle_root_lo];

    let verification_start = Instant::now();

    let valid = Groth16::<Bn254>::verify_proof(&prepared_verifying_key, &proof, &public_inputs)
        .map_err(|error| format!("Groth16 local verification failed: {}", error))?;

    let local_verification_time = verification_start.elapsed();

    println!(
        "Groth16 local verification time: {:?}",
        local_verification_time
    );

    if !valid {
        return Err("Generated Merkle-bound Groth16 batch proof is invalid".to_string());
    }

    println!("Kaonashi Merkle-bound Groth16 batch proof valid: true");

    // ---------------------------------------------------------
    // Serialize for Solana.
    // ---------------------------------------------------------

    let proof_bytes = serialize_proof(&proof);

    let public_inputs_bytes = [fr_to_be(&merkle_root_hi), fr_to_be(&merkle_root_lo)];

    println!("Groth16 serialized proof size: {} bytes", proof_bytes.len());

    println!(
        "Groth16 serialized public inputs: {} bytes",
        public_inputs_bytes.len() * 32
    );

    // ---------------------------------------------------------
    // Result.
    // ---------------------------------------------------------

    Ok(BatchProofResult {
        proof,
        proof_bytes,
        merkle_root_hi,
        merkle_root_lo,
        public_inputs_bytes,
        proving_key_load_time,
        proof_generation_time,
        local_verification_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::merkle::hash_leaf;

    use ark_relations::r1cs::ConstraintSystem;

    fn build_test_circuit(leaves: &[String], root: &str) -> BatchCircuit {
        let leaf_hashes = leaves
            .iter()
            .map(|leaf| Some(parse_hash_bytes(leaf).expect("leaf hash")))
            .collect::<Vec<_>>();

        let root_bytes = parse_hash_bytes(root).expect("root hash");
        let [root_hi, root_lo] = merkle_root_to_public_inputs(root_bytes);

        BatchCircuit {
            leaf_hashes,
            merkle_root_hi: Some(root_hi),
            merkle_root_lo: Some(root_lo),
        }
    }

    #[test]
    fn batch_circuit_accepts_correct_merkle_root() {
        let leaves = vec![
            hash_leaf(b"vote-0"),
            hash_leaf(b"vote-1"),
            hash_leaf(b"vote-2"),
            hash_leaf(b"vote-3"),
        ];

        let root = merkle_root(&leaves).expect("root");
        let circuit = build_test_circuit(&leaves, &root);

        let cs = ConstraintSystem::<Fr>::new_ref();

        circuit
            .generate_constraints(cs.clone())
            .expect("constraints");

        assert!(cs.is_satisfied().expect("constraint-system status"));
    }

    #[test]
    fn batch_circuit_rejects_wrong_merkle_root() {
        let leaves = vec![
            hash_leaf(b"vote-0"),
            hash_leaf(b"vote-1"),
            hash_leaf(b"vote-2"),
            hash_leaf(b"vote-3"),
        ];

        let correct_root = merkle_root(&leaves).expect("root");
        let mut wrong_root_bytes = parse_hash_bytes(&correct_root).expect("root bytes");
        wrong_root_bytes[0] ^= 0x01;

        let [wrong_hi, wrong_lo] = merkle_root_to_public_inputs(wrong_root_bytes);

        let leaf_hashes = leaves
            .iter()
            .map(|leaf| Some(parse_hash_bytes(leaf).expect("leaf hash")))
            .collect::<Vec<_>>();

        let circuit = BatchCircuit {
            leaf_hashes,
            merkle_root_hi: Some(wrong_hi),
            merkle_root_lo: Some(wrong_lo),
        };

        let cs = ConstraintSystem::<Fr>::new_ref();

        circuit
            .generate_constraints(cs.clone())
            .expect("constraints");

        assert!(!cs.is_satisfied().expect("constraint-system status"));
    }
}
