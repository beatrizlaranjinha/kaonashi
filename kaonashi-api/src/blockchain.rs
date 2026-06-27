use crate::models::BlockchainBallotResponse;
use crate::models::FinalResultsResponse;
use solana_sdk::hash::Hash;
use solana_sdk::signature::{Keypair, Signer};
use solana_zk_sdk::encryption::elgamal::ElGamalSecretKey;
use std::str::FromStr;
use zk_client::solana_client::set_final_winner;

use zk_client::crypto::{decrypt_tally, winner_index};

use crate::movies::movies_decades;
use solana_sdk::pubkey::Pubkey;
use solana_zk_sdk::encryption::elgamal::ElGamalPubkey;
use zk_client::{
    crypto::encrypt_values,
    solana_client::{
        close_election, connect_localnet, fetch_ballot, initialize_ballot, submit_rollup_batch,
    },
};

// ---------------------------------------------------
// Submit rollup batch to Solana
// ---------------------------------------------------
//
// Esta função é chamada pela API depois de criar um batch off-chain.
//
// Nesta fase, a API já fez:
// 1. verificação da assinatura;
// 2. verificação das ZK proofs;
// 3. criação do batch;
// 4. criação da Merkle root;
// 5. soma homomórfica dos votos cifrados.
//
// A blockchain NÃO recebe os votos individuais.
// Recebe apenas o resumo do batch:
// - merkle_root;
// - encrypted_batch_tally;
// - batch_size.
pub fn submit_rollup_batch_to_blockchain(
    ballot: Pubkey,
    decade_id: u8,
    merkle_root: &str,
    encrypted_batch_tally: Vec<[u8; 64]>,
    batch_size: usize,
) -> Result<(), String> {
    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    let merkle_root_hash =
        Hash::from_str(merkle_root).map_err(|error| format!("Invalid Merkle root: {}", error))?;

    submit_rollup_batch(
        &program,
        ballot,
        merkle_root_hash.to_bytes(),
        encrypted_batch_tally,
        batch_size as u64,
    )
    .map_err(|error| format!("Failed to submit rollup batch: {}", error))?;

    println!(
        "Submitted rollup batch on-chain for decade {}. Ballot: {}. Batch size: {}",
        decade_id, ballot, batch_size
    );

    Ok(())
}

// ---------------------------------------------------
// Create ballots on-chain
// ---------------------------------------------------
//
// Esta função cria ballots novos na blockchain.
// Cria 1 ballot por década, de 0 a 5.
//
// Nota:
// Depois de correr isto, é preciso copiar os endereços gerados
// para o ficheiro ballots.rs, para que a API saiba que ballot
// corresponde a cada década.

pub fn create_all_ballots_on_chain(
    elgamal_public_keys_by_decade: Vec<[u8; 32]>,
) -> Result<Vec<(u8, Pubkey)>, String> {
    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    let mut created_ballots = Vec::new();

    for decade_id in 0..=5 {
        let movies =
            movies_decades(decade_id).ok_or_else(|| format!("Invalid decade {}", decade_id))?;

        let public_key = elgamal_public_keys_by_decade
            .get(decade_id as usize)
            .ok_or_else(|| format!("Missing ElGamal public key for decade {}", decade_id))?;

        let elgamal_public_key = ElGamalPubkey::try_from(public_key.as_slice())
            .map_err(|_| format!("Invalid ElGamal public key for decade {}", decade_id))?;

        let ballot = Keypair::new();

        let initial_values = vec![0_u64; movies.len()];

        let initial_encrypted_tally = encrypt_values(&initial_values, &elgamal_public_key);

        initialize_ballot(
            &program,
            &ballot,
            movies,
            *public_key,
            initial_encrypted_tally,
        )
        .map_err(|error| {
            format!(
                "Failed to initialize ballot for decade {}: {}",
                decade_id, error
            )
        })?;

        println!("decade {} -> ballot {}", decade_id, ballot.pubkey());

        created_ballots.push((decade_id, ballot.pubkey()));
    }

    Ok(created_ballots)
}

pub fn get_ballot_state_from_blockchain(
    ballot: Pubkey,
    decade_id: u8,
) -> Result<BlockchainBallotResponse, String> {
    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    let ballot_account = fetch_ballot(&program, ballot)
        .map_err(|error| format!("Failed to fetch ballot: {}", error))?;

    Ok(BlockchainBallotResponse {
        success: true,
        decade_id,
        ballot: ballot.to_string(),
        merkle_root: bs58::encode(ballot_account.merkle_root).into_string(),
        total_votes: ballot_account.total_votes,
        batch_count: ballot_account.batch_count,
        encrypted_tally: ballot_account
            .encrypted_tally
            .iter()
            .map(|ciphertext| ciphertext.to_vec())
            .collect(),
        status: "Ballot fetched from blockchain".to_string(),
    })
}

pub fn finalize_election_from_blockchain(
    ballot: Pubkey,
    decade_id: u8,
    secret_key: ElGamalSecretKey,
) -> Result<FinalResultsResponse, String> {
    let movies =
        movies_decades(decade_id).ok_or_else(|| format!("Invalid decade {}", decade_id))?;

    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    let ballot_account = fetch_ballot(&program, ballot)
        .map_err(|error| format!("Failed to fetch ballot: {}", error))?;

    let results = decrypt_tally(&ballot_account.encrypted_tally, &secret_key)
        .map_err(|error| format!("Failed to decrypt tally: {}", error))?;

    let winner_index =
        winner_index(&results).map_err(|error| format!("Failed to determine winner: {}", error))?;

    let winner_movie = movies
        .get(winner_index)
        .ok_or_else(|| "Winner index does not match movie list".to_string())?
        .clone();
    match close_election(&program, ballot) {
        Ok(_) => {
            println!("Election closed on-chain");
        }

        Err(error) => {
            let error_text = error.to_string();

            if error_text.contains("ElectionNotOpen") || error_text.contains("Election is not open")
            {
                println!("Election was already closed, continuing finalize");
            } else {
                return Err(format!("Failed to close election on-chain: {}", error));
            }
        }
    }

    set_final_winner(&program, ballot, winner_index as u8)
        .map_err(|error| format!("Failed to set final winner on-chain: {}", error))?;

    Ok(FinalResultsResponse {
        success: true,
        decade_id,
        results,
        winner_index,
        winner_movie,
        total_votes: ballot_account.total_votes,
        batch_count: ballot_account.batch_count,
        status: "Election finalized successfully".to_string(),
    })
}
