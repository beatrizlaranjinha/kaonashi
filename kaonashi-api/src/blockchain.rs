use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

use reqwest::blocking::Client as HttpClient;
use serde_json::{json, Value};
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_zk_sdk::encryption::elgamal::{ElGamalPubkey, ElGamalSecretKey};

use crate::models::{BlockchainBallotResponse, FinalResultsResponse};
use crate::movies::movies_decades;

use zk_client::crypto::{decrypt_tally, encrypt_values};

use zk_client::solana_client::{
    close_election, connect_localnet, fetch_ballot, initialize_ballot, set_final_winner,
    submit_rollup_batch,
};

const LOCALNET_RPC_URL: &str = "http://127.0.0.1:8899";
const ONCHAIN_BENCHMARK_CSV: &str = "data/benchmarks/onchain.csv";

// ============================================================================
// Rollup batches
// ============================================================================

// Sends one encrypted batch tally and its Groth16 proof to the Solana smart
// contract.
//
// When KAONASHI_ONCHAIN_BENCHMARK=1, this function also records:
//
// - send-to-confirmed latency
// - compute units consumed
// - transaction fee
// - votes per second
// - compute units per vote
// - fee per vote
//
// The benchmark metrics are appended to:
// data/benchmarks/onchain.csv
pub fn submit_rollup_batch_to_blockchain(
    ballot: Pubkey,
    decade_id: u8,
    merkle_root: &str,
    encrypted_batch_tally: Vec<[u8; 64]>,
    batch_size: usize,
    proof: [u8; 256],
    public_inputs: [[u8; 32]; 2],
) -> Result<Signature, String> {
    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    let merkle_root_hash =
        Hash::from_str(merkle_root).map_err(|error| format!("Invalid Merkle root: {}", error))?;

    let benchmark_enabled = env::var("KAONASHI_ONCHAIN_BENCHMARK").as_deref() == Ok("1");

    let repetitions = if benchmark_enabled {
        env::var("KAONASHI_ONCHAIN_REPETITIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1)
    } else {
        1
    };

    if benchmark_enabled && repetitions > 1 {
        println!(
            "ON-CHAIN BENCHMARK MODE: submitting the same valid rollup payload {} times",
            repetitions
        );
        println!(
            "NOTE: this intentionally mutates the benchmark ballot {} times; use a dedicated local-validator run.",
            repetitions
        );
    }

    let mut last_signature: Option<Signature> = None;

    for run_index in 1..=repetitions {
        // Anchor's blocking .send() returns only after the transaction reaches
        // the configured confirmed commitment. We therefore call this metric
        // "send-to-confirmed latency" rather than claiming it is the isolated
        // execution time of Groth16 verification.
        let submission_start = Instant::now();

        let signature = submit_rollup_batch(
            &program,
            ballot,
            merkle_root_hash.to_bytes(),
            encrypted_batch_tally.clone(),
            batch_size as u64,
            proof,
            public_inputs,
        )
        .map_err(|error| {
            format!(
                "Failed to submit rollup batch on benchmark run {}/{}: {}",
                run_index, repetitions, error
            )
        })?;

        let confirmed_latency = submission_start.elapsed();

        println!(
            "Submitted Groth16-verified rollup batch on-chain for decade {}. \
             Ballot: {}. Batch size: {}. Run: {}/{}. Signature: {}",
            decade_id, ballot, batch_size, run_index, repetitions, signature
        );

        println!(
            "On-chain send-to-confirmed latency: {:.3} ms",
            confirmed_latency.as_secs_f64() * 1000.0
        );

        if benchmark_enabled {
            match collect_and_store_onchain_metrics(
                &signature,
                batch_size,
                run_index,
                confirmed_latency,
            ) {
                Ok(()) => {}
                Err(error) => {
                    // Benchmark collection must never make an otherwise valid
                    // blockchain submission appear to have failed.
                    eprintln!("ON-CHAIN BENCHMARK WARNING: {}", error);
                }
            }
        }

        last_signature = Some(signature);
    }

    last_signature.ok_or_else(|| "No rollup transaction was submitted".to_string())
}

// ============================================================================
// On-chain benchmark collection
// ============================================================================

fn collect_and_store_onchain_metrics(
    signature: &Signature,
    batch_size: usize,
    run_index: usize,
    confirmed_latency: Duration,
) -> Result<(), String> {
    let (compute_units, fee_lamports) = fetch_transaction_metrics(signature)?;

    let confirmed_latency_ms = confirmed_latency.as_secs_f64() * 1000.0;
    let confirmed_latency_s = confirmed_latency.as_secs_f64();

    let throughput_votes_s = if confirmed_latency_s > 0.0 {
        batch_size as f64 / confirmed_latency_s
    } else {
        0.0
    };

    let cu_per_vote = compute_units as f64 / batch_size as f64;
    let fee_per_vote_lamports = fee_lamports as f64 / batch_size as f64;

    println!("ON-CHAIN BENCHMARK");
    println!("  Batch size: {}", batch_size);
    println!("  Run index: {}", run_index);
    println!(
        "  Send-to-confirmed latency: {:.3} ms",
        confirmed_latency_ms
    );
    println!("  Compute units: {}", compute_units);
    println!("  Fee: {} lamports", fee_lamports);
    println!("  Throughput: {:.6} votes/s", throughput_votes_s);
    println!("  Compute units per vote: {:.3}", cu_per_vote);
    println!("  Fee per vote: {:.3} lamports", fee_per_vote_lamports);
    println!("  Signature: {}", signature);

    append_onchain_csv(
        signature,
        batch_size,
        run_index,
        confirmed_latency_ms,
        compute_units,
        fee_lamports,
        throughput_votes_s,
        cu_per_vote,
        fee_per_vote_lamports,
    )
}

fn fetch_transaction_metrics(signature: &Signature) -> Result<(u64, u64), String> {
    let client = HttpClient::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| format!("Failed to create benchmark RPC client: {}", error))?;

    // .send() already waits for confirmed commitment, but the transaction may
    // still take a short moment to become available through getTransaction.
    for attempt in 0..20 {
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                signature.to_string(),
                {
                    "encoding": "json",
                    "commitment": "confirmed",
                    "maxSupportedTransactionVersion": 0
                }
            ]
        });

        let response = client
            .post(LOCALNET_RPC_URL)
            .json(&request_body)
            .send()
            .map_err(|error| format!("getTransaction RPC request failed: {}", error))?
            .error_for_status()
            .map_err(|error| format!("getTransaction RPC returned HTTP error: {}", error))?
            .json::<Value>()
            .map_err(|error| format!("Failed to decode getTransaction RPC response: {}", error))?;

        if let Some(rpc_error) = response.get("error") {
            return Err(format!("getTransaction RPC error: {}", rpc_error));
        }

        if let Some(result) = response.get("result") {
            if !result.is_null() {
                let meta = result
                    .get("meta")
                    .ok_or_else(|| "getTransaction result has no meta field".to_string())?;

                let fee_lamports = meta
                    .get("fee")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "Transaction metadata has no fee".to_string())?;

                let compute_units = meta
                    .get("computeUnitsConsumed")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        "Transaction metadata has no computeUnitsConsumed field".to_string()
                    })?;

                return Ok((compute_units, fee_lamports));
            }
        }

        if attempt < 19 {
            thread::sleep(Duration::from_millis(250));
        }
    }

    Err(format!(
        "Transaction {} was confirmed but its metadata was not available after retries",
        signature
    ))
}

#[allow(clippy::too_many_arguments)]
fn append_onchain_csv(
    signature: &Signature,
    batch_size: usize,
    run_index: usize,
    confirmed_latency_ms: f64,
    compute_units: u64,
    fee_lamports: u64,
    throughput_votes_s: f64,
    cu_per_vote: f64,
    fee_per_vote_lamports: f64,
) -> Result<(), String> {
    fs::create_dir_all("data/benchmarks")
        .map_err(|error| format!("Failed to create benchmark directory: {}", error))?;

    let csv_path = Path::new(ONCHAIN_BENCHMARK_CSV);

    let write_header = match fs::metadata(csv_path) {
        Ok(metadata) => metadata.len() == 0,
        Err(_) => true,
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(csv_path)
        .map_err(|error| format!("Failed to open {}: {}", ONCHAIN_BENCHMARK_CSV, error))?;

    if write_header {
        writeln!(
            file,
            "batch_size,run_index,send_to_confirmed_latency_ms,compute_units,fee_lamports,throughput_votes_s,cu_per_vote,fee_per_vote_lamports,signature"
        )
        .map_err(|error| format!("Failed to write benchmark CSV header: {}", error))?;
    }

    writeln!(
        file,
        "{},{},{:.6},{},{},{:.9},{:.6},{:.6},{}",
        batch_size,
        run_index,
        confirmed_latency_ms,
        compute_units,
        fee_lamports,
        throughput_votes_s,
        cu_per_vote,
        fee_per_vote_lamports,
        signature
    )
    .map_err(|error| format!("Failed to append benchmark CSV row: {}", error))?;

    Ok(())
}

// ============================================================================
// Ballot creation
// ============================================================================

// Creates one on-chain ballot for each decade.
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

        // The encrypted tally starts with zero votes for every movie.
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

// ============================================================================
// Ballot closing
// ============================================================================

// Closes one on-chain ballot.
pub fn close_ballot_on_chain(ballot: Pubkey, decade_id: u8) -> Result<(), String> {
    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    match close_election(&program, ballot) {
        Ok(_) => {
            println!("Election closed on-chain for decade {}", decade_id);
            Ok(())
        }

        Err(error) => {
            let error_text = error.to_string();

            // If the ballot is already closed, we treat it as a successful close.
            if error_text.contains("ElectionNotOpen") || error_text.contains("Election is not open")
            {
                println!(
                    "Election for decade {} was already closed on-chain",
                    decade_id
                );

                Ok(())
            } else {
                Err(format!(
                    "Failed to close election on-chain for decade {}: {}",
                    decade_id, error
                ))
            }
        }
    }
}

// ============================================================================
// Ballot state
// ============================================================================

// Fetches the current on-chain state of one ballot.
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

// ============================================================================
// Election finalization
// ============================================================================

// Decrypts the on-chain encrypted tally and sets the final winner.
pub fn finalize_election_from_blockchain(
    ballot: Pubkey,
    decade_id: u8,
    secret_key: ElGamalSecretKey,
    resolved_winner_index: Option<usize>,
) -> Result<FinalResultsResponse, String> {
    let movies =
        movies_decades(decade_id).ok_or_else(|| format!("Invalid decade {}", decade_id))?;

    let program = connect_localnet()
        .map_err(|error| format!("Failed to connect to Solana localnet: {}", error))?;

    let ballot_account = fetch_ballot(&program, ballot)
        .map_err(|error| format!("Failed to fetch ballot: {}", error))?;

    let results = decrypt_tally(&ballot_account.encrypted_tally, &secret_key)
        .map_err(|error| format!("Failed to decrypt tally: {}", error))?;

    let decrypted_total_votes: u32 = results.iter().sum();

    if decrypted_total_votes == 0 {
        println!(
            "Election has no votes for decade {}. Results: {:?}",
            decade_id, results
        );

        return Ok(FinalResultsResponse {
            success: false,
            decade_id,
            results,
            winner_index: 0,
            winner_movie: String::new(),
            total_votes: ballot_account.total_votes,
            batch_count: ballot_account.batch_count,
            status: "NoVotes".to_string(),
        });
    }

    let max_votes = results.iter().copied().max().unwrap_or(0);

    let tie_indices = results
        .iter()
        .enumerate()
        .filter_map(|(index, votes)| {
            if *votes == max_votes {
                Some(index)
            } else {
                None
            }
        })
        .collect::<Vec<usize>>();

    let final_winner_index = if tie_indices.len() > 1 {
        match resolved_winner_index {
            Some(index) if tie_indices.contains(&index) => index,

            Some(index) => {
                return Ok(FinalResultsResponse {
                    success: false,
                    decade_id,
                    results,
                    winner_index: index,
                    winner_movie: String::new(),
                    total_votes: ballot_account.total_votes,
                    batch_count: ballot_account.batch_count,
                    status: format!(
                        "Resolved winner index {} is not part of the tie {:?}",
                        index, tie_indices
                    ),
                });
            }

            None => {
                println!(
                    "Tie detected for decade {}. Tied indices: {:?}. Results: {:?}",
                    decade_id, tie_indices, results
                );

                return Ok(FinalResultsResponse {
                    success: false,
                    decade_id,
                    results,
                    winner_index: 0,
                    winner_movie: String::new(),
                    total_votes: ballot_account.total_votes,
                    batch_count: ballot_account.batch_count,
                    status: "Tie".to_string(),
                });
            }
        }
    } else {
        tie_indices[0]
    };

    let winner_movie = movies
        .get(final_winner_index)
        .ok_or_else(|| "Winner index does not match movie list".to_string())?
        .clone();

    set_final_winner(&program, ballot, final_winner_index as u8)
        .map_err(|error| format!("Failed to set final winner on-chain: {}", error))?;

    println!(
        "Election finalized for decade {}. Winner: {}. Results: {:?}",
        decade_id, winner_movie, results
    );

    Ok(FinalResultsResponse {
        success: true,
        decade_id,
        results,
        winner_index: final_winner_index,
        winner_movie,
        total_votes: ballot_account.total_votes,
        batch_count: ballot_account.batch_count,
        status: "Finalized".to_string(),
    })
}
