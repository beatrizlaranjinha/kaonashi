//structs

#[derive(Clone, Copy, Debug)]
pub struct PublicKey {
    pub p: u64, //primo
    pub q: u64, //group order
    pub g: u64, //generator
    pub h: u64, //h = g^x mod p
}

#[derive(Clone, Copy, Debug)]
pub struct SecretKey {
    pub x: u64, //private key
}

#[derive(Clone, Copy, Debug)]
pub struct Ciphertext {
    pub c1: u64, // c1 = g^y mod p
    pub c2: u64, // c2 = h^y * m mod p
}

#[derive(Debug)]
pub enum ElGamalError {
    InvalidInverse,
    LogNotFound,
}

//funções que vão ser utilizadas

// x -> base , n -> expoente , p-> modulus
// exponente modular
pub fn mod_pow(mut x: u64, mut n: u64, p: u64) -> u64 {
    let mut res = 1;

    x %= p; // 4 % 167 = 4 ; 200 % 167 = 33 manter dentro do intrevalo ,se x ≥ p → reduz

    while n > 0 {
        if n % 2 == 1 {
            // se for impar
            res = (res * x) % p;
        }

        x = (x * x) % p;
        n /= 2;
    }

    res
}

//Inverso multiplicativo
pub fn mod_inverse(a: u64, p: u64) -> Result<u64, ElGamalError> {
    if a == 0 {
        return Err(ElGamalError::InvalidInverse);
    }

    Ok(mod_pow(a, p - 2, p))
}

// Elgamal: keygen , encryption and decryption

// Key Generation
pub fn keygen(p: u64, q: u64, g: u64, x: u64) -> (PublicKey, SecretKey) {
    let h = mod_pow(g, x, p); // h = g^x mod p //calculado a partir do segredo

    let pk = PublicKey { p, q, g, h };
    let sk = SecretKey { x };

    (pk, sk)
}
