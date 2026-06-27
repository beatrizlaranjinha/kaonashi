use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub fn ballot_for_decade(decade_id: u8) -> Option<Pubkey> {
    let address = match decade_id {
        0 => "99g8mWXpfdAL52gRbdyb2dJ73bC63tu4HToEcp8SghnK",
        1 => "H2uTc4ub9iR8X6dMM2Ep9VFTK59jsEsUET8ETbXNurfA",
        2 => "9rFbofStFnBCdCozvn92youqudZmB4DQ88hheduyToPL",
        3 => "H8fYHnfcP1youLzJT85deaTss6TBZMaBnndT8yqiBDev",
        4 => "FHqc74GJNdh6UFZVbDoFm4G5jUCRkSDZhn2D9pg1snNn",
        5 => "F5mU7Anj2nrvV5CvvExwh8yGgHsVZRfn3AzvpSgNQK7R",
        _ => return None,
    };

    Pubkey::from_str(address).ok()
}
