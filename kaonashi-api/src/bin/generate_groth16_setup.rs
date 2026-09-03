use ark_bn254::{Bn254, Fq, Fq2, G1Affine, G2Affine};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::Groth16;
use ark_serialize::CanonicalSerialize;
use ark_std::test_rng;

use kaonashi_api::groth16::{BatchCircuit, SUPPORTED_GROTH16_BATCH_SIZES};

use std::{env, fs, path::Path};

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

fn verifying_key_path(batch_size: usize) -> String {
    match batch_size {
        10 => "../kaonashi-smart-contract/programs/projeto-kaonashi/src/groth16_verifying_key.rs"
            .to_string(),
        50 => {
            "../kaonashi-smart-contract/programs/projeto-kaonashi/src/groth16_verifying_key_50.rs"
                .to_string()
        }
        100 => {
            "../kaonashi-smart-contract/programs/projeto-kaonashi/src/groth16_verifying_key_100.rs"
                .to_string()
        }
        _ => unreachable!("validated batch size"),
    }
}

fn main() {
    let batch_size = env::args()
        .nth(1)
        .unwrap_or_else(|| {
            eprintln!(
                "Usage: cargo run --bin generate_groth16_setup -- <BATCH_SIZE>\nSupported sizes: {:?}",
                SUPPORTED_GROTH16_BATCH_SIZES
            );
            std::process::exit(1);
        })
        .parse::<usize>()
        .unwrap_or_else(|_| {
            eprintln!("Batch size must be an integer");
            std::process::exit(1);
        });

    if !SUPPORTED_GROTH16_BATCH_SIZES.contains(&batch_size) {
        eprintln!(
            "Unsupported batch size {}. Supported sizes: {:?}",
            batch_size, SUPPORTED_GROTH16_BATCH_SIZES
        );
        std::process::exit(1);
    }

    let proving_key_path = format!("data/groth16/batch{}_proving_key.bin", batch_size);
    let verifying_key_path = verifying_key_path(batch_size);

    // The proving key must not already exist, because we do not want to
    // accidentally replace a setup without noticing.
    //
    // The verifying-key Rust module MAY already exist. In fact, the Solana
    // program needs those modules to exist so that the dependency graph can
    // compile while this generator is running. The generator therefore
    // intentionally overwrites only the corresponding verifying-key module.
    if Path::new(&proving_key_path).exists() {
        eprintln!(
            "Refusing to overwrite existing proving key:\n  {}\nBack it up or remove it first.",
            proving_key_path
        );
        std::process::exit(1);
    }

    if Path::new(&verifying_key_path).exists() {
        println!(
            "Existing verifying-key module will be replaced: {}",
            verifying_key_path
        );
    }

    println!(
        "Generating Merkle-bound Groth16 setup for Kaonashi batch size {}...",
        batch_size
    );

    let mut rng = test_rng();

    let setup_circuit = BatchCircuit {
        leaf_hashes: vec![None; batch_size],
        merkle_root_hi: None,
        merkle_root_lo: None,
    };

    let proving_key =
        Groth16::<Bn254>::generate_random_parameters_with_reduction(setup_circuit, &mut rng)
            .expect("Groth16 setup failed");

    fs::create_dir_all("data/groth16").expect("Could not create data/groth16");

    let mut proving_key_bytes = Vec::new();
    proving_key
        .serialize_compressed(&mut proving_key_bytes)
        .expect("Could not serialize proving key");

    fs::write(&proving_key_path, &proving_key_bytes).expect("Could not write proving key");

    let vk = &proving_key.vk;

    let alpha = g1_to_bytes(&vk.alpha_g1);
    let beta = g2_to_bytes(&vk.beta_g2);
    let gamma = g2_to_bytes(&vk.gamma_g2);
    let delta = g2_to_bytes(&vk.delta_g2);

    let mut source = String::new();

    source.push_str("use groth16_solana::groth16::Groth16Verifyingkey;\n\n");
    source.push_str("pub const VERIFYING_KEY: Groth16Verifyingkey = Groth16Verifyingkey {\n");
    source.push_str("    nr_pubinputs: 2,\n");
    source.push_str(&format!("    vk_alpha_g1: [{}],\n", byte_array(&alpha)));
    source.push_str(&format!("    vk_beta_g2: [{}],\n", byte_array(&beta)));
    source.push_str(&format!("    vk_gamme_g2: [{}],\n", byte_array(&gamma)));
    source.push_str(&format!("    vk_delta_g2: [{}],\n", byte_array(&delta)));
    source.push_str("    vk_ic: &[\n");

    for point in &vk.gamma_abc_g1 {
        let point_bytes = g1_to_bytes(point);
        source.push_str(&format!("        [{}],\n", byte_array(&point_bytes)));
    }

    source.push_str("    ],\n");
    source.push_str("};\n");

    fs::write(&verifying_key_path, source).expect("Could not write Groth16 verifying key");

    println!(
        "Proving key written: {} ({} bytes)",
        proving_key_path,
        proving_key_bytes.len()
    );
    println!("Verifying key written: {}", verifying_key_path);
    println!("Public inputs: {}", vk.gamma_abc_g1.len() - 1);
    println!("Groth16 setup complete.");
}
