use zk_client::{
    crypto::validate_vote_vector,
    rollups::{aggregate_votes, merkle_root, sha256},
};

#[test]
fn accepts_valid_vote_vector() {
    assert!(validate_vote_vector(&[0, 1, 0]).is_ok());
}

#[test]
fn rejects_empty_vote_vector() {
    assert!(validate_vote_vector(&[]).is_err());
}

#[test]
fn rejects_non_binary_vote_vector() {
    assert!(validate_vote_vector(&[0, 2, 0]).is_err());
}

#[test]
fn rejects_vote_vector_with_more_than_one_vote() {
    assert!(validate_vote_vector(&[1, 1, 0]).is_err());
}

#[test]
fn rejects_vote_vector_without_a_vote() {
    assert!(validate_vote_vector(&[0, 0, 0]).is_err());
}

#[test]
fn aggregates_batch_votes() {
    let votes = vec![
        vec![0, 1, 0],
        vec![1, 0, 0],
        vec![0, 1, 0],
    ];

    let tally = aggregate_votes(&votes, 3).unwrap();
    assert_eq!(tally, vec![1, 2, 0]);
}

#[test]
fn rejects_votes_with_incorrect_size() {
    let votes = vec![vec![0, 1, 0], vec![1, 0]];
    assert!(aggregate_votes(&votes, 3).is_err());
}

#[test]
fn empty_merkle_tree_has_zero_root() {
    assert_eq!(merkle_root(vec![]), [0u8; 32]);
}

#[test]
fn one_leaf_is_the_merkle_root() {
    let leaf = sha256(b"vote");
    assert_eq!(merkle_root(vec![leaf]), leaf);
}
