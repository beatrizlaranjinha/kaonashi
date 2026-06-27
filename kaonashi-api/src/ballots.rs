use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn ballot_for_decade(decade_id: u8) -> Option<Pubkey> {
    let address = match decade_id {
        0 => "4eozuE6yuNwukUjTiEQbCMBboYzNMJaoPnc25s4eF2k5",
        1 => "4WWtLV9ojrACQeZttrVzpPjn3fze1WyPk9GKaABWAFwA",
        2 => "H2LQfqa9gBfuNeDdvMh6v9mqmypkqKNvsVtwdrMjhiFp",
        3 => "2e7F5tCcZfrF1HrNNJtpGMfE11EotNwVPCv47MpmNs7D",
        4 => "G5aAgUgArtC4CVUV1TSzLMKrYHxe9SYGAULSdFsA2xYj",
        5 => "AgknD6zjkoJFrjb9mDjSs9fsxVxu7J2Ev6dEBNYdahC5",
        _ => return None,
    };

    Pubkey::from_str(address).ok()
}
