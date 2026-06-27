use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};

// we are not using jwt no more , this is useless rigth??_________

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub public_key: String,
    pub exp: usize,
}

// Cria uma mensagem única que o frontend vai assinar com a private key.
pub fn create_login_message(public_key: &str) -> String {
    // Gera 32 bytes aleatórios .
    let mut nonce_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut nonce_bytes);

    // Converte o nonce para hexadecimal para ficar fácil de enviar como texto.
    let nonce = bs58::encode(nonce_bytes).into_string();

    format!(
        "Kaonashi login\nPublic key: {}\nNonce: {}",
        public_key, nonce
    )
}

//_________ Verifica se a assinatura foi feita pela private key correspondente à public key.________________
pub fn verify_signature(public_key: &str, message: &str, signature: &str) -> Result<(), String> {
    // A public_key vem em base58, formato típico Solana.
    let public_key_bytes = bs58::decode(public_key)
        .into_vec()
        .map_err(|error| format!("Invalid public key base58: {}", error))?;

    if public_key_bytes.len() != 32 {
        return Err(format!(
            "Public key must have 32 bytes, but has {}",
            public_key_bytes.len()
        ));
    }

    let mut public_key_array = [0u8; 32];
    public_key_array.copy_from_slice(&public_key_bytes);

    // Cria a verifying key a partir da public key.
    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|error| format!("Invalid verifying key: {}", error))?;

    // A assinatura também vem em base58.
    let signature_bytes = bs58::decode(signature)
        .into_vec()
        .map_err(|error| format!("Invalid signature base58: {}", error))?;

    if signature_bytes.len() != 64 {
        return Err(format!(
            "Signature must have 64 bytes, but has {}",
            signature_bytes.len()
        ));
    }

    let mut signature_array = [0u8; 64];
    signature_array.copy_from_slice(&signature_bytes);

    let signature = Signature::from_bytes(&signature_array);

    // Verifica se a assinatura corresponde à mensagem e à public key.
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| "Invalid signature for this public key".to_string())?;

    Ok(())
}
