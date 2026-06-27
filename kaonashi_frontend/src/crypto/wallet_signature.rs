use bs58;
use solana_sdk::signature::{Keypair, Signer};

pub fn decode_wallet_keypair(secret_key: &str) -> Result<Keypair, String> {
    let keypair_bytes = bs58::decode(secret_key.trim())
        .into_vec()
        .map_err(|error| format!("Invalid base58 secret key: {error}"))?;

    if keypair_bytes.len() != 64 {
        return Err(format!(
            "Secret key must have 64 bytes, but has {}",
            keypair_bytes.len()
        ));
    }
    Keypair::try_from(keypair_bytes.as_slice())
        .map_err(|error| format!("Invalid Solana keypair: {error}"))
}

pub fn sign_message(secret_key: &str, message: &str) -> Result<String, String> {
    let keypair = decode_wallet_keypair(secret_key)?;

    let signature = keypair.sign_message(message.as_bytes());

    Ok(signature.to_string())
}

pub fn get_public_key(secret_key: &str) -> Result<String, String> {
    let keypair = decode_wallet_keypair(secret_key)?;

    Ok(keypair.pubkey().to_string())
}
