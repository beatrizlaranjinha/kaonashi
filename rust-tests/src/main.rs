use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program_test::ProgramTest; //cria uma blockchain local de teste
                                      // use solana_sdk::signature::read_keypair_file; //para usar com json
use solana_sdk::{
    instruction::Instruction,     //chama função do programa
    signature::{Keypair, Signer}, //cria chaves e contas
    transaction::Transaction,
};

// importar o contrato
use kaonashi::{accounts, instruction, VotingState};

#[tokio::main] //para funções assincronas
async fn main() {
    // id do programa
    let program_id = kaonashi::id();

    // usar o .so compilado
    let program_test = ProgramTest::new("kaonashi", program_id, None);

    // arrancar blockchain local
    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    // conta onde guardamos os dados
    let voting_state_account = Keypair::new(); //cria a pubkey do new owner do contrato

    //initialize -> cria uma instrução para chamar a função initialize do programa , indica as contas e dados

    let i = Instruction {
        //cria uma instrução para chamar initialize (guarda owner e value)
        program_id,
        accounts: accounts::Initialize {
            //diz ao programa que queremos começar uma instrução
            voting_state: voting_state_account.pubkey(), //onde guarda o owner e o value por agoraaa
            user: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID, //necessário para criar contas
        }
        .to_account_metas(None), //converter para o formato que a transação quer
        data: instruction::Initialize {}.data(), //qual é a função que queremos executar
    };

    let mut tx = Transaction::new_with_payer(&[i], Some(&payer.pubkey())); //cria uma transação
    tx.sign(&[&payer, &voting_state_account], recent_blockhash); //a transição tem de ser paga pelo pauer e pela conta que esta a ser criada
                                                                 //Associa o blockhash mais recente
    banks_client.process_transaction(tx).await.unwrap(); //envia para a blockchain local

    println!("Initialize doneee");

    //set value
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let i = Instruction {
        program_id,
        accounts: accounts::OnlyOwner {
            //quer a assinatura do owner
            voting_state: voting_state_account.pubkey(),
            owner: payer.pubkey(),
        }
        .to_account_metas(None),
        data: instruction::SetValue { value: 10 }.data(), //em vez de initialize muda o value
    };

    let mut tx = Transaction::new_with_payer(&[i], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    println!("Value mudado para 10");

    // 3. change_owner

    let new_owner = Keypair::new(); //blablah.pubkey()    solana-keygen new    ~/.config/solana/id.json

    /*use solana_sdk::signature::read_keypair_file;

    let other_user = read_keypair_file("~/.config/solana/id.json")
        .expect("failed to read keypair"); */

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let i = Instruction {
        //Instrução para chamar change owner
        program_id,
        accounts: accounts::OnlyOwner {
            voting_state: voting_state_account.pubkey(), //conta com os dados
            owner: payer.pubkey(),                       //owner que esta autorizado
        }
        .to_account_metas(None), //converte para o formato esperado
        data: instruction::ChangeOwner {
            new_owner: new_owner.pubkey(),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[i], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash); //cria a transação com o owner atual
    banks_client.process_transaction(tx).await.unwrap(); //envia a transação para a blockchain atual

    println!("Owner alterado");

    // 4. non-owner tenta mudar value

    let fake_owner = Keypair::new();
    /*let fake_owner = read_keypair_file("~/.config/solana/id.json")
    .expect("failed to read keypair"); */
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let i = Instruction {
        program_id,
        accounts: accounts::OnlyOwner {
            voting_state: voting_state_account.pubkey(),
            owner: fake_owner.pubkey(),
        }
        .to_account_metas(None),
        data: instruction::SetValue { value: 99 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[i], Some(&payer.pubkey()));
    tx.sign(&[&payer, &fake_owner], recent_blockhash);

    let result = banks_client.process_transaction(tx).await;

    match result {
        Ok(_) => println!("isto não é suposto acontecerrrr"),
        Err(_) => println!("yay funcionou"),
    }

    // 5. ler dados (get)

    let account = banks_client
        .get_account(voting_state_account.pubkey()) //vai a blockchain e procura voting_state
        .await
        .unwrap() //espera resultado async
        .unwrap(); //garante que a conta existe

    let mut data_slice: &[u8] = &account.data; //Os dados vÊm como u[8],
    let voting_state = VotingState::try_deserialize(&mut data_slice).unwrap(); //Converter para struct , bytes ficam VotingState {owner,value}

    println!("Owner atual: {}", voting_state.owner);
    println!("Value atual: {}", voting_state.value);
}
