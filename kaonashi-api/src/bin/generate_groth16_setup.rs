use ark_bn254::{Bn254, Fq, Fq2, G1Affine, G2Affine};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::Groth16;
use ark_serialize::CanonicalSerialize;
use ark_std::test_rng;

use kaonashi_api::groth16::BatchCircuit;

use std::fs;

const BATCH_SIZE: usize = 10;

fn fq_to_be(value: &Fq) -> [u8; 32] {
    let bytes = value.into_bigint().to_bytes_be();

    let mut out = [0u8; 32];
    out[32 - bytes.len()..].copy_from_slice(&bytes);

    out
}

fn g1_to_bytes(point: &G1Affine) -> [u8; 64] {
    let mut out = [0u8; 64];

    out[..32].copy_from_slice(&fq_to_be(&point.x));
    out[32..].copy_from_slice(&fq_to_be(&point.y));

    out
}

fn fq2_to_bytes(value: &Fq2) -> [u8; 64] {
    let mut out = [0u8; 64];

    // Format expected by groth16-solana.
    out[..32].copy_from_slice(&fq_to_be(&value.c1));
    out[32..].copy_from_slice(&fq_to_be(&value.c0));

    out
}

fn g2_to_bytes(point: &G2Affine) -> [u8; 128] {
    let mut out = [0u8; 128];

    out[..64].copy_from_slice(&fq2_to_bytes(&point.x));
    out[64..].copy_from_slice(&fq2_to_bytes(&point.y));

    out
}

fn byte_array(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte}u8"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn main() {
    println!(
        "Generating fixed Groth16 setup for Kaonashi batch size {}...",
        BATCH_SIZE
    );

    let mut rng = test_rng();

    // Same circuit shape that will be used for real 10-vote batches.
    let setup_circuit = BatchCircuit {
        vote_commitments: vec![None; BATCH_SIZE],
        batch_commitment: None,
        batch_size: None,
    };

    let proving_key =
        Groth16::<Bn254>::generate_random_parameters_with_reduction(setup_circuit, &mut rng)
            .expect("Groth16 setup failed");

    // ---------------------------------------------------------
    // Save proving key
    // ---------------------------------------------------------

    fs::create_dir_all("data/groth16").expect("Could not create data/groth16");

    let mut proving_key_bytes = Vec::new();

    proving_key
        .serialize_compressed(&mut proving_key_bytes)
        .expect("Could not serialize proving key");

    fs::write("data/groth16/batch10_proving_key.bin", &proving_key_bytes)
        .expect("Could not write proving key");

    println!("Proving key written: {} bytes", proving_key_bytes.len());

    // ---------------------------------------------------------
    // Convert verifying key to groth16-solana format
    // ---------------------------------------------------------

    let vk = &proving_key.vk;

    let alpha = g1_to_bytes(&vk.alpha_g1);
    let beta = g2_to_bytes(&vk.beta_g2);
    let gamma = g2_to_bytes(&vk.gamma_g2);
    let delta = g2_to_bytes(&vk.delta_g2);

    let mut source = String::new();

    source.push_str("use groth16_solana::groth16::Groth16Verifyingkey;\n\n");

    source.push_str("pub const VERIFYING_KEY: Groth16Verifyingkey = Groth16Verifyingkey {\n");

    // BatchCircuit has:
    // public input 1 = batch_commitment
    // public input 2 = batch_size
    source.push_str("    nr_pubinputs: 2,\n");

    source.push_str(&format!("    vk_alpha_g1: [{}],\n", byte_array(&alpha)));

    source.push_str(&format!("    vk_beta_g2: [{}],\n", byte_array(&beta)));

    // groth16-solana 0.2 uses the field name `vk_gamme_g2`.
    source.push_str(&format!("    vk_gamme_g2: [{}],\n", byte_array(&gamma)));

    source.push_str(&format!("    vk_delta_g2: [{}],\n", byte_array(&delta)));

    source.push_str("    vk_ic: &[\n");

    for point in &vk.gamma_abc_g1 {
        let point_bytes = g1_to_bytes(point);

        source.push_str(&format!("        [{}],\n", byte_array(&point_bytes)));
    }

    source.push_str("    ],\n");
    source.push_str("};\n");

    let verifying_key_path =
        "../kaonashi-smart-contract/programs/projeto-kaonashi/src/groth16_verifying_key.rs";

    fs::write(verifying_key_path, source).expect("Could not write Groth16 verifying key");

    println!("Verifying key written to: {}", verifying_key_path);

    println!("Public inputs: {}", vk.gamma_abc_g1.len() - 1);

    println!("Groth16 setup complete.");
}
