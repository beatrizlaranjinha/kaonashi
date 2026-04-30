use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program_test::ProgramTest; // cria uma blockchain local de teste
use solana_sdk::{
    instruction::Instruction, // representa uma chamada a uma função do programa
    signature::{Keypair, Signer}, // cria contas/chaves e permite assinar transações
    transaction::Transaction, // representa uma transação enviada para a blockchain
};

// importa as contas, instruções e struct principal do contrato
use kaonashi::{accounts, instruction, VotingState};

#[tokio::main] // permite usar async/await no main
async fn main() {
    // ID do programa Anchor
    let program_id = kaonashi::id();

    // Cria uma blockchain local de teste com o programa carregado
    let program_test = ProgramTest::new("kaonashi", program_id, None);

    // Arranca a blockchain local
    // banks_client -> cliente para interagir com a blockchain
    // payer -> conta principal que paga as transações
    // recent_blockhash -> blockhash necessário para assinar a primeira transação
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Conta onde vai ficar guardado o estado da votação
    let voting_state_account = Keypair::new();

    // initialize -> criar a votação com propostas

    // Lista de propostas da votação
    let proposals_names = vec!["wine".to_string(), "beer".to_string(), "water".to_string()];

    // Cria a instrução para chamar initialize no contrato
    let ix = Instruction {
        program_id,
        accounts: accounts::Initialize {
            voting_state: voting_state_account.pubkey(), // conta onde ficam os dados da votação
            user: payer.pubkey(),                        // quem cria a votação fica chairperson
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
        data: instruction::Initialize { proposals_names }.data(),
    };

    // Cria a transação com a instrução initialize
    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // Assina com o payer e com a conta voting_state_account,
    // porque esta conta está a ser criada agora
    tx.sign(&[&payer, &voting_state_account], recent_blockhash);

    // Envia a transação para a blockchain local
    banks_client.process_transaction(tx).await.unwrap();

    println!("Initialize feito");
    println!("Chairperson: {}", payer.pubkey());

    // Criar voters

    // Cria dois utilizadores/voters
    let voter1 = Keypair::new();
    let voter2 = Keypair::new();

    // Dar direito de voto ao voter1

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let ix = Instruction {
        program_id,
        accounts: accounts::OnlyChairperson {
            voting_state: voting_state_account.pubkey(),
            chairperson: payer.pubkey(), // só o chairperson pode dar direito de voto
        }
        .to_account_metas(None),
        data: instruction::GiveRightToVote {
            voter: voter1.pubkey(),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // Só o chairperson precisa de assinar esta transação
    tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    println!("Voter1 autorizado");

    // Dar direito de voto ao voter2
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let ix = Instruction {
        program_id,
        accounts: accounts::OnlyChairperson {
            voting_state: voting_state_account.pubkey(),
            chairperson: payer.pubkey(),
        }
        .to_account_metas(None),
        data: instruction::GiveRightToVote {
            voter: voter2.pubkey(),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    println!("Voter2 autorizado");

    // 5. Chairperson vota em wine
    // 0 -> wine
    // 1 -> beer
    // 2 -> water

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let ix = Instruction {
        program_id,
        accounts: accounts::Vote {
            voting_state: voting_state_account.pubkey(),
            voter: payer.pubkey(), // chairperson é também voter
        }
        .to_account_metas(None),
        data: instruction::Vote { proposal_index: 0 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // O chairperson assina porque é ele que está a votar
    tx.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    println!("Chairperson votou em wine");

    // Voter1 vota em beer

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let ix = Instruction {
        program_id,
        accounts: accounts::Vote {
            voting_state: voting_state_account.pubkey(),
            voter: voter1.pubkey(),
        }
        .to_account_metas(None),
        data: instruction::Vote { proposal_index: 1 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // O payer paga a transação, mas o voter1 também tem de assinar
    // porque é ele que está a votar
    tx.sign(&[&payer, &voter1], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    println!("Voter1 votou em beer");

    // Voter2 vota em beer

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let ix = Instruction {
        program_id,
        accounts: accounts::Vote {
            voting_state: voting_state_account.pubkey(),
            voter: voter2.pubkey(),
        }
        .to_account_metas(None),
        data: instruction::Vote { proposal_index: 1 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));

    // O voter2 tem de assinar porque é ele que está a votar
    tx.sign(&[&payer, &voter2], recent_blockhash);

    banks_client.process_transaction(tx).await.unwrap();

    println!("Voter2 votou em beer");

    //  Testar double voting
    // o voter1 tenta votar outra vez.
    // já tem voted = true.

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let ix = Instruction {
        program_id,
        accounts: accounts::Vote {
            voting_state: voting_state_account.pubkey(),
            voter: voter1.pubkey(),
        }
        .to_account_metas(None),
        data: instruction::Vote { proposal_index: 2 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &voter1], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;

    match result {
        Ok(_) => println!("Erro: o voter1 conseguiu votar duas vezes"),
        Err(_) => println!("Double voting bloqueado corretamente"),
    }

    //9. Ler o estado final da votação

    let account = banks_client
        .get_account(voting_state_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    // A conta vem em bytes, por isso temos de desserializar
    let mut data_slice: &[u8] = &account.data;
    let voting_state = VotingState::try_deserialize(&mut data_slice).unwrap();

    println!("\nEstado final da votação:");
    println!("Chairperson atual: {}", voting_state.chairperson);
    println!(
        "Voto do chairperson: {:?}",
        voting_state.chairperson_vote_index
    );

    // Mostrar os resultados das propostas
    println!("\nResultados:");
    for (i, proposal) in voting_state.proposals.iter().enumerate() {
        println!("{} -> {} votos: {}", i, proposal.name, proposal.vote_count);
    }

    // Mostrar todos os voters registados
    println!("\nVoters:");
    for voter in voting_state.voters.iter() {
        println!(
            "address: {}, allowed: {}, voted: {}, vote_index: {:?}, vote_vector: {:?}",
            voter.address, voter.allowed_to_vote, voter.voted, voter.vote_index, voter.vote_vector
        );
    }
}
