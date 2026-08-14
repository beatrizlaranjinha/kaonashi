use std::fs;

fn main() {
    let home = std::env::var("HOME").unwrap();
    let path = format!("{}/.config/solana/id.json", home);

    let content = fs::read_to_string(path)
        .expect("Could not read Solana keypair");

    let bytes: Vec<u8> = serde_json::from_str(&content)
        .expect("Invalid Solana keypair JSON");

    println!("{}", bs58::encode(bytes).into_string());
}
