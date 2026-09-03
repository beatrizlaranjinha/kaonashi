use crate::models::PendingEncryptedVote;

/// Returns the canonical byte representation of one accepted encrypted vote.
///
/// This function is the single source of truth for the byte ordering used when
/// the rollup commits to a vote. Both the Merkle leaf construction and the
/// Groth16 batch commitment must start from these exact bytes.
///
/// Current layout:
///
/// wallet_id
/// || public_key
/// || decade_id
/// || encrypted_vote_hash
/// || ciphertext_0
/// || ciphertext_1
/// || ...
///
/// Keeping this encoding in one place prevents the Merkle and Groth16 paths
/// from silently diverging if the vote representation changes later.
pub fn canonical_vote_bytes(vote: &PendingEncryptedVote) -> Vec<u8> {
    let ciphertext_bytes = vote.encrypted_vote.len() * 64;

    let mut bytes = Vec::with_capacity(
        vote.wallet_id.len()
            + vote.public_key.len()
            + 1
            + vote.encrypted_vote_hash.len()
            + ciphertext_bytes,
    );

    bytes.extend_from_slice(vote.wallet_id.as_bytes());
    bytes.extend_from_slice(vote.public_key.as_bytes());
    bytes.push(vote.decade_id);
    bytes.extend_from_slice(vote.encrypted_vote_hash.as_bytes());

    for ciphertext in &vote.encrypted_vote {
        bytes.extend_from_slice(ciphertext);
    }

    bytes
}
