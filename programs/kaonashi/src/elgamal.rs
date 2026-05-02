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

// Encryption
pub fn encrypt(pk: &PublicKey, m: u64, y: u64) -> Ciphertext {
    let c1 = mod_pow(pk.g, y, pk.p); // g^y mod p
    let s = mod_pow(pk.h, y, pk.p); // h^y mod p (máscara)
    let c2 = (s * m) % pk.p; // h^y * m mod p
    Ciphertext { c1, c2 }
}

// Decryption
pub fn decrypt(pk: &PublicKey, sk: &SecretKey, ct: &Ciphertext) -> Result<u64, ElGamalError> {
    let s = mod_pow(ct.c1, sk.x, pk.p); // c1^x mod p
    let inv = mod_inverse(s, pk.p)?; // inverso de s
    let m = (ct.c2 * inv) % pk.p; // recuperar m

    Ok(m)
}

//agora vamos lidar com os nossos votos

// converter voto 0/1 para elemento do grupo
pub fn encode_vote(pk: &PublicKey, vote: u64) -> u64 {
    mod_pow(pk.g, vote, pk.p) // vote 0 -> 1, vote 1 -> g
}

// cifrar diretamente um voto 0/1
pub fn encrypt_vote(pk: &PublicKey, vote: u64, y: u64) -> Ciphertext {
    let m = encode_vote(pk, vote);
    encrypt(pk, m, y)
}

// somar votos cifrados
pub fn add_ciphertexts(pk: &PublicKey, a: &Ciphertext, b: &Ciphertext) -> Ciphertext {
    Ciphertext {
        c1: (a.c1 * b.c1) % pk.p,
        c2: (a.c2 * b.c2) % pk.p,
    }
}

// criar tally inicial E(0)
pub fn encrypted_zero(pk: &PublicKey, y: u64) -> Ciphertext {
    encrypt_vote(pk, 0, y) //para adicionar o primeiro tally
}

// cifrar vetor de voto
pub fn encrypt_vote_vector(
    pk: &PublicKey,
    vote_index: usize,
    proposal_count: usize,
    y_values: &[u64], // y aleatorio para cada posição
) -> Vec<Ciphertext> {
    (0..proposal_count) //iterador
        .map(|i| encrypt_vote(pk, (i == vote_index) as u64, y_values[i])) //aplica encrypt_vote a cada i, onde o voto é 1 se i == vote_index, caso contrário 0
        .collect() //Junta tudo num
}
