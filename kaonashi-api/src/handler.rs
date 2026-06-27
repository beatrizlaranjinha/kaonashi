use std::sync::Arc;

use crate::blockchain::get_ballot_state_from_blockchain;
use crate::models::BlockchainBallotResponse;
use axum::extract::{Path, State};
use axum::Json;
use sha2::{Digest, Sha256};

use crate::auth::{create_login_message, verify_signature};
use crate::batches::{create_batch_for_decade, MAX_BATCH_SIZE};
use crate::blockchain::create_all_ballots_on_chain;
use crate::keeping_votes::KeepingVotes;
use crate::merkle::{verify_merkle_proof, MerkleProofNode};
use crate::models::{
    ChallengeRequest, ChallengeResponse, ElGamalPublicKeyResponse, FlushBatchResponse,
    LoginRequest, LoginResponse, PendingEncryptedVote, SubmitVoteResponse, SubmittedVote,
    VerifyReceiptRequest, VerifyReceiptResponse, VoteReceipt,
};
use crate::movies::movies_decades;
use crate::zk_verify::verify_encrypted_vote_proofs;

use solana_zk_sdk::encryption::elgamal::ElGamalPubkey;

// Testa se a API está online.
pub async fn is_running() -> &'static str {
    "api is indeed running"
}

// Devolve a ElGamal public key da década.
pub async fn get_elgamal_public_key(
    Path(decade_id): Path<u8>,
    State(keeping_votes): State<Arc<KeepingVotes>>,
) -> Result<Json<ElGamalPublicKeyResponse>, String> {
    if movies_decades(decade_id).is_none() {
        return Err("Invalid decade".to_string());
    }

    let keypairs = keeping_votes.elgamal_keypairs_by_decade.lock().unwrap();

    let Some(keypair) = keypairs.get(decade_id as usize) else {
        return Err("No ElGamal keypair found for this decade".to_string());
    };

    Ok(Json(ElGamalPublicKeyResponse {
        decade_id,
        decade: format!("{}s", decade_label(decade_id)),
        public_key: keypair.pubkey().to_bytes().to_vec(),
    }))
}

// Recebe, valida e guarda um voto cifrado.
pub async fn submit_vote(
    State(keeping_votes): State<Arc<KeepingVotes>>,
    Json(vote): Json<SubmittedVote>,
) -> Json<SubmitVoteResponse> {
    let Some(movies) = movies_decades(vote.decade_id) else {
        return Json(vote_response(
            false,
            vote.wallet_id,
            vote.decade_id,
            "invalid decade".to_string(),
            0,
            false,
            "Invalid decade".to_string(),
        ));
    };

    let proposal_count = movies.len();

    let encrypted_vote = match normalize_encrypted_vote(&vote.encrypted_vote, proposal_count) {
        Ok(encrypted_vote) => encrypted_vote,
        Err(error) => {
            println!("Invalid encrypted vote: {}", error);

            return Json(vote_response(
                false,
                vote.wallet_id,
                vote.decade_id,
                format!("{}s", decade_label(vote.decade_id)),
                0,
                false,
                "Invalid encrypted vote".to_string(),
            ));
        }
    };

    let expected_hash = hash_encrypted_vote(&encrypted_vote);

    if expected_hash != vote.encrypted_vote_hash {
        println!("Invalid encrypted vote hash");

        return Json(vote_response(
            false,
            vote.wallet_id,
            vote.decade_id,
            format!("{}s", decade_label(vote.decade_id)),
            0,
            false,
            "Invalid encrypted vote hash".to_string(),
        ));
    }

    let expected_message = format!(
        "Kaonashi encrypted vote\nwallet_id: {}\npublic_key: {}\ndecade_id: {}\nencrypted_vote_hash: {}",
        vote.wallet_id, vote.public_key, vote.decade_id, vote.encrypted_vote_hash
    );

    if vote.message != expected_message {
        println!("Vote message does not match encrypted vote");

        return Json(vote_response(
            false,
            vote.wallet_id,
            vote.decade_id,
            format!("{}s", decade_label(vote.decade_id)),
            0,
            false,
            "Vote message does not match".to_string(),
        ));
    }

    if let Err(error) = verify_signature(&vote.public_key, &vote.message, &vote.signature) {
        println!("Invalid vote signature: {}", error);

        return Json(vote_response(
            false,
            vote.wallet_id,
            vote.decade_id,
            format!("{}s", decade_label(vote.decade_id)),
            0,
            false,
            "Invalid vote signature".to_string(),
        ));
    }

    println!("Encrypted vote signature verified");
    println!("Encrypted vote hash: {}", vote.encrypted_vote_hash);
    println!("Received vote proofs: {}", vote.vote_proofs.len());

    let elgamal_public_key = {
        let keypairs = keeping_votes.elgamal_keypairs_by_decade.lock().unwrap();

        let Some(keypair) = keypairs.get(vote.decade_id as usize) else {
            return Json(vote_response(
                false,
                vote.wallet_id.clone(),
                vote.decade_id,
                format!("{}s", decade_label(vote.decade_id)),
                0,
                false,
                "Invalid decade keypair".to_string(),
            ));
        };

        let public_key_bytes = keypair.pubkey().to_bytes();

        ElGamalPubkey::try_from(public_key_bytes.as_slice())
            .map_err(|_| "Invalid ElGamal public key".to_string())
    };

    let Ok(elgamal_public_key) = elgamal_public_key else {
        return Json(vote_response(
            false,
            vote.wallet_id.clone(),
            vote.decade_id,
            format!("{}s", decade_label(vote.decade_id)),
            0,
            false,
            "Invalid ElGamal public key".to_string(),
        ));
    };

    if let Err(error) = verify_encrypted_vote_proofs(
        &elgamal_public_key,
        &encrypted_vote,
        &vote.vote_proofs,
        &vote.vote_sum_proof,
    ) {
        println!("Invalid vote proof: {}", error);

        return Json(vote_response(
            false,
            vote.wallet_id.clone(),
            vote.decade_id,
            format!("{}s", decade_label(vote.decade_id)),
            0,
            false,
            format!("Invalid vote proof: {}", error),
        ));
    }

    println!("Encrypted vote proofs verified");

    let mut pending_votes = keeping_votes.pending_encrypted_votes.lock().unwrap();

    pending_votes[vote.decade_id as usize].push(PendingEncryptedVote {
        wallet_id: vote.wallet_id.clone(),
        public_key: vote.public_key.clone(),
        decade_id: vote.decade_id,
        encrypted_vote_hash: vote.encrypted_vote_hash.clone(),
        encrypted_vote,
    });

    let pending_votes_count = pending_votes[vote.decade_id as usize].len();

    drop(pending_votes);

    let batch_submitted = if pending_votes_count >= MAX_BATCH_SIZE {
        match create_batch_for_decade(keeping_votes.as_ref(), vote.decade_id) {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(error) => {
                println!("Auto flush failed: {}", error);
                false
            }
        }
    } else {
        false
    };

    Json(vote_response(
        true,
        vote.wallet_id,
        vote.decade_id,
        format!("{}s", decade_label(vote.decade_id)),
        if batch_submitted {
            0
        } else {
            pending_votes_count
        },
        batch_submitted,
        "Encrypted vote accepted".to_string(),
    ))
}

// Cria uma resposta padrão para submissão de voto.
fn vote_response(
    accepted: bool,
    wallet_id: String,
    decade_id: u8,
    decade: String,
    pending_votes: usize,
    batch_submitted: bool,
    status: String,
) -> SubmitVoteResponse {
    SubmitVoteResponse {
        accepted,
        wallet_id,
        decade_id,
        decade,
        movie_index: 0,
        movie: String::new(),
        status,
        pending_votes,
        batch_submitted,
    }
}

// Converte o voto cifrado recebido por JSON para o formato interno.
fn normalize_encrypted_vote(
    encrypted_vote: &[Vec<u8>],
    proposal_count: usize,
) -> Result<Vec<[u8; 64]>, String> {
    if encrypted_vote.len() != proposal_count {
        return Err(format!(
            "Expected {} ciphertexts, got {}",
            proposal_count,
            encrypted_vote.len()
        ));
    }

    encrypted_vote
        .iter()
        .map(|ciphertext| {
            if ciphertext.len() != 64 {
                return Err(format!(
                    "Each ciphertext must have 64 bytes, got {}",
                    ciphertext.len()
                ));
            }

            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(ciphertext);

            Ok(bytes)
        })
        .collect()
}

// Calcula o hash do voto cifrado.
fn hash_encrypted_vote(encrypted_vote: &[[u8; 64]]) -> String {
    let mut hasher = Sha256::new();

    for ciphertext in encrypted_vote {
        hasher.update(ciphertext);
    }

    hex::encode(hasher.finalize())
}

// Cria manualmente um batch com os votos pendentes de uma década.
pub async fn flush_batch(
    Path(decade_id): Path<u8>,
    State(keeping_votes): State<Arc<KeepingVotes>>,
) -> Json<FlushBatchResponse> {
    if movies_decades(decade_id).is_none() {
        return Json(FlushBatchResponse {
            success: false,
            decade_id,
            batch_id: String::new(),
            merkle_root: String::new(),
            vote_count: 0,
            encrypted_batch_tally: Vec::new(),
            receipts: Vec::new(),
            status: "No pending encrypted votes".to_string(),
        });
    }

    match create_batch_for_decade(keeping_votes.as_ref(), decade_id) {
        Ok(Some(response)) => Json(response),

        Ok(None) => Json(FlushBatchResponse {
            success: false,
            decade_id,
            batch_id: String::new(),
            merkle_root: String::new(),
            vote_count: 0,
            encrypted_batch_tally: Vec::new(),
            receipts: Vec::new(),
            status: "No pending encrypted votes".to_string(),
        }),

        Err(error) => Json(FlushBatchResponse {
            success: false,
            decade_id,
            batch_id: String::new(),
            merkle_root: String::new(),
            vote_count: 0,
            encrypted_batch_tally: Vec::new(),
            receipts: Vec::new(),
            status: error,
        }),
    }
}

// Devolve o receipt de um voto através do hash do voto cifrado.
pub async fn get_vote_receipt(
    Path(vote_hash): Path<String>,
    State(keeping_votes): State<Arc<KeepingVotes>>,
) -> Json<Option<VoteReceipt>> {
    let receipts = keeping_votes.vote_receipts_by_hash.lock().unwrap();

    Json(receipts.get(&vote_hash).cloned())
}

// Verifica se o receipt guardado reconstrói a Merkle root.
pub async fn verify_vote_receipt(
    State(keeping_votes): State<Arc<KeepingVotes>>,
    Json(payload): Json<VerifyReceiptRequest>,
) -> Json<VerifyReceiptResponse> {
    let receipts = keeping_votes.vote_receipts_by_hash.lock().unwrap();

    let Some(receipt) = receipts.get(&payload.vote_hash) else {
        return Json(VerifyReceiptResponse {
            vote_hash: payload.vote_hash,
            verified: false,
            batch_id: String::new(),
            merkle_root: String::new(),
            status: "Receipt not found".to_string(),
        });
    };

    let proof = receipt
        .merkle_proof
        .iter()
        .map(|node| MerkleProofNode {
            hash: node.hash.clone(),
            is_left: node.is_left,
        })
        .collect::<Vec<MerkleProofNode>>();

    let verified = verify_merkle_proof(&receipt.leaf_hash, &proof, &receipt.merkle_root);

    Json(VerifyReceiptResponse {
        vote_hash: receipt.vote_hash.clone(),
        verified,
        batch_id: receipt.batch_id.clone(),
        merkle_root: receipt.merkle_root.clone(),
        status: if verified {
            "Receipt verified".to_string()
        } else {
            "Receipt verification failed".to_string()
        },
    })
}

// Devolve resultados antigos em claro.
pub async fn get_results(
    Path(decade_id): Path<u8>,
    State(keeping_votes): State<Arc<KeepingVotes>>,
) -> String {
    let Some(movies) = movies_decades(decade_id) else {
        return "invalid decade".to_string();
    };

    let votes = keeping_votes.votes_by_decade.lock().unwrap();

    let selected_decade_votes = &votes[decade_id as usize];

    movies
        .iter()
        .zip(selected_decade_votes.iter())
        .map(|(movie, vote_count)| format!("{}:{}", movie, vote_count))
        .collect::<Vec<String>>()
        .join("\n")
}

// Devolve vencedor antigo em claro.
pub async fn get_winner(
    Path(decade_id): Path<u8>,
    State(keeping_votes): State<Arc<KeepingVotes>>,
) -> String {
    let Some(movies) = movies_decades(decade_id) else {
        return "invalid decade".to_string();
    };

    let votes = keeping_votes.votes_by_decade.lock().unwrap();

    let selected_decade_votes = &votes[decade_id as usize];

    let Some((winner_index, winner_votes)) = selected_decade_votes
        .iter()
        .enumerate()
        .max_by_key(|(_, vote_count)| *vote_count)
    else {
        return "no votes found".to_string();
    };

    if *winner_votes == 0 {
        return "No votes yet".to_string();
    }

    let tied_movies = selected_decade_votes
        .iter()
        .enumerate()
        .filter_map(|(index, vote_count)| {
            if vote_count == winner_votes {
                Some(movies[index].clone())
            } else {
                None
            }
        })
        .collect::<Vec<String>>();

    if tied_movies.len() > 1 {
        return format!(
            "tie between: {} with {} votes",
            tied_movies.join(", "),
            winner_votes
        );
    }

    format!(
        "Winner: {} with {} votes",
        movies[winner_index], winner_votes
    )
}

// Devolve os filmes de uma década.
pub async fn get_movies(Path(decade_id): Path<u8>) -> Json<Option<Vec<String>>> {
    let movies = movies_decades(decade_id)
        .map(|movies| movies.iter().map(|movie| movie.to_string()).collect());

    Json(movies)
}

// Cria os ballots na blockchain.
pub async fn create_ballots(State(keeping_votes): State<Arc<KeepingVotes>>) -> String {
    let elgamal_public_keys_by_decade = {
        let keypairs = keeping_votes.elgamal_keypairs_by_decade.lock().unwrap();

        keypairs
            .iter()
            .map(|keypair| keypair.pubkey().to_bytes())
            .collect::<Vec<[u8; 32]>>()
    };

    let result = tokio::task::spawn_blocking(move || {
        create_all_ballots_on_chain(elgamal_public_keys_by_decade)
    })
    .await;

    match result {
        Ok(Ok(ballots)) => ballots.join("\n"),
        Ok(Err(error)) => format!("Blockchain error: {}", error),
        Err(error) => format!("Blockchain task failed: {}", error),
    }
}

// Cria uma mensagem de autenticação para a wallet assinar.
pub async fn create_auth_challenge(
    State(keeping_votes): State<Arc<KeepingVotes>>,
    Json(payload): Json<ChallengeRequest>,
) -> Json<ChallengeResponse> {
    let message = create_login_message(&payload.public_key);

    {
        let mut challenges = keeping_votes.login_challenges.lock().unwrap();

        challenges.insert(payload.public_key.clone(), message.clone());
    }

    Json(ChallengeResponse {
        public_key: payload.public_key,
        message,
    })
}

// Verifica o login por assinatura.
pub async fn login_with_signature(
    State(keeping_votes): State<Arc<KeepingVotes>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, String> {
    {
        let mut challenges = keeping_votes.login_challenges.lock().unwrap();

        let Some(expected_message) = challenges.get(&payload.public_key) else {
            return Err("No login challenge found for this public key".to_string());
        };

        if expected_message != &payload.message {
            return Err("Login message does not match the current challenge".to_string());
        }

        verify_signature(&payload.public_key, &payload.message, &payload.signature)?;

        challenges.remove(&payload.public_key);
    }

    Ok(Json(LoginResponse {
        authenticated: true,
        public_key: payload.public_key,
    }))
}

pub async fn get_blockchain_ballot(Path(decade_id): Path<u8>) -> Json<BlockchainBallotResponse> {
    let result =
        tokio::task::spawn_blocking(move || get_ballot_state_from_blockchain(decade_id)).await;

    match result {
        Ok(Ok(response)) => Json(response),

        Ok(Err(error)) => Json(BlockchainBallotResponse {
            success: false,
            decade_id,
            ballot: String::new(),
            merkle_root: String::new(),
            total_votes: 0,
            batch_count: 0,
            encrypted_tally: Vec::new(),
            status: error,
        }),

        Err(_) => Json(BlockchainBallotResponse {
            success: false,
            decade_id,
            ballot: String::new(),
            merkle_root: String::new(),
            total_votes: 0,
            batch_count: 0,
            encrypted_tally: Vec::new(),
            status: "Failed to run blockchain fetch task".to_string(),
        }),
    }
}

// Converte decade_id para texto.
fn decade_label(decade_id: u8) -> &'static str {
    match decade_id {
        0 => "1970",
        1 => "1980",
        2 => "1990",
        3 => "2000",
        4 => "2010",
        _ => "2020",
    }
}
