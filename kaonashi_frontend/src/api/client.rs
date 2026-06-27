use crate::crypto::vote_crypto::{
    create_vote_vector, encrypt_vote_with_witness, hash_encrypted_vote,
};
use crate::crypto::wallet_signature::sign_message;
use crate::crypto::zk_vote::{generate_vote_proofs, generate_vote_sum_proof};

use ed25519_dalek::ed25519::signature::SignerMut;
use ed25519_dalek::SigningKey;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use solana_zk_sdk::encryption::elgamal::ElGamalPubkey;

const API_BASE_URL: &str = "http://127.0.0.1:3000";

// =======================================================
// ERROS DA API
// =======================================================

#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    pub error: String,
}

// =======================================================
// AUTENTICAÇÃO DA WALLET
// =======================================================

#[derive(Debug, Serialize)]
pub struct ChallengeRequest {
    pub public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ChallengeResponse {
    pub message: String,
    pub public_key: String,
}

#[derive(Debug, Serialize)]
pub struct WalletLoginRequest {
    pub public_key: String,
    pub message: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct WalletLoginResponse {
    pub authenticated: bool,
    pub public_key: String,
    pub token: String,
}

pub async fn login_wallet(
    public_key: String,
    secret_key: String,
) -> Result<WalletLoginResponse, String> {
    // 1. Pedir ao backend uma mensagem/challenge para assinar.
    let response = Request::post(&format!("{API_BASE_URL}/api/auth/challenge"))
        .header("Content-Type", "application/json")
        .json(&ChallengeRequest {
            public_key: public_key.clone(),
        })
        .map_err(|error| format!("Failed to create challenge request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    let challenge = if response.ok() {
        response
            .json::<ChallengeResponse>()
            .await
            .map_err(|error| format!("Invalid challenge response: {error}"))?
    } else {
        let status = response.status();

        return match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("Failed to get challenge with status {status}")),
        };
    };

    // 2. Converter a secret key base58 para bytes.
    // Nesta versão de login estás a usar uma private key de 32 bytes.
    let secret_key_bytes = bs58::decode(secret_key.trim())
        .into_vec()
        .map_err(|error| format!("Invalid base58 secret key: {error}"))?;

    if secret_key_bytes.len() != 32 {
        return Err(format!(
            "Secret key must have 32 bytes, but has {}",
            secret_key_bytes.len()
        ));
    }

    let mut secret_key_array = [0u8; 32];
    secret_key_array.copy_from_slice(&secret_key_bytes);

    // 3. Criar SigningKey a partir da private key.
    let mut signing_key = SigningKey::from_bytes(&secret_key_array);

    // 4. Assinar a mensagem recebida do backend.
    let signature = signing_key.sign(challenge.message.as_bytes());

    // 5. Converter assinatura para base58.
    let signature_base58 = bs58::encode(signature.to_bytes()).into_string();

    // 6. Enviar public_key + message + signature para o backend validar.
    let response = Request::post(&format!("{API_BASE_URL}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(&WalletLoginRequest {
            public_key,
            message: challenge.message,
            signature: signature_base58,
        })
        .map_err(|error| format!("Failed to create login request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        response
            .json::<WalletLoginResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("Login failed with status {status}")),
        }
    }
}

// =======================================================
// VOTOS CIFRADOS
// =======================================================

#[derive(Debug, Deserialize)]
pub struct ElGamalPublicKeyResponse {
    pub decade_id: u8,
    pub decade: String,

    // Public key ElGamal da década/ballot.
    // O frontend usa esta chave para cifrar o voto.
    pub public_key: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RistrettoVoteProof {
    pub a0: Vec<u8>,
    pub b0: Vec<u8>,
    pub c0: Vec<u8>,
    pub s0: Vec<u8>,

    pub a1: Vec<u8>,
    pub b1: Vec<u8>,
    pub c1: Vec<u8>,
    pub s1: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RistrettoVoteSumProof {
    pub a: Vec<u8>,
    pub b: Vec<u8>,
    pub c: Vec<u8>,
    pub s: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct SubmitVoteRequest {
    pub wallet_id: String,
    pub public_key: String,
    pub decade_id: u8,

    pub encrypted_vote: Vec<Vec<u8>>,
    pub encrypted_vote_hash: String,

    pub vote_proofs: Vec<RistrettoVoteProof>,
    pub vote_sum_proof: RistrettoVoteSumProof,

    pub message: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitVoteResponse {
    pub accepted: bool,
    pub decade_id: u8,
    pub decade: String,

    // O backend já não sabe o filme escolhido.
    // Estes campos ficam com default para o frontend continuar a mostrar a UI.
    #[serde(default)]
    pub movie_index: usize,

    #[serde(default)]
    pub movie: String,

    pub pending_votes: usize,
    pub batch_submitted: bool,
}

// Vai buscar ao backend a ElGamal public key da década escolhida.
async fn get_elgamal_public_key(decade_id: u8) -> Result<ElGamalPubkey, String> {
    let response = Request::get(&format!(
        "{API_BASE_URL}/api/election/{decade_id}/elgamal-public-key"
    ))
    .send()
    .await
    .map_err(|error| format!("Failed to contact the API: {error}"))?;

    let response = if response.ok() {
        response
            .json::<ElGamalPublicKeyResponse>()
            .await
            .map_err(|error| format!("Invalid ElGamal public key response: {error}"))?
    } else {
        let status = response.status();

        return match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!(
                "Failed to get ElGamal public key with status {status}"
            )),
        };
    };

    // A ElGamal public key deve ter 32 bytes.
    if response.public_key.len() != 32 {
        return Err(format!(
            "ElGamal public key must have 32 bytes, got {}",
            response.public_key.len()
        ));
    }

    ElGamalPubkey::try_from(response.public_key.as_slice())
        .map_err(|_| "Invalid ElGamal public key bytes".to_string())
}

pub async fn submit_vote(
    wallet_id: String,
    public_key: String,
    secret_key: String,
    decade_id: u8,
    movie_index: usize,
    movie_name: String,
) -> Result<SubmitVoteResponse, String> {
    // 1. Buscar a ElGamal public key da década.
    // Esta chave pertence à eleição/década, não à wallet.
    let elgamal_public_key = get_elgamal_public_key(decade_id).await?;

    // 2. Criar o vetor one-hot localmente.
    //
    // Exemplo:
    // movie_index = 2
    // vote_vector = [0,0,1,0,0,0,0,0]
    let vote_vector = create_vote_vector(movie_index, 8)?;

    // 3. Validar e cifrar o voto.
    //
    // Dentro de encrypt_vote:
    // - validate_vote_vector verifica VoteProof e VoteSumProof localmente
    // - cada valor é cifrado com ElGamal
    // - agora usando PedersenOpening no vote_crypto.rs
    let encrypted_witness = encrypt_vote_with_witness(&vote_vector, &elgamal_public_key)?;

    let encrypted_vote = encrypted_witness.encrypted_vote;
    let opening_scalars = encrypted_witness.opening_scalars;

    if opening_scalars.len() != encrypted_vote.len() {
        return Err("Missing openings for encrypted vote".to_string());
    }

    let vote_proofs = generate_vote_proofs(
        &elgamal_public_key,
        &vote_vector,
        &encrypted_vote,
        &opening_scalars,
    )?;

    let vote_sum_proof =
        generate_vote_sum_proof(&elgamal_public_key, &encrypted_vote, &opening_scalars)?;

    // TESTE: estragar a proof de propósito
    /*let mut vote_proofs = vote_proofs;
    vote_proofs[0].c0[0] ^= 1;*/

    // 4. Calcular hash do voto cifrado.
    // Este hash identifica exatamente os ciphertexts enviados.
    let encrypted_vote_hash = hash_encrypted_vote(&encrypted_vote);

    // 5. Criar mensagem assinada.
    //
    // A mensagem não contém movie_index nem movie_name.
    // Assim, o backend não recebe o voto em claro.
    //
    // IMPORTANTE:
    // Este texto tem de ser igual ao texto reconstruído no backend.
    let message = format!(
        "Kaonashi encrypted vote\nwallet_id: {}\npublic_key: {}\ndecade_id: {}\nencrypted_vote_hash: {}",
        wallet_id,
        public_key,
        decade_id,
        encrypted_vote_hash
    );

    // 6. Assinar mensagem com a secret key da wallet.
    let signature_base58 = sign_message(&secret_key, &message)?;

    leptos::logging::log!("ENCRYPTED VOTE HASH: {}", encrypted_vote_hash);
    leptos::logging::log!("SIGNED MESSAGE: {}", message);
    leptos::logging::log!("VOTE SIGNATURE: {}", signature_base58);

    // 7. Converter Vec<[u8; 64]> para Vec<Vec<u8>>.
    // JSON não envia arrays fixos [u8; 64] diretamente.
    let encrypted_vote_json = encrypted_vote
        .iter()
        .map(|ciphertext| ciphertext.to_vec())
        .collect::<Vec<Vec<u8>>>();

    // 8. Enviar voto cifrado para a API.
    let response = Request::post(&format!("{API_BASE_URL}/api/vote"))
        .header("Content-Type", "application/json")
        .json(&SubmitVoteRequest {
            wallet_id,
            public_key,
            decade_id,
            encrypted_vote: encrypted_vote_json,
            encrypted_vote_hash,
            vote_proofs,
            vote_sum_proof,
            message,
            signature: signature_base58,
        })
        .map_err(|error| format!("Failed to create vote request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        let mut vote_response = response
            .json::<SubmitVoteResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))?;

        // Como o backend já não sabe o filme escolhido,
        // preenchemos estes campos localmente só para a UI.
        if vote_response.movie.is_empty() {
            vote_response.movie = movie_name;
            vote_response.movie_index = movie_index;
        }

        Ok(vote_response)
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("The API rejected the vote with status {status}")),
        }
    }
}

// =======================================================
// BATCHES
// =======================================================

#[derive(Debug, Serialize)]
pub struct FlushBatchRequest {
    pub wallet_id: String,
    pub public_key: String,
    pub decade_id: u8,
}

#[derive(Debug, Deserialize)]
pub struct FlushBatchResponse {
    pub submitted: bool,
    pub decade_id: u8,
    pub decade: String,
    pub batch_size: usize,
    pub pending_votes: usize,
}

pub async fn flush_batch(
    wallet_id: String,
    public_key: String,
    decade_id: u8,
) -> Result<FlushBatchResponse, String> {
    let response = Request::post(&format!("{API_BASE_URL}/api/admin/flush-batch"))
        .header("Content-Type", "application/json")
        .json(&FlushBatchRequest {
            wallet_id,
            public_key,
            decade_id,
        })
        .map_err(|error| format!("Failed to create flush request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        response
            .json::<FlushBatchResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("Failed to submit batch with status {status}")),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FlushBatchesResponse {
    pub submitted: bool,
    pub total_batches: usize,
    pub total_votes: usize,
    pub results: Vec<DecadeOperationResult>,
}

pub async fn flush_batches(
    wallet_id: String,
    public_key: String,
) -> Result<FlushBatchesResponse, String> {
    let response = Request::post(&format!("{API_BASE_URL}/api/admin/flush-batches"))
        .header("Content-Type", "application/json")
        .json(&AdminElectionRequest {
            wallet_id,
            public_key,
        })
        .map_err(|error| format!("Failed to create flush batches request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        response
            .json::<FlushBatchesResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("Failed to submit batches with status {status}")),
        }
    }
}

// =======================================================
// RESULTADOS
// =======================================================

#[derive(Debug, Deserialize, Clone)]
pub struct MovieResult {
    pub index: usize,
    pub title: String,
    pub votes: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResultsResponse {
    pub decade_id: u8,
    pub decade: String,
    pub ballot_address: String,
    pub total_votes: usize,
    pub winner_index: Option<usize>,
    pub winner: Option<String>,
    pub tie_indices: Vec<usize>,
    pub final_winner_index: Option<usize>,
    pub final_winner: Option<String>,
    pub results: Vec<MovieResult>,
}

pub async fn get_results(decade_id: u8) -> Result<ResultsResponse, String> {
    let response = Request::get(&format!("{API_BASE_URL}/api/results/{decade_id}"))
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        response
            .json::<ResultsResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("Failed to load results with status {status}")),
        }
    }
}

// =======================================================
// EMPATES
// =======================================================

#[derive(Debug, Serialize)]
pub struct ResolveTieRequest {
    pub wallet_id: String,
    pub public_key: String,
    pub decade_id: u8,
    pub winner_index: usize,
}

#[derive(Debug, Deserialize)]
pub struct ResolveTieResponse {
    pub resolved: bool,
    pub decade_id: u8,
    pub decade: String,
    pub winner_index: usize,
    pub winner: String,
}

pub async fn resolve_tie(
    wallet_id: String,
    public_key: String,
    decade_id: u8,
    winner_index: usize,
) -> Result<ResolveTieResponse, String> {
    let response = Request::post(&format!("{API_BASE_URL}/api/admin/resolve-tie"))
        .header("Content-Type", "application/json")
        .json(&ResolveTieRequest {
            wallet_id,
            public_key,
            decade_id,
            winner_index,
        })
        .map_err(|error| format!("Failed to create tie request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        response
            .json::<ResolveTieResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("Failed to resolve tie with status {status}")),
        }
    }
}

// =======================================================
// ESTADO DA ELEIÇÃO
// =======================================================

#[derive(Debug, Deserialize, Clone)]
pub struct IncompleteVoter {
    pub wallet_id: String,
    pub missing_decades: Vec<u8>,
    pub missing_decade_names: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ElectionCompletionResponse {
    pub complete: bool,
    pub eligible_voters: usize,
    pub completed_voters: usize,
    pub incomplete_voters: Vec<IncompleteVoter>,
}

pub async fn get_election_completion() -> Result<ElectionCompletionResponse, String> {
    let response = Request::get(&format!("{API_BASE_URL}/api/admin/election-completion"))
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        response
            .json::<ElectionCompletionResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!(
                "Failed to load election completion with status {status}"
            )),
        }
    }
}

// =======================================================
// FUNÇÕES CHAIRPERSON
// =======================================================

#[derive(Debug, Serialize)]
pub struct AdminElectionRequest {
    pub wallet_id: String,
    pub public_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DecadeOperationResult {
    pub decade_id: u8,
    pub decade: String,
    pub success: bool,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CloseElectionResponse {
    pub closed: bool,
    pub results: Vec<DecadeOperationResult>,
}

pub async fn close_election(
    wallet_id: String,
    public_key: String,
) -> Result<CloseElectionResponse, String> {
    let response = Request::post(&format!("{API_BASE_URL}/api/admin/close-election"))
        .header("Content-Type", "application/json")
        .json(&AdminElectionRequest {
            wallet_id,
            public_key,
        })
        .map_err(|error| format!("Failed to create close election request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        response
            .json::<CloseElectionResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("Failed to close election with status {status}")),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FinalizeElectionResponse {
    pub finalized: bool,
    pub results: Vec<DecadeOperationResult>,
}

pub async fn finalize_election(
    wallet_id: String,
    public_key: String,
) -> Result<FinalizeElectionResponse, String> {
    let response = Request::post(&format!("{API_BASE_URL}/api/admin/finalize-election"))
        .header("Content-Type", "application/json")
        .json(&AdminElectionRequest {
            wallet_id,
            public_key,
        })
        .map_err(|error| format!("Failed to create finalize request: {error}"))?
        .send()
        .await
        .map_err(|error| format!("Failed to contact the API: {error}"))?;

    if response.ok() {
        response
            .json::<FinalizeElectionResponse>()
            .await
            .map_err(|error| format!("Invalid API response: {error}"))
    } else {
        let status = response.status();

        match response.json::<ApiErrorResponse>().await {
            Ok(api_error) => Err(api_error.error),
            Err(_) => Err(format!("Failed to finalize election with status {status}")),
        }
    }
}
