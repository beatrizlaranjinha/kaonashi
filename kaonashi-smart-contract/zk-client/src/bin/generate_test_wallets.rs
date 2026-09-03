use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;
use solana_sdk::signature::{Keypair, Signer};

const OUTPUT_DIRECTORY: &str = "test-wallets";

#[derive(Serialize)]
struct WalletRecord {
    wallet_id: String,
    public_key: String,
    secret_key_32_file: String,
    keypair_64_file: String,
}

fn main() -> Result<()> {
    // ---------------------------------------------------------
    // Read number of wallets from command line
    // ---------------------------------------------------------

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        anyhow::bail!("Usage: cargo run --bin generate_test_wallets -- <NUMBER_OF_WALLETS>");
    }

    let number_of_wallets: usize = args[1]
        .parse()
        .context("NUMBER_OF_WALLETS must be a positive integer")?;

    if number_of_wallets == 0 {
        anyhow::bail!("NUMBER_OF_WALLETS must be greater than zero");
    }

    // ---------------------------------------------------------
    // Create output directory
    // ---------------------------------------------------------

    let output_directory = Path::new(OUTPUT_DIRECTORY);

    fs::create_dir_all(output_directory).context("Failed to create test-wallets directory")?;

    let mut wallet_records = Vec::with_capacity(number_of_wallets);

    // ---------------------------------------------------------
    // Generate wallets
    // ---------------------------------------------------------

    for index in 1..=number_of_wallets {
        let wallet_id = format!("voter_{index:02}");
        let keypair = Keypair::new();

        let secret_32_filename = format!("{wallet_id}_secret32.json");

        let keypair_64_filename = format!("{wallet_id}_keypair64.json");

        let secret_32_path = output_directory.join(&secret_32_filename);

        let keypair_64_path = output_directory.join(&keypair_64_filename);

        save_secret_key_32(&keypair, &secret_32_path)?;

        save_keypair_64(&keypair, &keypair_64_path)?;

        wallet_records.push(WalletRecord {
            wallet_id: wallet_id.clone(),
            public_key: keypair.pubkey().to_string(),
            secret_key_32_file: secret_32_path.to_string_lossy().to_string(),
            keypair_64_file: keypair_64_path.to_string_lossy().to_string(),
        });

        println!("{} -> {}", wallet_id, keypair.pubkey());
    }

    // ---------------------------------------------------------
    // Save wallet registry
    // ---------------------------------------------------------

    let registry_path = output_directory.join("wallets.json");

    let registry_json = serde_json::to_string_pretty(&wallet_records)?;

    fs::write(&registry_path, registry_json).context("Failed to write wallets registry")?;

    println!();
    println!("Generated {} test wallets.", number_of_wallets);

    println!("Registry saved in {}", registry_path.display());

    Ok(())
}

// ============================================================================
// Save 32-byte secret key
// ============================================================================

fn save_secret_key_32(keypair: &Keypair, path: &PathBuf) -> Result<()> {
    let keypair_bytes = keypair.to_bytes();

    let secret_key_32 = &keypair_bytes[0..32];

    let json = serde_json::to_string_pretty(&bs58::encode(secret_key_32).into_string())?;

    fs::write(path, json)
        .with_context(|| format!("Failed to save 32-byte secret key in {}", path.display()))?;

    Ok(())
}

// ============================================================================
// Save complete 64-byte Solana keypair
// ============================================================================

fn save_keypair_64(keypair: &Keypair, path: &PathBuf) -> Result<()> {
    let keypair_64 = keypair.to_bytes();

    let json = serde_json::to_string_pretty(&bs58::encode(keypair_64).into_string())?;

    fs::write(path, json)
        .with_context(|| format!("Failed to save 64-byte keypair in {}", path.display()))?;

    Ok(())
}
