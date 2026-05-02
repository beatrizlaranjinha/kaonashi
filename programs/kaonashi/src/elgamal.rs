//structs

#[derive(clone, copy, debug)]
pub struct PublicKey {
    pub p: u64, //primo
    pub q: u64, //group order
    pub g: u64, //generator
    pub h: u64, //h = g^x mod p
}

#[derive(clone, copy, debug)]
pub struct secretKey {
    pub x: u64, //private key
}

#[derive(clone, copy, debug)]
pub struct ciphertext {
    pub c1: u64, // c1 = g^y mod p
    pub c2: u64, // c2 = h^y * m mod p
}
//funções que vão ser utilizadas

// x -> base , n -> expoente , p-> modulus
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
pub fn mod_inverse(a: u64, p: u64) -> Result<u64, &'static str> {
    if a == 0 {
        return Err("non existent");
    }

    Ok(mod_pow(a, p - 2, p))
}
