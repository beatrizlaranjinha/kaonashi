use crate::blockchain::submit_rollup_batch_to_blockchain;
use crate::groth16::{generate_batch_proof, SUPPORTED_GROTH16_BATCH_SIZES};
use crate::keeping_votes::KeepingVotes;
use crate::merkle::{hash_leaf, merkle_proof, merkle_root};
use crate::models::{
    EncryptedVoteBatch, FlushBatchResponse, MerkleProofNodeResponse, PendingEncryptedVote,
    VoteReceipt,
};
use crate::vote_encoding::canonical_vote_bytes;

use solana_sdk::hash::{hashv, Hash};
use solana_zk_sdk::encryption::elgamal::ElGamalCiphertext;

use std::{
    env,
    str::FromStr,
    sync::OnceLock,
    time::{Duration, Instant},
};

static CONFIGURED_BATCH_SIZE: OnceLock<usize> = OnceLock::new();

pub fn configured_batch_size() -> usize {
    *CONFIGURED_BATCH_SIZE.get_or_init(|| {
        let raw = env::var("KAONASHI_BATCH_SIZE").unwrap_or_else(|_| "10".to_string());

        let batch_size = raw.parse::<usize>().unwrap_or_else(|_| {
            panic!("Invalid KAONASHI_BATCH_SIZE '{}': expected an integer", raw)
        });

        if !SUPPORTED_GROTH16_BATCH_SIZES.contains(&batch_size) {
            panic!(
                "Unsupported KAONASHI_BATCH_SIZE {}. Supported sizes: {:?}",
                batch_size, SUPPORTED_GROTH16_BATCH_SIZES
            );
        }

        println!("Kaonashi configured batch size: {}", batch_size);

        batch_size
    })
}

// ============================================================================
// Create batch
// ============================================================================

// Creates a batch for one decade if there are enough pending encrypted votes.
pub fn create_batch_for_decade(
    keeping_votes: &KeepingVotes,
    decade_id: u8,
) -> Result<Option<FlushBatchResponse>, String> {
    // ------------------------------------------------------------------------
    // Get pending votes
    // ------------------------------------------------------------------------

    let mut pending_votes = keeping_votes.pending_encrypted_votes.lock().unwrap();

    if pending_votes[decade_id as usize].is_empty() {
        return Ok(None);
    }

    let target_batch_size = configured_batch_size();
    let available_votes = pending_votes[decade_id as usize].len();

    if available_votes < target_batch_size {
        return Err(format!(
            "Not enough pending votes to create a Groth16 batch: have {}, need {}",
            available_votes, target_batch_size
        ));
    }

    let votes = pending_votes[decade_id as usize]
        .drain(0..target_batch_size)
        .collect::<Vec<PendingEncryptedVote>>();

    drop(pending_votes);

    // ------------------------------------------------------------------------
    // Create Merkle leaves
    // ------------------------------------------------------------------------

    let leaves = votes.iter().map(batch_vote_leaf).collect::<Vec<String>>();

    // ------------------------------------------------------------------------
    // Create encrypted homomorphic batch tally
    // ------------------------------------------------------------------------

    let encrypted_batch_tally = create_encrypted_batch_tally(&votes)?;

    // ------------------------------------------------------------------------
    // Build Merkle tree
    // ------------------------------------------------------------------------

    let tree_start = Instant::now();

    let merkle_root = merkle_root(&leaves)?;

    let tree_build_time = tree_start.elapsed();

    println!("Merkle tree build: {:?}", tree_build_time);

    // ============================================================================
    // Groth16 batch proof
    // ============================================================================

    println!(
        "Generating Groth16 proof for batch of {} votes...",
        votes.len()
    );

    let zk_result = generate_batch_proof(&leaves, &merkle_root)?;

    println!("Groth16 batch proof generated successfully");
    println!("  Proving key load: {:?}", zk_result.proving_key_load_time);
    println!("  Proof generation: {:?}", zk_result.proof_generation_time);
    println!(
        "  Local verification: {:?}",
        zk_result.local_verification_time
    );

    // These are the exact representations expected by the Solana instruction.
    //
    // proof:
    //     -A = 64 bytes
    //      B = 128 bytes
    //      C = 64 bytes
    //     total = 256 bytes
    //
    // public inputs:
    //     [0] = high 128 bits of the exact Merkle root
    //     [1] = low 128 bits of the exact Merkle root

    let groth16_proof = zk_result.proof_bytes;
    let groth16_public_inputs = zk_result.public_inputs_bytes;

    // ============================================================================
    // Create batch identifier
    // ============================================================================

    let batch_index = {
        let batches = keeping_votes.encrypted_vote_batches.lock().unwrap();
        batches[decade_id as usize].len()
    };

    let decade_bytes = [decade_id];
    let batch_index_text = batch_index.to_string();
    let vote_count_text = votes.len().to_string();

    let batch_id = hashv(&[
        b"kaonashi-batch",
        &decade_bytes,
        batch_index_text.as_bytes(),
        merkle_root.as_bytes(),
        vote_count_text.as_bytes(),
    ])
    .to_string();

    // ============================================================================
    // Generate Merkle inclusion proofs / receipts
    // ============================================================================

    let mut receipts = Vec::new();
    let mut total_proof_time = Duration::ZERO;

    for (index, vote) in votes.iter().enumerate() {
        let start = Instant::now();

        let proof = merkle_proof(&leaves, index)?;

        total_proof_time += start.elapsed();

        receipts.push(VoteReceipt {
            vote_hash: vote.encrypted_vote_hash.clone(),
            leaf_hash: leaves[index].clone(),
            batch_id: batch_id.clone(),
            decade_id,
            leaf_index: index,
            merkle_root: merkle_root.clone(),
            merkle_proof: proof
                .into_iter()
                .map(|node| MerkleProofNodeResponse {
                    hash: node.hash,
                    is_left: node.is_left,
                })
                .collect(),
        });
    }

    println!(
        "Average Merkle proof generation: {:?}",
        total_proof_time / receipts.len() as u32
    );

    // ============================================================================
    // Verify generated Merkle proofs locally
    // ============================================================================

    let mut total_verification_time = Duration::ZERO;

    for receipt in &receipts {
        let proof = receipt
            .merkle_proof
            .iter()
            .map(|node| crate::merkle::MerkleProofNode {
                hash: node.hash.clone(),
                is_left: node.is_left,
            })
            .collect::<Vec<_>>();

        let start = Instant::now();

        let verified =
            crate::merkle::verify_merkle_proof(&receipt.leaf_hash, &proof, &receipt.merkle_root);

        total_verification_time += start.elapsed();

        assert!(verified);
    }

    println!(
        "Average Merkle proof verification: {:?}",
        total_verification_time / receipts.len() as u32
    );

    // ============================================================================
    // Store batch and receipts in API memory
    // ============================================================================

    let batch = EncryptedVoteBatch {
        batch_id: batch_id.clone(),
        decade_id,
        merkle_root: merkle_root.clone(),
        vote_count: votes.len(),
        encrypted_batch_tally: encrypted_batch_tally.clone(),
        votes,
    };

    {
        let mut batches = keeping_votes.encrypted_vote_batches.lock().unwrap();
        batches[decade_id as usize].push(batch);
    }

    {
        let mut stored_receipts = keeping_votes.vote_receipts_by_hash.lock().unwrap();

        for receipt in &receipts {
            stored_receipts.insert(receipt.vote_hash.clone(), receipt.clone());
        }
    }

    // ============================================================================
    // Find corresponding on-chain ballot
    // ============================================================================

    let ballot_for_chain = {
        let ballots = keeping_votes.ballots_by_decade.lock().unwrap();

        ballots
            .get(decade_id as usize)
            .and_then(|ballot| ballot.as_ref())
            .cloned()
            .ok_or_else(|| {
                "No on-chain ballot found in API memory. \
                 Run /api/admin/create-ballots before submitting batches."
                    .to_string()
            })?
    };

    // ============================================================================
    // Prepare values for Solana submission
    // ============================================================================

    let decade_id_for_chain = decade_id;
    let mut merkle_root_for_chain = merkle_root.clone();
    let encrypted_tally_for_chain = encrypted_batch_tally.clone();
    let batch_size_for_chain = receipts.len();

    let mut proof_for_chain = groth16_proof;
    let public_inputs_for_chain = groth16_public_inputs;

    // Test-only switch used to demonstrate that the on-chain verifier rejects
    // a modified Merkle root while the Groth16 proof/public inputs remain valid
    // for the original root.
    //
    // Leave this environment variable unset during normal execution and
    // benchmarks.
    if env::var("KAONASHI_CORRUPT_MERKLE_ROOT").as_deref() == Ok("1") {
        let parsed_root = Hash::from_str(&merkle_root_for_chain)
            .map_err(|error| format!("Invalid Merkle root before corruption test: {}", error))?;

        let mut root_bytes = parsed_root.to_bytes();
        root_bytes[31] ^= 0x01;

        merkle_root_for_chain = Hash::new_from_array(root_bytes).to_string();

        println!("TEST: Merkle root intentionally corrupted before on-chain submission");
        println!("  Original Merkle root: {}", merkle_root);
        println!("  Corrupted Merkle root: {}", merkle_root_for_chain);
    }

    // Test-only switch used to demonstrate that the on-chain verifier rejects
    // a modified Groth16 proof. Leave this environment variable unset during
    // normal execution and benchmarks.
    if env::var("KAONASHI_CORRUPT_GROTH16_PROOF").as_deref() == Ok("1") {
        proof_for_chain[255] ^= 0x01;
        println!("TEST: Groth16 proof intentionally corrupted before on-chain submission");
    }

    // ============================================================================
    // Submit Groth16-verified batch to Solana
    // ============================================================================

    let (on_chain_success, on_chain_status) = match std::thread::spawn(move || {
        submit_rollup_batch_to_blockchain(
            ballot_for_chain,
            decade_id_for_chain,
            &merkle_root_for_chain,
            encrypted_tally_for_chain,
            batch_size_for_chain,
            proof_for_chain,
            public_inputs_for_chain,
        )
    })
    .join()
    {
        Ok(Ok(_)) => (
            true,
            "Encrypted vote batch created and submitted on-chain".to_string(),
        ),

        Ok(Err(error)) => (
            false,
            format!(
                "Encrypted vote batch created off-chain, but on-chain submission failed: {}",
                error
            ),
        ),

        Err(_) => (
            false,
            "Encrypted vote batch created off-chain, but on-chain submission panicked".to_string(),
        ),
    };

    // ============================================================================
    // API response
    // ============================================================================

    Ok(Some(FlushBatchResponse {
        success: on_chain_success,
        decade_id,
        batch_id,
        merkle_root,
        vote_count: receipts.len(),
        encrypted_batch_tally: encrypted_batch_tally
            .iter()
            .map(|ciphertext| ciphertext.to_vec())
            .collect(),
        receipts,
        status: on_chain_status,
    }))
}

// ============================================================================
// Merkle leaf
// ============================================================================

// Creates the Merkle leaf corresponding to one encrypted vote.
pub fn batch_vote_leaf(vote: &PendingEncryptedVote) -> String {
    let data = canonical_vote_bytes(vote);
    hash_leaf(&data)
}

// ============================================================================
// Homomorphic encrypted batch tally
// ============================================================================

fn create_encrypted_batch_tally(votes: &[PendingEncryptedVote]) -> Result<Vec<[u8; 64]>, String> {
    if votes.is_empty() {
        return Err("Cannot create encrypted tally from empty batch".to_string());
    }

    let proposal_count = votes[0].encrypted_vote.len();

    if proposal_count == 0 {
        return Err("Encrypted vote has no proposals".to_string());
    }

    // ------------------------------------------------------------------------
    // Start tally with the first encrypted vote
    // ------------------------------------------------------------------------

    let mut tally = votes[0]
        .encrypted_vote
        .iter()
        .enumerate()
        .map(|(index, ciphertext_bytes)| {
            ElGamalCiphertext::from_bytes(ciphertext_bytes)
                .ok_or_else(|| format!("Invalid ciphertext at vote 0, proposal {}", index))
        })
        .collect::<Result<Vec<ElGamalCiphertext>, String>>()?;

    // ------------------------------------------------------------------------
    // Homomorphically add the remaining encrypted votes
    // ------------------------------------------------------------------------

    for (vote_index, vote) in votes.iter().enumerate().skip(1) {
        if vote.encrypted_vote.len() != proposal_count {
            return Err(format!(
                "Vote {} has {} ciphertexts, expected {}",
                vote_index,
                vote.encrypted_vote.len(),
                proposal_count
            ));
        }

        for (proposal_index, ciphertext_bytes) in vote.encrypted_vote.iter().enumerate() {
            let ciphertext = ElGamalCiphertext::from_bytes(ciphertext_bytes).ok_or_else(|| {
                format!(
                    "Invalid ciphertext at vote {}, proposal {}",
                    vote_index, proposal_index
                )
            })?;

            let current = tally[proposal_index];
            tally[proposal_index] = current + ciphertext;
        }
    }

    Ok(tally
        .into_iter()
        .map(|ciphertext| ciphertext.to_bytes())
        .collect())
}
