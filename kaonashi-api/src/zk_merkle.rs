use ark_bn254::Fr;
use ark_crypto_primitives::crh::sha256::constraints::{DigestVar, Sha256Gadget};
use ark_r1cs_std::{alloc::AllocVar, uint8::UInt8};
use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError};

/// Computes the Kaonashi Merkle root inside an R1CS circuit from private
/// 32-byte Merkle leaf hashes.
///
/// The leaf hashes are assumed to already be:
///
///     SHA256("kaonashi-leaf" || canonical_vote_bytes)
///
/// This gadget reproduces the internal-node rule used by `src/merkle.rs`:
///
///     parent = SHA256("kaonashi-node" || left[32] || right[32])
///
/// If a level has an odd number of nodes, the final node is duplicated.
pub fn merkle_root_from_leaf_hashes(
    cs: ConstraintSystemRef<Fr>,
    leaves: &[Option<[u8; 32]>],
) -> Result<DigestVar<Fr>, SynthesisError> {
    assert!(
        !leaves.is_empty(),
        "Cannot build an in-circuit Merkle root from zero leaves"
    );

    // ------------------------------------------------------------------------
    // Allocate each 32-byte leaf as a private witness.
    // ------------------------------------------------------------------------

    let mut level = leaves
        .iter()
        .map(|leaf| {
            let bytes = (0..32)
                .map(|index| {
                    UInt8::new_witness(cs.clone(), || {
                        leaf.map(|value| value[index])
                            .ok_or(SynthesisError::AssignmentMissing)
                    })
                })
                .collect::<Result<Vec<_>, SynthesisError>>()?;

            Ok(DigestVar(bytes))
        })
        .collect::<Result<Vec<DigestVar<Fr>>, SynthesisError>>()?;

    // ------------------------------------------------------------------------
    // Build the same Merkle tree as src/merkle.rs, but inside R1CS.
    // ------------------------------------------------------------------------

    while level.len() > 1 {
        let mut next = Vec::with_capacity((level.len() + 1) / 2);

        for pair in level.chunks(2) {
            let left = &pair[0];

            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };

            // Exact byte input used by the off-chain Merkle implementation:
            //
            // "kaonashi-node" || left_raw_hash || right_raw_hash
            let mut input = b"kaonashi-node"
                .iter()
                .copied()
                .map(UInt8::constant)
                .collect::<Vec<_>>();

            input.extend(left.0.iter().cloned());
            input.extend(right.0.iter().cloned());

            let parent = Sha256Gadget::<Fr>::digest(&input)?;

            next.push(parent);
        }

        level = next;
    }

    Ok(level.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::merkle::{hash_leaf, merkle_root};

    use ark_r1cs_std::R1CSVar;
    use ark_relations::r1cs::ConstraintSystem;

    use solana_sdk::hash::Hash;

    use std::str::FromStr;

    fn raw_hash(value: &str) -> [u8; 32] {
        Hash::from_str(value).expect("valid Solana hash").to_bytes()
    }

    #[test]
    fn circuit_merkle_root_matches_off_chain_even_tree() {
        let leaves = vec![
            hash_leaf(b"vote-0"),
            hash_leaf(b"vote-1"),
            hash_leaf(b"vote-2"),
            hash_leaf(b"vote-3"),
        ];

        let expected_root = merkle_root(&leaves).expect("off-chain root");

        let leaf_witnesses = leaves
            .iter()
            .map(|leaf| Some(raw_hash(leaf)))
            .collect::<Vec<_>>();

        let cs = ConstraintSystem::<Fr>::new_ref();

        let circuit_root =
            merkle_root_from_leaf_hashes(cs.clone(), &leaf_witnesses).expect("circuit root");

        assert_eq!(
            circuit_root.value().expect("circuit root value"),
            raw_hash(&expected_root)
        );

        assert!(cs.is_satisfied().expect("constraint-system status"));
    }

    #[test]
    fn circuit_merkle_root_matches_off_chain_odd_tree() {
        let leaves = vec![
            hash_leaf(b"vote-0"),
            hash_leaf(b"vote-1"),
            hash_leaf(b"vote-2"),
        ];

        let expected_root = merkle_root(&leaves).expect("off-chain root");

        let leaf_witnesses = leaves
            .iter()
            .map(|leaf| Some(raw_hash(leaf)))
            .collect::<Vec<_>>();

        let cs = ConstraintSystem::<Fr>::new_ref();

        let circuit_root =
            merkle_root_from_leaf_hashes(cs.clone(), &leaf_witnesses).expect("circuit root");

        assert_eq!(
            circuit_root.value().expect("circuit root value"),
            raw_hash(&expected_root)
        );

        assert!(cs.is_satisfied().expect("constraint-system status"));
    }
}
