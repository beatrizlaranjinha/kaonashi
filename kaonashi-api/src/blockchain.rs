use crate::models::BlockchainBallotResponse;
use solana_sdk::hash::Hash;
use solana_sdk::signature::{Keypair, Signer};
use std::str::FromStr;

use crate::{ballots::ballot_for_decade, movies::movies_decades};

use zk_client::{
    crypto::{encrypt_values, generate_elgamal_keypair},
    solana_client::{connect_localnet, fetch_ballot, initialize_ballot, submit_rollup_batch},
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
    decade_id: u8,
    merkle_root: &str,
    encrypted_batch_tally: Vec<[u8; 64]>,
    batch_size: usize,
) -> Result<(), String> {
    // Vai buscar o endereço do ballot on-chain associado à década.
    let ballot = ballot_for_decade(decade_id)
        .ok_or_else(|| "No ballot found for this decade".to_string())?;

    // Liga à Solana localnet.
    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    // A Merkle root vem da API como string base58.
    // Aqui convertemos para Hash para obter os 32 bytes esperados pelo smart contract.
    let merkle_root_hash =
        Hash::from_str(merkle_root).map_err(|error| format!("Invalid Merkle root: {}", error))?;

    // Chama a instruction Anchor submit_rollup_batch.
    //
    // Esta instruction atualiza no Ballot:
    // - merkle_root;
    // - encrypted_tally global;
    // - total_votes;
    // - batch_count.
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
pub fn create_all_ballots_on_chain() -> Result<Vec<String>, String> {
    // Liga à Solana localnet.
    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    // Guarda as mensagens com os ballots criados.
    let mut created_ballots = Vec::new();

    // Cria uma eleição/ballot para cada década.
    for decade_id in 0..=5 {
        // Vai buscar os filmes da década.
        let movies =
            movies_decades(decade_id).ok_or_else(|| format!("Invalid decade {}", decade_id))?;

        // Cria uma nova conta Solana para o ballot.
        let ballot = Keypair::new();

        // Cria uma nova chave ElGamal para cifrar os votos desta década.
        let elgamal = generate_elgamal_keypair();

        // O tally inicial começa com zero votos para todos os filmes.
        let initial_values = vec![0_u64; movies.len()];

        // Cifra o tally inicial.
        let initial_encrypted_tally = encrypt_values(&initial_values, elgamal.pubkey());

        // Inicializa o ballot on-chain.
        initialize_ballot(
            &program,
            &ballot,
            movies,
            elgamal.pubkey().to_bytes(),
            initial_encrypted_tally,
        )
        .map_err(|error| {
            format!(
                "Failed to initialize ballot for decade {}: {}",
                decade_id, error
            )
        })?;

        // Guarda e imprime o endereço criado.
        let line = format!("decade {} -> ballot {}", decade_id, ballot.pubkey());

        println!("{}", line);

        created_ballots.push(line);
    }

    Ok(created_ballots)
}

pub fn get_ballot_state_from_blockchain(decade_id: u8) -> Result<BlockchainBallotResponse, String> {
    let ballot = ballot_for_decade(decade_id)
        .ok_or_else(|| "No ballot found for this decade".to_string())?;

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
