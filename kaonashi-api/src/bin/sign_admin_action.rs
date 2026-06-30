use solana_sdk::signature::{read_keypair_file, Signer};
use std::{env, path::PathBuf};

fn admin_message(public_key: &str, action: &str, decade_id: Option<u8>) -> String {
    match decade_id {
        Some(decade_id) => format!(
            "Kaonashi admin action\npublic_key: {}\naction: {}\ndecade_id: {}",
            public_key, action, decade_id
        ),
        None => format!(
            "Kaonashi admin action\npublic_key: {}\naction: {}",
            public_key, action
        ),
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin sign_admin_action -- <action> [decade_id]");
        std::process::exit(1);
    }

    let action = &args[1];

    let decade_id = if args.len() >= 3 {
        Some(args[2].parse::<u8>().expect("Invalid decade_id"))
    } else {
        None
    };

    let home = env::var("HOME").expect("HOME environment variable not set");
    let keypair_path = PathBuf::from(home).join(".config/solana/id.json");

    let keypair = read_keypair_file(&keypair_path)
        .expect("Failed to read Solana keypair");

    let public_key = keypair.pubkey().to_string();
    let message = admin_message(&public_key, action, decade_id);
    let signature = keypair.sign_message(message.as_bytes()).to_string();

    let body = serde_json::json!({
        "public_key": public_key,
        "message": message,
        "signature": signature
    });

    println!("{}", serde_json::to_string_pretty(&body).unwrap());
}
