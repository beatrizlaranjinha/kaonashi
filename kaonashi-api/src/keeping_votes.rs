use std::{collections::HashMap, sync::Mutex};

use solana_zk_sdk::encryption::elgamal::ElGamalKeypair;

use crate::models::{EncryptedVoteBatch, PendingEncryptedVote, VoteReceipt};

// Guarda o estado temporário da API.
pub struct KeepingVotes {
    // Guarda votos antigos em claro, só para não partir endpoints antigos.
    pub votes_by_decade: Mutex<Vec<Vec<u64>>>,

    // Guarda mensagens temporárias usadas no login por assinatura.
    pub login_challenges: Mutex<HashMap<String, String>>,

    // Guarda votos cifrados que já foram verificados, mas ainda não entraram num batch.
    pub pending_encrypted_votes: Mutex<Vec<Vec<PendingEncryptedVote>>>,

    // Guarda batches já criados por década.
    pub encrypted_vote_batches: Mutex<Vec<Vec<EncryptedVoteBatch>>>,

    // Guarda receipts por hash do voto, para o user consultar mais tarde.
    pub vote_receipts_by_hash: Mutex<HashMap<String, VoteReceipt>>,

    // Guarda uma ElGamal keypair por década.
    pub elgamal_keypairs_by_decade: Mutex<Vec<ElGamalKeypair>>,
}

impl KeepingVotes {
    // Cria o estado inicial da API.
    pub fn new() -> Self {
        let elgamal_keypairs_by_decade = (0..6)
            .map(|_| ElGamalKeypair::new_rand())
            .collect::<Vec<_>>();

        Self {
            // votes_by_decade[decade_id][movie_index]
            votes_by_decade: Mutex::new(vec![vec![0; 8]; 6]),

            login_challenges: Mutex::new(HashMap::new()),

            // pending_encrypted_votes[decade_id]
            pending_encrypted_votes: Mutex::new(vec![Vec::new(); 6]),

            // encrypted_vote_batches[decade_id]
            encrypted_vote_batches: Mutex::new(vec![Vec::new(); 6]),

            vote_receipts_by_hash: Mutex::new(HashMap::new()),

            elgamal_keypairs_by_decade: Mutex::new(elgamal_keypairs_by_decade),
        }
    }
}
