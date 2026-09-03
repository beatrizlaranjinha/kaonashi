use std::{fs, str::FromStr, time::Duration};

use ark_bn254::{Bn254, Fr};
use ark_groth16::{prepare_verifying_key, Groth16, ProvingKey};
use ark_serialize::CanonicalDeserialize;
use ark_std::test_rng;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use solana_sdk::hash::Hash;

use kaonashi_api::{
    groth16::{merkle_root_to_public_inputs, BatchCircuit},
    merkle::{hash_leaf, merkle_proof, merkle_root, verify_merkle_proof},
};

const BATCH_SIZES: [usize; 3] = [10, 50, 100];

fn proving_key_path(batch_size: usize) -> String {
    format!("data/groth16/batch{}_proving_key.bin", batch_size)
}

fn load_proving_key(batch_size: usize) -> ProvingKey<Bn254> {
    let path = proving_key_path(batch_size);

    let bytes =
        fs::read(&path).unwrap_or_else(|error| panic!("Failed to read {}: {}", path, error));

    ProvingKey::<Bn254>::deserialize_compressed(bytes.as_slice()).unwrap_or_else(|error| {
        panic!(
            "Failed to deserialize proving key for batch {}: {}",
            batch_size, error
        )
    })
}

fn make_leaves(batch_size: usize) -> Vec<String> {
    (0..batch_size)
        .map(|index| {
            let payload = format!("kaonashi-benchmark-vote-{batch_size}-{index}");
            hash_leaf(payload.as_bytes())
        })
        .collect()
}

fn hash_bytes(value: &str) -> [u8; 32] {
    Hash::from_str(value)
        .unwrap_or_else(|error| panic!("Invalid Merkle hash {}: {}", value, error))
        .to_bytes()
}

// ============================================================================
// Merkle benchmarks
// ============================================================================

fn bench_merkle(c: &mut Criterion) {
    for batch_size in BATCH_SIZES {
        let leaves = make_leaves(batch_size);
        let root = merkle_root(&leaves).expect("Merkle root");
        let proof_index = batch_size / 2;
        let proof = merkle_proof(&leaves, proof_index).expect("Merkle proof");
        let leaf = leaves[proof_index].clone();

        let mut group = c.benchmark_group(format!("merkle/batch_{batch_size}"));

        group.bench_function("tree_build", |b| {
            b.iter(|| {
                let result = merkle_root(black_box(&leaves)).expect("Merkle root");
                black_box(result);
            });
        });

        group.bench_function("proof_generation", |b| {
            b.iter(|| {
                let result =
                    merkle_proof(black_box(&leaves), black_box(proof_index)).expect("Merkle proof");
                black_box(result);
            });
        });

        group.bench_function("proof_verification", |b| {
            b.iter(|| {
                let valid =
                    verify_merkle_proof(black_box(&leaf), black_box(&proof), black_box(&root));

                assert!(valid);
                black_box(valid);
            });
        });

        group.finish();
    }
}

// ============================================================================
// Groth16 benchmarks
// ============================================================================
//
// IMPORTANT:
//
// The proving key is loaded ONCE before Criterion measures a batch size.
// Therefore "proof_generation" measures the prover itself and does not include
// reading/deserializing the proving key.
//
// Trusted setup is also NOT part of these benchmarks.
// ============================================================================

fn bench_groth16(c: &mut Criterion) {
    for batch_size in BATCH_SIZES {
        println!(
            "\nLoading Groth16 proving key for batch {} OUTSIDE Criterion measurement...",
            batch_size
        );

        let proving_key = load_proving_key(batch_size);
        let prepared_verifying_key = prepare_verifying_key(&proving_key.vk);

        let leaves = make_leaves(batch_size);
        let root = merkle_root(&leaves).expect("Merkle root");

        let leaf_hashes = leaves
            .iter()
            .map(|leaf| hash_bytes(leaf))
            .collect::<Vec<[u8; 32]>>();

        let root_bytes = hash_bytes(&root);
        let [root_hi, root_lo] = merkle_root_to_public_inputs(root_bytes);
        let public_inputs: [Fr; 2] = [root_hi, root_lo];

        // Generate one valid proof outside the verification measurement.
        let verification_circuit = BatchCircuit {
            leaf_hashes: leaf_hashes.iter().copied().map(Some).collect(),
            merkle_root_hi: Some(root_hi),
            merkle_root_lo: Some(root_lo),
        };

        let mut setup_rng = test_rng();

        let verification_proof = Groth16::<Bn254>::create_random_proof_with_reduction(
            verification_circuit,
            &proving_key,
            &mut setup_rng,
        )
        .expect("Pre-generate Groth16 proof for verification benchmark");

        let valid = Groth16::<Bn254>::verify_proof(
            &prepared_verifying_key,
            &verification_proof,
            &public_inputs,
        )
        .expect("Pre-check Groth16 proof");

        assert!(valid);

        let mut group = c.benchmark_group(format!("groth16/batch_{batch_size}"));

        // Groth16 proofs are intentionally expensive after putting the SHA-256
        // Merkle tree inside the circuit. Keep the statistically meaningful
        // minimum of 10 samples while avoiding Criterion's default 100-sample
        // run, which would be impractical for batch 100.
        group.sample_size(10);
        group.warm_up_time(Duration::from_secs(1));
        group.measurement_time(Duration::from_secs(30));

        group.bench_function("proof_generation", |b| {
            b.iter(|| {
                let circuit = BatchCircuit {
                    leaf_hashes: leaf_hashes.iter().copied().map(Some).collect(),
                    merkle_root_hi: Some(root_hi),
                    merkle_root_lo: Some(root_lo),
                };

                let mut rng = test_rng();

                let proof = Groth16::<Bn254>::create_random_proof_with_reduction(
                    circuit,
                    &proving_key,
                    &mut rng,
                )
                .expect("Groth16 proof generation");

                black_box(proof);
            });
        });

        group.bench_function("local_verification", |b| {
            b.iter(|| {
                let valid = Groth16::<Bn254>::verify_proof(
                    black_box(&prepared_verifying_key),
                    black_box(&verification_proof),
                    black_box(&public_inputs),
                )
                .expect("Groth16 local verification");

                assert!(valid);
                black_box(valid);
            });
        });

        group.finish();

        // `proving_key`, the prepared VK and the proof are dropped here before
        // the next batch size is loaded. This is important for batch 100
        // because its proving key is roughly 1.5 GB.
    }
}

fn criterion_config() -> Criterion {
    Criterion::default()
        .configure_from_args()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5))
}

criterion_group! {
    name = benches;
    config = criterion_config();
    targets = bench_merkle, bench_groth16
}

criterion_main!(benches);
