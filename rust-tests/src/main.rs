use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program_test::ProgramTest;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

// importar o contrato
use kaonashi::{accounts, instruction, Data};

#[tokio::main]
async fn main() {
    // id do programa
    let program_id = kaonashi::id();

    // usar o .so compilado
    let program_test = ProgramTest::new("kaonashi", program_id, None);

    // arrancar blockchain local
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // conta onde guardamos os dados
    let data_account = Keypair::new();

    // =========================
    // 1. initialize
    // =========================
    let ix = Instruction {
        program_id,
        accounts: accounts::Initialize {
            data: data_account.pubkey(),
            user: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
        data: instruction::Initialize {}.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &data_account], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    println!("Initialize feito");

    // =========================
    // 2. set_value
    // =========================
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let ix = Instruction {
        program_id,
        accounts: accounts::OnlyOwner {
            data: data_account.pubkey(),
            owner: payer.pubkey(),
        }
        .to_account_metas(None),
        data: instruction::SetValue { value: 10 }.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    println!("Value mudado para 10");

    // =========================
    // 3. change_owner
    // =========================
    let new_owner = Keypair::new();
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();

    let ix = Instruction {
        program_id,
        accounts: accounts::OnlyOwner {
            data: data_account.pubkey(),
            owner: payer.pubkey(),
        }
        .to_account_metas(None),
        data: instruction::ChangeOwner {
            new_owner: new_owner.pubkey(),
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    println!("Owner alterado");

    // =========================
    // 4. ler dados (get)
    // =========================
    let account = banks_client
        .get_account(data_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let mut data_slice: &[u8] = &account.data;
    let data_state = Data::try_deserialize(&mut data_slice).unwrap();

    println!("Owner atual: {}", data_state.owner);
    println!("Value atual: {}", data_state.value);
}
