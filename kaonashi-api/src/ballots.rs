use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn ballot_for_decade(decade_id: u8) -> Option<Pubkey> {
    let address = match decade_id {
        0 => "87P9jJ7ezeqy9UvXyEGXTky2eSQjTL9f9fskPjEzgEem",
        1 => "FNyVSxWgst96kFQpzpWow18MpTB6BV7EbwxvNcBkZA5G",
        2 => "9PRqtrL8iLnBgeqxGgz6Z9ghqp9rvwMDxsZHPbtZHT2",
        3 => "F6RijPZvME6mY7cDXNftetvVL6RqDcmkf9G6A1zkvTYU",
        4 => "FMDeRAQfxMzknh7xCxCqXYDurGiR593ej2a9cz46msfC",
        5 => "9LdKbbLRVwHUPLJrKgg2LW7SgRoxgEWjTRcYqWFzC2jg",
        _ => return None,
    };

    Pubkey::from_str(address).ok()
}
