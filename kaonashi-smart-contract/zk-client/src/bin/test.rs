use serde::Deserialize;
use solana_sdk::signature::{Keypair, Signer};
use std::{env, fs, path::Path};

#[derive(Debug, Deserialize)]
struct WalletRecord {
    wallet_id: String,
    public_key: String,
    keypair_file: String,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        return Err(
            "Usage: cargo run --bin test_keypair -- <wallets_json_path> <wallet_id>".to_string(),
        );
    }

    let wallets_path = &args[1];
    let wallet_id = &args[2];

    let wallets_json = fs::read_to_string(wallets_path)
        .map_err(|e| format!("Failed to read wallets.json: {e}"))?;

    let wallets: Vec<WalletRecord> =
        serde_json::from_str(&wallets_json).map_err(|e| format!("Invalid wallets.json: {e}"))?;

    let wallet = wallets
        .iter()
        .find(|w| w.wallet_id == *wallet_id)
        .ok_or_else(|| format!("Wallet {wallet_id} not found"))?;

    let keypair_path = Path::new(wallets_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(
            Path::new(&wallet.keypair_file)
                .file_name()
                .ok_or("Invalid keypair_file")?,
        );

    let keypair_json = fs::read_to_string(&keypair_path)
        .map_err(|e| format!("Failed to read {}: {e}", keypair_path.display()))?;

    let keypair_base58: String = serde_json::from_str(&keypair_json)
        .map_err(|e| format!("Invalid keypair JSON string: {e}"))?;

    let keypair_bytes = bs58::decode(keypair_base58)
        .into_vec()
        .map_err(|e| format!("Invalid base58 keypair: {e}"))?;

    let keypair =
        Keypair::from_bytes(&keypair_bytes).map_err(|e| format!("Invalid Solana keypair: {e}"))?;

    if keypair.pubkey().to_string() != wallet.public_key {
        return Err(format!(
            "Public key mismatch. wallets.json has {}, keypair gives {}",
            wallet.public_key,
            keypair.pubkey()
        ));
    }

    let message = b"Kaonashi test message";
    let signature = keypair.sign_message(message);
    let valid = signature.verify(keypair.pubkey().as_ref(), message);

    println!("Wallet ID: {}", wallet.wallet_id);
    println!("Public key: {}", keypair.pubkey());
    println!("Keypair file: {}", keypair_path.display());
    println!("Signature: {}", signature);
    println!("Valid signature: {}", valid);

    if !valid {
        return Err("Signature verification failed".to_string());
    }

    Ok(())
}
