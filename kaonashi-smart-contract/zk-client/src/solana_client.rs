use std::{rc::Rc, str::FromStr};

use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
    },
    Client, Cluster, Program,
};
use anyhow::Result;

pub const PROGRAM_ID: &str = "4ybufDXMBSQpQ6kxGqEud9afLC9ayoN925Fk6SkAJxx7";

pub type KaonashiProgram = Program<Rc<Keypair>>;

pub fn connect_localnet() -> Result<KaonashiProgram> {
    let payer_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Não foi possível encontrar o home directory"))?
        .join(".config/solana/id.json");

    let payer = read_keypair_file(&payer_path)
        .map_err(|error| anyhow::anyhow!("Failed to read keypair: {}", error))?;

    let client = Client::new_with_options(
        Cluster::Localnet,
        Rc::new(payer),
        CommitmentConfig::processed(),
    );

    let program_id = Pubkey::from_str(PROGRAM_ID)
        .map_err(|error| anyhow::anyhow!("Invalid Program ID: {}", error))?;

    client
        .program(program_id)
        .map_err(|error| anyhow::anyhow!("Failed to connect to program: {}", error))
}

pub fn initialize_ballot(
    program: &KaonashiProgram,
    ballot: &Keypair,
    proposals: Vec<String>,
    public_key: [u8; 32],
    initial_encrypted_tally: Vec<[u8; 64]>,
) -> Result<()> {
    program
        .request()
        .accounts(projeto_kaonashi::accounts::Initialize {
            ballot: ballot.pubkey(),
            chairperson: program.payer(),
            system_program: anchor_lang::system_program::ID,
        })
        .args(projeto_kaonashi::instruction::Initialize {
            proposals,
            public_key,
            initial_encrypted_tally,
        })
        .signer(ballot)
        .send()
        .map_err(|error| anyhow::anyhow!("Failed to initialize ballot: {}", error))?;

    Ok(())
}

pub fn register_voter(program: &KaonashiProgram, ballot: Pubkey, voter: Pubkey) -> Result<Pubkey> {
    let (voter_record, _) =
        Pubkey::find_program_address(&[b"voter", ballot.as_ref(), voter.as_ref()], &program.id());

    program
        .request()
        .accounts(projeto_kaonashi::accounts::RegisterVoter {
            ballot,
            chairperson: program.payer(),
            voter,
            voter_record,
            system_program: anchor_lang::system_program::ID,
        })
        .args(projeto_kaonashi::instruction::RegisterVoter {})
        .send()
        .map_err(|error| anyhow::anyhow!("Failed to register voter: {}", error))?;

    Ok(voter_record)
}

pub fn submit_rollup_batch(
    program: &KaonashiProgram,
    ballot: Pubkey,
    merkle_root: [u8; 32],
    encrypted_batch_tally: Vec<[u8; 64]>,
    batch_size: u64,
) -> Result<()> {
    program
        .request()
        .accounts(projeto_kaonashi::accounts::SubmitRollupBatchAccounts {
            ballot,
            chairperson: program.payer(),
        })
        .args(projeto_kaonashi::instruction::SubmitRollupBatch {
            new_merkle_root: merkle_root,
            encrypted_batch_tally,
            batch_size,
        })
        .send()
        .map_err(|error| anyhow::anyhow!("Failed to submit rollup batch: {}", error))?;

    Ok(())
}

pub fn fetch_ballot(program: &KaonashiProgram, ballot: Pubkey) -> Result<projeto_kaonashi::Ballot> {
    program
        .account::<projeto_kaonashi::Ballot>(ballot)
        .map_err(|error| anyhow::anyhow!("Failed to fetch ballot account: {}", error))
}

pub fn set_final_winner(program: &KaonashiProgram, ballot: Pubkey, winner_index: u8) -> Result<()> {
    program
        .request()
        .accounts(projeto_kaonashi::accounts::SetFinalWinner {
            ballot,
            chairperson: program.payer(),
        })
        .args(projeto_kaonashi::instruction::SetFinalWinner { winner_index })
        .send()
        .map_err(|error| anyhow::anyhow!("Failed to set final winner: {}", error))?;

    Ok(())
}

pub fn close_election(program: &KaonashiProgram, ballot: Pubkey) -> Result<()> {
    program
        .request()
        .accounts(projeto_kaonashi::accounts::ManageElection {
            ballot,
            chairperson: program.payer(),
        })
        .args(projeto_kaonashi::instruction::CloseElection {})
        .send()
        .map_err(|error| anyhow::anyhow!("Failed to close election: {}", error))?;

    Ok(())
}
