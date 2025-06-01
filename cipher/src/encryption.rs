use base64::prelude::*;
use chacha20poly1305::{
    Key, XChaCha20Poly1305, XNonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use snafu::{OptionExt, ResultExt, ensure};

use crate::{
    Error, Result,
    error::{CipherSnafu, DecodeSnafu},
};

const DEFAULT_ENC_METHOD: &'static str = "xcha20";

/// Encrypts data with a random key which is encrypted with the master key
/// Result format: enc_method:key_nonce:key_data|enc_method:input_nonce:input_data
pub fn encrypt(key: &str, data: &str) -> Result<String> {
    // Create a random key and encrypt it with the main key
    let random_key = XChaCha20Poly1305::generate_key(OsRng);
    let cipher_key = raw_encrypt(key.as_bytes(), &random_key)?;

    // Now that we have a random key encrypted, encypt the data with it
    let cipher_data = raw_encrypt(&random_key, data.as_bytes())?;
    Ok(format!("{}|{}", cipher_key, cipher_data))
}

/// Encrypts data with the provided key
/// Result format: enc_method:nonce:data
fn raw_encrypt(key: &[u8], data: &[u8]) -> Result<String> {
    let kb = Key::from_slice(key);
    let c = XChaCha20Poly1305::new(&kb);
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    match c.encrypt(&nonce, data) {
        Ok(res) => {
            let bres = BASE64_STANDARD.encode(res);
            let bnonce = BASE64_STANDARD.encode(nonce);
            Ok(format!("{}:{}:{}", DEFAULT_ENC_METHOD, bnonce, bres))
        }
        Err(e) => Err(Error::Cipher { msg: e.to_string() }),
    }
}

pub fn decrypt(key: &str, data: &str) -> Result<String> {
    let mut chunks = data.split('|');
    let key_part = chunks.next().context(CipherSnafu {
        msg: "Cipher text format must be valid",
    })?;

    let data_part = chunks.next().context(CipherSnafu {
        msg: "Cipher text format must be valid",
    })?;

    // Decrypt the key first
    let plain_key = decrypt_part(key.as_bytes(), key_part)?;

    // Decrypt the data using the stored key
    let result = decrypt_part(&plain_key, data_part)?;
    Ok(String::from_utf8_lossy(&result).to_string())
}

fn decrypt_part(key: &[u8], data: &str) -> Result<Vec<u8>> {
    let mut chunks = data.split(':');
    let method = chunks.next().context(CipherSnafu {
        msg: "Cipher text part format must be valid",
    })?;

    ensure!(
        method == DEFAULT_ENC_METHOD,
        CipherSnafu {
            msg: "Encryption method not supported"
        }
    );

    let nonce = chunks.next().context(CipherSnafu {
        msg: "Cipher text part format must be valid",
    })?;
    let data_part = chunks.next().context(CipherSnafu {
        msg: "Cipher text part format must be valid",
    })?;

    raw_decrypt(
        key,
        &BASE64_STANDARD.decode(nonce).context(DecodeSnafu)?,
        &BASE64_STANDARD.decode(data_part).context(DecodeSnafu)?,
    )
}

/// Decrypts the data using the provided key and nonce using default enc method
fn raw_decrypt(key: &[u8], nonce: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let kb = Key::from_slice(key);
    let c = XChaCha20Poly1305::new(&kb);
    let bnonce = XNonce::from_slice(nonce);
    match c.decrypt(&bnonce, data) {
        Ok(res) => Ok(res),
        Err(e) => Err(Error::Cipher { msg: e.to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = "371d6394db654411b64a3366d407d8f7";
        let plain = "the quick brown fox jumps over the lazy dog under the mango tree the quick brown fox jumps";

        let crypted = encrypt(key, plain).unwrap();
        let plain_back = decrypt(key, &crypted).unwrap();
        assert_eq!(plain, plain_back);
    }
}
