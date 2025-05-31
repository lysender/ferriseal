use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};

use crate::Result;

pub fn encrypt(key: &str, data: &str) -> Result<String> {
    todo!()
}

pub fn decrypt(key: &str, data: &str) -> Result<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = "371d6394db654411b64a3366d407d8f7";
        let plain = "secret-password";

        let crypted = encrypt(key, plain).unwrap();
        let plain_back = decrypt(key, &crypted).unwrap();
        assert_eq!(plain, plain_back);
    }
}
