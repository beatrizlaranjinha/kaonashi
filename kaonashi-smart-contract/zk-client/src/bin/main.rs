use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anyhow::{bail, Result};

use zk_client::{
    crypto::{
        create_vote_vector, decrypt_tally, encrypt_values, generate_elgamal_keypair, winner_index,
    },
    rollups::prepare_rollup_batch,
    solana_client::{
        connect_localnet, fetch_ballot, initialize_ballot, register_voter, set_final_winner,
        submit_rollup_batch,
    },
};

fn main() -> Result<()> {
    println!("Connecting to Solana localnet...");

    let program = connect_localnet()?;

    let ballot = Keypair::new();
    let voter = Keypair::new();

    println!("Ballot account: {}", ballot.pubkey());
    println!("Voter account: {}", voter.pubkey());

    // ---------------------------------------------------
    // ElGamal keys
    // ---------------------------------------------------

    let elgamal = generate_elgamal_keypair();
    let secret_key = elgamal.secret();
    let public_key = elgamal.pubkey();

    // ---------------------------------------------------
    // Movie proposals
    // 1990s ballot
    // ---------------------------------------------------

    let proposals = vec![
        "Fight Club".to_string(),
        "Pulp Fiction".to_string(),
        "Se7en".to_string(),
        "Goodfellas".to_string(),
        "Eyes Wide Shut".to_string(),
        "Casino".to_string(),
        "Fallen Angels".to_string(),
        "Reservoir Dogs".to_string(),
    ];

    println!("\nMovies in the ballot:");

    proposals.iter().enumerate().for_each(|(index, movie)| {
        println!("{}: {}", index, movie);
    });

    // ---------------------------------------------------
    // Initial encrypted tally
    // E(0) for each movie
    // ---------------------------------------------------

    let initial_values = vec![0u64; proposals.len()];

    let initial_encrypted_tally = encrypt_values(&initial_values, public_key);

    // ---------------------------------------------------
    // Initialize ballot on-chain
    // ---------------------------------------------------

    println!("\nInitializing ballot...");

    initialize_ballot(
        &program,
        &ballot,
        proposals.clone(),
        public_key.to_bytes(),
        initial_encrypted_tally,
    )?;

    println!("Ballot initialized successfully");

    // ---------------------------------------------------
    // Register voter
    // ---------------------------------------------------

    println!("\nRegistering voter...");

    let voter_record = register_voter(&program, ballot.pubkey(), voter.pubkey())?;

    println!("Voter registered successfully");
    println!("VoterRecord PDA: {}", voter_record);

    // ---------------------------------------------------
    // Example Layer 2 votes
    //
    // 0 = Fight Club
    // 1 = Pulp Fiction
    // 2 = Se7en
    // 3 = Goodfellas
    // 4 = Eyes Wide Shut
    // 5 = Casino
    // 6 = Fallen Angels
    // 7 = Reservoir Dogs
    // ---------------------------------------------------

    let plain_votes = vec![
        create_vote_vector(1, proposals.len())?, // Pulp Fiction
        create_vote_vector(0, proposals.len())?, // Fight Club
        create_vote_vector(1, proposals.len())?, // Pulp Fiction
        create_vote_vector(2, proposals.len())?, // Se7en
        create_vote_vector(1, proposals.len())?, // Pulp Fiction
    ];

    println!("\nPreparing rollup batch...");

    let batch = prepare_rollup_batch(&plain_votes, proposals.len(), public_key)?;

    // encrypted_votes are kept in the batch because they are used
    // to construct the Merkle Tree.
    println!("Encrypted votes generated: {}", batch.encrypted_votes.len());

    println!("Merkle root: {:?}", batch.merkle_root);
    println!("Batch size: {}", batch.batch_size);

    println!("\nPlain batch tally:");

    batch
        .plain_tally
        .iter()
        .enumerate()
        .for_each(|(index, votes)| {
            println!("{}: {}", proposals[index], votes);
        });

    // ---------------------------------------------------
    // Submit aggregated rollup batch on-chain
    // ---------------------------------------------------

    println!("\nSubmitting rollup batch...");

    submit_rollup_batch(
        &program,
        ballot.pubkey(),
        batch.merkle_root,
        batch.encrypted_tally,
        batch.batch_size,
    )?;

    println!("Rollup batch submitted successfully");

    // ---------------------------------------------------
    // Read ballot account
    // ---------------------------------------------------

    println!("\nFetching ballot account...");

    let ballot_account = fetch_ballot(&program, ballot.pubkey())?;

    println!("Rollup state:");
    println!("Total votes: {}", ballot_account.total_votes);
    println!("Batch count: {}", ballot_account.batch_count);
    println!("Merkle root: {:?}", ballot_account.merkle_root);

    // ---------------------------------------------------
    // Decrypt final tally off-chain
    // ---------------------------------------------------

    let decrypted_tally = decrypt_tally(&ballot_account.encrypted_tally, secret_key)?;

    println!("\nDecrypted tally:");

    decrypted_tally
        .iter()
        .enumerate()
        .for_each(|(index, votes)| {
            println!("{}: {}", proposals[index], votes);
        });

    // ---------------------------------------------------
    // Determine winner off-chain
    // ---------------------------------------------------

    let winner = winner_index(&decrypted_tally)?;

    println!(
        "\nWinning movie: {} with {} votes",
        proposals[winner], decrypted_tally[winner]
    );

    // ---------------------------------------------------
    // Store winner on-chain
    // ---------------------------------------------------

    set_final_winner(&program, ballot.pubkey(), winner as u8)?;

    println!("Winner saved on-chain: {}", proposals[winner]);

    Ok(())
}
