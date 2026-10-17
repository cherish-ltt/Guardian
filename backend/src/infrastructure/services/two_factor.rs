use crate::domain::error::AppError;
use aws_lc_rs::{
    aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey},
    digest, rand,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use totp_rs::{Builder, Totp};

pub struct TwoFactorCrypto {
    key: Option<LessSafeKey>,
}

impl TwoFactorCrypto {
    pub fn new(encoded: Option<&str>) -> Result<Self, AppError> {
        let key = encoded
            .map(|encoded| {
                let bytes = STANDARD.decode(encoded).map_err(|_| {
                    AppError::Config("TWO_FACTOR_KEY must be a base64 encoded 32-byte key".into())
                })?;
                let key = UnboundKey::new(&AES_256_GCM, &bytes).map_err(|_| {
                    AppError::Config("TWO_FACTOR_KEY must be a base64 encoded 32-byte key".into())
                })?;
                Ok::<_, AppError>(LessSafeKey::new(key))
            })
            .transpose()?;
        Ok(Self { key })
    }

    fn key(&self) -> Result<&LessSafeKey, AppError> {
        self.key
            .as_ref()
            .ok_or_else(|| AppError::unavailable("TWO_FACTOR_KEY is not configured"))
    }

    pub fn encrypt(&self, user_id: i64, secret: &[u8]) -> Result<String, AppError> {
        let nonce: [u8; 12] = random_bytes()?;
        let mut encrypted = secret.to_vec();
        self.key()?
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(user_id.to_be_bytes()),
                &mut encrypted,
            )
            .map_err(|_| AppError::internal("2FA encryption failed"))?;
        let mut result = nonce.to_vec();
        result.extend(encrypted);
        Ok(STANDARD.encode(result))
    }

    pub fn decrypt(&self, user_id: i64, encrypted: &str) -> Result<Vec<u8>, AppError> {
        let mut bytes = STANDARD
            .decode(encrypted)
            .map_err(|_| AppError::internal("invalid encrypted 2FA secret"))?;
        if bytes.len() < 28 {
            return Err(AppError::internal("invalid encrypted 2FA secret"));
        }
        let nonce: [u8; 12] = bytes[..12]
            .try_into()
            .map_err(|_| AppError::internal("invalid 2FA nonce"))?;
        let plain = self
            .key()?
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(user_id.to_be_bytes()),
                &mut bytes[12..],
            )
            .map_err(|_| AppError::internal("2FA decryption failed"))?;
        Ok(plain.to_vec())
    }
}

pub fn random_bytes<const N: usize>() -> Result<[u8; N], AppError> {
    let mut bytes = [0; N];
    rand::fill(&mut bytes).map_err(|_| AppError::internal("secure random generation failed"))?;
    Ok(bytes)
}

pub fn authenticator(secret: Vec<u8>, username: &str) -> Result<Totp, AppError> {
    Builder::new()
        .with_secret(secret)
        .with_account_name(username.replace(':', "_"))
        .with_issuer(Some("Guardian"))
        .build()
        .map_err(|_| AppError::internal("invalid TOTP configuration"))
}

pub fn recovery_hash(code: &str) -> String {
    STANDARD.encode(digest::digest(&digest::SHA256, code.as_bytes()).as_ref())
}

pub fn new_recovery_codes() -> Result<Vec<String>, AppError> {
    (0..10)
        .map(|_| Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes::<16>()?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encryption_is_random_authenticated_and_bound_to_account() {
        let crypto = TwoFactorCrypto::new(Some(&STANDARD.encode([7; 32]))).unwrap();
        let first = crypto.encrypt(1, b"secret").unwrap();
        assert_ne!(first, crypto.encrypt(1, b"secret").unwrap());
        assert_eq!(crypto.decrypt(1, &first).unwrap(), b"secret");
        assert!(crypto.decrypt(2, &first).is_err());
        assert!(crypto.decrypt(1, "invalid").is_err());
        assert!(TwoFactorCrypto::new(Some("bad")).is_err());
        assert!(
            TwoFactorCrypto::new(None)
                .unwrap()
                .encrypt(1, b"x")
                .is_err()
        );
    }
    #[test]
    fn totp_matches_rfc_vector() {
        let totp = Builder::new()
            .with_secret(b"12345678901234567890".to_vec())
            .with_digits(8)
            .build()
            .unwrap();
        assert_eq!(totp.generate(59).to_string(), "94287082");
        assert_eq!(totp.check("94287082", 59), Some(1));
        assert!(totp.check("00000000", 59).is_none());
    }
}
