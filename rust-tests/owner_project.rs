use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

use kaonashi::{accounts, instruction, Data};

#[tokio::test]
async fn test_initialize_set_value_change_owner() {
    let program_id = kaonashi::id();

    let program_test = ProgramTest::new("kaonashi", program_id, None);
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let data_account = Keypair::new();

    // initialize
    let ix = Instruction {
        program_id,
        accounts: accounts::Initialize {
            data: data_account.pubkey(),
            user: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: instruction::Initialize {}.data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer, &data_account], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    // ler conta depois do initialize
    let account = banks_client
        .get_account(data_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let mut data_slice: &[u8] = &account.data;
    let data_state = Data::try_deserialize(&mut data_slice).unwrap();

    assert_eq!(data_state.owner, payer.pubkey());
    assert_eq!(data_state.value, 0);

    // set_value
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

    // ler conta depois do set_value
    let account = banks_client
        .get_account(data_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let mut data_slice: &[u8] = &account.data;
    let data_state = Data::try_deserialize(&mut data_slice).unwrap();

    assert_eq!(data_state.value, 10);

    // change_owner
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

    // ler conta depois do change_owner
    let account = banks_client
        .get_account(data_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let mut data_slice: &[u8] = &account.data;
    let data_state = Data::try_deserialize(&mut data_slice).unwrap();

    assert_eq!(data_state.owner, new_owner.pubkey());
}
