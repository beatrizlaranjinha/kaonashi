use solana_sdk::hash::{hashv, Hash};
use std::str::FromStr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MerkleProofNode {
    pub hash: String,
    pub is_left: bool,
}

// -----------------------------------------------------------------------------
// Internal hash helpers
// -----------------------------------------------------------------------------

fn parse_hash(value: &str) -> Result<Hash, String> {
    Hash::from_str(value).map_err(|error| format!("Invalid Merkle hash '{}': {}", value, error))
}

// Internal Merkle nodes are hashed from the raw 32-byte child hashes.
// Base58 is only used for transport/display in the API and receipts.
fn hash_pair_raw(left: &Hash, right: &Hash) -> Hash {
    let left_bytes = left.to_bytes();
    let right_bytes = right.to_bytes();

    hashv(&[b"kaonashi-node", &left_bytes, &right_bytes])
}

// -----------------------------------------------------------------------------
// Public Merkle API
// -----------------------------------------------------------------------------

// Creates the hash of one Merkle leaf.
pub fn hash_leaf(data: &[u8]) -> String {
    hashv(&[b"kaonashi-leaf", data]).to_string()
}

// Calculates the Merkle root from leaf hashes.
pub fn merkle_root(leaves: &[String]) -> Result<String, String> {
    if leaves.is_empty() {
        return Err("Cannot build Merkle root from empty leaves".to_string());
    }

    let mut level = leaves
        .iter()
        .map(|leaf| parse_hash(leaf))
        .collect::<Result<Vec<Hash>, String>>()?;

    while level.len() > 1 {
        let mut next = Vec::with_capacity((level.len() + 1) / 2);

        for pair in level.chunks(2) {
            let left = &pair[0];
            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };

            next.push(hash_pair_raw(left, right));
        }

        level = next;
    }

    Ok(level[0].to_string())
}

// Generates the Merkle inclusion proof for the leaf at `index`.
pub fn merkle_proof(leaves: &[String], mut index: usize) -> Result<Vec<MerkleProofNode>, String> {
    if leaves.is_empty() {
        return Err("Cannot build Merkle proof from empty leaves".to_string());
    }

    if index >= leaves.len() {
        return Err("Leaf index out of bounds".to_string());
    }

    let mut proof = Vec::new();

    let mut level = leaves
        .iter()
        .map(|leaf| parse_hash(leaf))
        .collect::<Result<Vec<Hash>, String>>()?;

    while level.len() > 1 {
        let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };

        let sibling_hash = if sibling_index < level.len() {
            &level[sibling_index]
        } else {
            &level[index]
        };

        proof.push(MerkleProofNode {
            hash: sibling_hash.to_string(),
            is_left: index % 2 == 1,
        });

        let mut next = Vec::with_capacity((level.len() + 1) / 2);

        for pair in level.chunks(2) {
            let left = &pair[0];
            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };

            next.push(hash_pair_raw(left, right));
        }

        index /= 2;
        level = next;
    }

    Ok(proof)
}

// Verifies that a leaf belongs to the supplied Merkle root.
pub fn verify_merkle_proof(leaf: &str, proof: &[MerkleProofNode], root: &str) -> bool {
    let mut current = match parse_hash(leaf) {
        Ok(hash) => hash,
        Err(_) => return false,
    };

    let expected_root = match parse_hash(root) {
        Ok(hash) => hash,
        Err(_) => return false,
    };

    for node in proof {
        let sibling = match parse_hash(&node.hash) {
            Ok(hash) => hash,
            Err(_) => return false,
        };

        current = if node.is_left {
            hash_pair_raw(&sibling, &current)
        } else {
            hash_pair_raw(&current, &sibling)
        };
    }

    current == expected_root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_proofs_verify_for_even_number_of_leaves() {
        let leaves = vec![
            hash_leaf(b"vote-0"),
            hash_leaf(b"vote-1"),
            hash_leaf(b"vote-2"),
            hash_leaf(b"vote-3"),
        ];

        let root = merkle_root(&leaves).expect("root");

        for index in 0..leaves.len() {
            let proof = merkle_proof(&leaves, index).expect("proof");

            assert!(verify_merkle_proof(&leaves[index], &proof, &root));
        }
    }

    #[test]
    fn merkle_proofs_verify_for_odd_number_of_leaves() {
        let leaves = vec![
            hash_leaf(b"vote-0"),
            hash_leaf(b"vote-1"),
            hash_leaf(b"vote-2"),
        ];

        let root = merkle_root(&leaves).expect("root");

        for index in 0..leaves.len() {
            let proof = merkle_proof(&leaves, index).expect("proof");

            assert!(verify_merkle_proof(&leaves[index], &proof, &root));
        }
    }

    #[test]
    fn modified_leaf_is_rejected() {
        let leaves = vec![hash_leaf(b"vote-0"), hash_leaf(b"vote-1")];

        let root = merkle_root(&leaves).expect("root");
        let proof = merkle_proof(&leaves, 0).expect("proof");

        let modified_leaf = hash_leaf(b"modified-vote");

        assert!(!verify_merkle_proof(&modified_leaf, &proof, &root));
    }
}
