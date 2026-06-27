use solana_sdk::hash::hashv;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MerkleProofNode {
    pub hash: String,
    pub is_left: bool,
}

// Cria o hash de uma folha da Merkle tree.
pub fn hash_leaf(data: &[u8]) -> String {
    hashv(&[b"kaonashi-leaf", data]).to_string()
}

// Cria o hash de dois nós filhos.
pub fn hash_pair(left: &str, right: &str) -> String {
    hashv(&[b"kaonashi-node", left.as_bytes(), right.as_bytes()]).to_string()
}

// Calcula a Merkle root a partir das folhas.
pub fn merkle_root(leaves: &[String]) -> Result<String, String> {
    if leaves.is_empty() {
        return Err("Cannot build Merkle root from empty leaves".to_string());
    }

    let mut level = leaves.to_vec();

    while level.len() > 1 {
        let mut next = Vec::new();

        for pair in level.chunks(2) {
            let left = &pair[0];
            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };

            next.push(hash_pair(left, right));
        }

        level = next;
    }

    Ok(level[0].clone())
}

// Gera a Merkle proof de uma folha pelo seu índice.
pub fn merkle_proof(leaves: &[String], mut index: usize) -> Result<Vec<MerkleProofNode>, String> {
    if leaves.is_empty() {
        return Err("Cannot build Merkle proof from empty leaves".to_string());
    }

    if index >= leaves.len() {
        return Err("Leaf index out of bounds".to_string());
    }

    let mut proof = Vec::new();
    let mut level = leaves.to_vec();

    while level.len() > 1 {
        let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };

        let sibling_hash = if sibling_index < level.len() {
            level[sibling_index].clone()
        } else {
            level[index].clone()
        };

        proof.push(MerkleProofNode {
            hash: sibling_hash,
            is_left: index % 2 == 1,
        });

        let mut next = Vec::new();

        for pair in level.chunks(2) {
            let left = &pair[0];
            let right = if pair.len() == 2 { &pair[1] } else { &pair[0] };

            next.push(hash_pair(left, right));
        }

        index /= 2;
        level = next;
    }

    Ok(proof)
}

// Verifica se uma folha pertence a uma Merkle root.
pub fn verify_merkle_proof(leaf: &str, proof: &[MerkleProofNode], root: &str) -> bool {
    let mut current = leaf.to_string();

    for node in proof {
        current = if node.is_left {
            hash_pair(&node.hash, &current)
        } else {
            hash_pair(&current, &node.hash)
        };
    }

    current == root
}
