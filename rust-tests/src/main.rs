use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program_test::ProgramTest; // cria uma blockchain local de teste
use solana_sdk::{
    instruction::Instruction, // representa uma chamada a uma função do programa
    pubkey::Pubkey,           // usado para calcular PDA
    signature::{Keypair, Signer}, // cria contas/chaves e permite assinar transações
    transaction::Transaction, // representa uma transação enviada para a blockchain
};

// importa as contas e instruções do contrato
use kaonashi::owner_project::{Ballot, VoterRecord};
use kaonashi::{accounts, instruction};

#[tokio::main] // permite usar async/await no main
async fn main() {
    // ID do programa Anchor
    let program_id = kaonashi::id();

    // Cria uma blockchain local de teste com o programa carregado
    let program_test = ProgramTest::new("kaonashi", program_id, None);

    // Arranca a blockchain local
    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    // Conta global onde vai ficar guardada a votação
    // Esta conta é a Ballot Account
    let ballot_account = Keypair::new();

    // Estes nomes ficam guardados on-chain dentro da Ballot
    let proposals_names = vec!["wine".to_string(), "beer".to_string(), "water".to_string()];

    // 1. INITIALIZE
    // Cria a votação

    let ix = Instruction {
        program_id,
        accounts: accounts::Initialize {
            ballot: ballot_account.pubkey(), // conta global da votação
            chairperson: payer.pubkey(),     // quem cria a votação fica chairperson
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
        data: instruction::Initialize { proposals_names }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // Assina com:
    tx.sign(&[&payer, &ballot_account], recent_blockhash);

    banks_client
        .process_transaction(tx)
        .await
        .expect("Erro ao fazer initialize");

    println!("Initialize feito");
    println!("Chairperson: {}", payer.pubkey());
    println!("Ballot account: {}", ballot_account.pubkey());

    // 2. CRIAR VOTERS

    let voter1 = Keypair::new();
    let voter2 = Keypair::new();

    println!("\nVoter1: {}", voter1.pubkey());
    println!("Voter2: {}", voter2.pubkey());

    // Cada voter tem uma conta PDA própria para esta votação
    // seeds = [b"voter", ballot.key().as_ref(), voter.key().as_ref()]

    let (voter1_record_pda, _bump1) = Pubkey::find_program_address(
        &[
            b"voter",
            ballot_account.pubkey().as_ref(),
            voter1.pubkey().as_ref(),
        ],
        &program_id,
    );

    let (voter2_record_pda, _bump2) = Pubkey::find_program_address(
        &[
            b"voter",
            ballot_account.pubkey().as_ref(),
            voter2.pubkey().as_ref(),
        ],
        &program_id,
    );

    println!("\nVoter1 PDA: {}", voter1_record_pda);
    println!("Voter2 PDA: {}", voter2_record_pda);

    // O chairperson autoriza o voter1 a votar
    // Isto cria a conta PDA VoterRecord do voter1

    let recent_blockhash = banks_client
        .get_latest_blockhash()
        .await
        .expect("Erro ao obter blockhash");

    let ix = Instruction {
        program_id,
        accounts: accounts::RegisterVoter {
            chairperson: payer.pubkey(),     // só o chairperson pode registar voters
            ballot: ballot_account.pubkey(), // votação onde o voter vai ser registado
            voter_record: voter1_record_pda, // PDA do voter1
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
        data: instruction::RegisterVoter {
            _voter_address: voter1.pubkey(),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // Só o chairperson assina.
    // O voter1 não precisa de assinar para ser registado.
    tx.sign(&[&payer], recent_blockhash);

    banks_client
        .process_transaction(tx)
        .await
        .expect("Erro ao registar voter1");

    println!("\nVoter1 autorizado");

    // O chairperson autoriza o voter2 a votar

    let recent_blockhash = banks_client
        .get_latest_blockhash()
        .await
        .expect("Erro ao obter blockhash");

    let ix = Instruction {
        program_id,
        accounts: accounts::RegisterVoter {
            chairperson: payer.pubkey(),
            ballot: ballot_account.pubkey(),
            voter_record: voter2_record_pda,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
        data: instruction::RegisterVoter {
            _voter_address: voter2.pubkey(),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    banks_client
        .process_transaction(tx)
        .await
        .expect("Erro ao registar voter2");

    println!("Voter2 autorizado");

    // proposals:
    // 0 -> wine
    // 1 -> beer
    // 2 -> water

    let recent_blockhash = banks_client
        .get_latest_blockhash()
        .await
        .expect("Erro ao obter blockhash");

    let ix = Instruction {
        program_id,
        accounts: accounts::CastVote {
            voter: voter1.pubkey(),          // quem está a votar
            ballot: ballot_account.pubkey(), // votação
            voter_record: voter1_record_pda, // PDA do voter1
        }
        .to_account_metas(None),
        data: instruction::CastVote { proposal_index: 0 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // payer paga a transação
    // voter1 assina porque é ele que está a votar
    tx.sign(&[&payer, &voter1], recent_blockhash);

    banks_client
        .process_transaction(tx)
        .await
        .expect("Erro ao votar com voter1");

    println!("\nVoter1 votou em wine");

    // Isto vai criar empate:
    // wine -> 1 voto
    // beer -> 1 voto
    // water -> 0 votos

    let recent_blockhash = banks_client
        .get_latest_blockhash()
        .await
        .expect("Erro ao obter blockhash");

    let ix = Instruction {
        program_id,
        accounts: accounts::CastVote {
            voter: voter2.pubkey(),
            ballot: ballot_account.pubkey(),
            voter_record: voter2_record_pda,
        }
        .to_account_metas(None),
        data: instruction::CastVote { proposal_index: 1 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &voter2], recent_blockhash);

    banks_client
        .process_transaction(tx)
        .await
        .expect("Erro ao votar com voter2");

    println!("Voter2 votou em beer");

    // O voter1 tenta votar outra vez.
    // Como has_voted = true, o contrato deve bloquear.

    let recent_blockhash = banks_client
        .get_latest_blockhash()
        .await
        .expect("Erro ao obter blockhash");

    let ix = Instruction {
        program_id,
        accounts: accounts::CastVote {
            voter: voter1.pubkey(),
            ballot: ballot_account.pubkey(),
            voter_record: voter1_record_pda,
        }
        .to_account_metas(None),
        data: instruction::CastVote { proposal_index: 2 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &voter1], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;

    match result {
        Ok(_) => println!("Erro: o voter1 conseguiu votar duas vezes"),
        Err(_) => println!("Double voting bloqueado corretamente"),
    }

    // Como há empate entre wine e beer, o chairperson escolhe beer.
    // winning_index = 1 -> beer

    let recent_blockhash = banks_client
        .get_latest_blockhash()
        .await
        .expect("Erro ao obter blockhash");

    let ix = Instruction {
        program_id,
        accounts: accounts::ResolveTie {
            chairperson: payer.pubkey(),     // só o chairperson pode resolver empate
            ballot: ballot_account.pubkey(), // votação onde existe empate
        }
        .to_account_metas(None),
        data: instruction::ResolveTie { winning_index: 1 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // Só o chairperson assina.
    tx.sign(&[&payer], recent_blockhash);

    banks_client
        .process_transaction(tx)
        .await
        .expect("Erro ao resolver empate");

    println!("Chairperson resolveu o empate: beer ganhou");

    // estado final do ballot

    let account = banks_client
        .get_account(ballot_account.pubkey())
        .await
        .expect("Erro ao ler Ballot")
        .expect("Ballot account não encontrada");

    let mut data_slice: &[u8] = &account.data;
    let ballot = Ballot::try_deserialize(&mut data_slice).expect("Erro ao desserializar Ballot");

    println!("\nEstado final da votação:");
    println!("Chairperson: {}", ballot.chairperson);
    println!("Proposal count: {}", ballot.proposal_count);
    println!("Final winner index: {}", ballot.final_winner_index);

    println!("\nResultados:");

    for i in 0..ballot.proposal_count as usize {
        println!(
            "{} -> {} votos: {}",
            i, ballot.proposals[i], ballot.tally[i]
        );
    }

    // 11. vote record do voter1

    let account = banks_client
        .get_account(voter1_record_pda)
        .await
        .expect("Erro ao ler VoterRecord do voter1")
        .expect("VoterRecord do voter1 não encontrado");

    let mut data_slice: &[u8] = &account.data;
    let voter1_record = VoterRecord::try_deserialize(&mut data_slice)
        .expect("Erro ao desserializar VoterRecord do voter1");

    println!("\nVoter1 record:");
    println!("can_vote: {}", voter1_record.can_vote);
    println!("has_voted: {}", voter1_record.has_voted);
    println!("vote: {}", voter1_record.vote);

    // vote recorde do voter2

    let account = banks_client
        .get_account(voter2_record_pda)
        .await
        .expect("Erro ao ler VoterRecord do voter2")
        .expect("VoterRecord do voter2 não encontrado");

    let mut data_slice: &[u8] = &account.data;
    let voter2_record = VoterRecord::try_deserialize(&mut data_slice)
        .expect("Erro ao desserializar VoterRecord do voter2");

    println!("\nVoter2 record:");
    println!("can_vote: {}", voter2_record.can_vote);
    println!("has_voted: {}", voter2_record.has_voted);
    println!("vote: {}", voter2_record.vote);
}
