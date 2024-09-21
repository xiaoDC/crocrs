use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, // Or `Aes128Gcm`
    Nonce,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use sha2::Sha256;

// New generates a new key based on a passphrase and salt
pub fn new(
    passphrase: &[u8],
    usersalt: &[u8],
) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    if passphrase.is_empty() {
        anyhow::bail!("need more than that for passphrase")
    }

    let salt: Vec<u8> = if usersalt.is_empty() {
        let mut rng = StdRng::from_entropy();
        let random: [u8; 8] = rng.gen();
        random.to_vec()
    } else {
        usersalt.to_vec()
    };
    let key = pbkdf2::pbkdf2_hmac_array::<Sha256, 32>(passphrase, &salt, 100);
    Ok((key.to_vec(), salt))
}

// Encrypt will encrypt using the pre-generated key
pub fn encrypt(
    plaintext: &[u8],
    key: &[u8],
) -> anyhow::Result<Vec<u8>> {
    // generate a random iv each time
    // http://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf
    // Section 8.2
    let mut rng = StdRng::from_entropy();
    let iv: [u8; 12] = rng.gen();
    let cipher = Aes256Gcm::new(key.into());
    let mut encrypted = cipher
        .encrypt(Nonce::from_slice(&iv), plaintext)
        .map_err(|err| anyhow::anyhow!(format!("{:?}", err)))?;
    let mut rst = Vec::from(iv);
    rst.append(&mut encrypted);
    Ok(rst)
}

// Decrypt using the pre-generated key
pub fn decrypt(
    encrypted: &[u8],
    key: &[u8],
) -> anyhow::Result<Vec<u8>> {
    if encrypted.len() < 13 {
        anyhow::bail!("incorrect passphrase")
    }
    let cipher = Aes256Gcm::new(key.into());
    let (left, right) = encrypted.split_at(12);
    let iv = Nonce::from_slice(left);
    let plaintext =
        cipher.decrypt(iv, right).map_err(|err| anyhow::anyhow!(format!("{:?}", err)))?;
    Ok(plaintext)
}
