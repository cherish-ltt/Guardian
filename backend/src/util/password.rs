use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

use crate::error::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    if password.trim().is_empty() {
        return Err(AppError::bad_request("password must not be empty"));
    }

    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hashed| hashed.to_string())
        .map_err(|error| AppError::internal(format!("Argon2 hashing failed: {error}")))
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    if password.trim().is_empty() {
        return Err(AppError::bad_request("password must not be empty"));
    }

    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|error| AppError::bad_request(format!("invalid password hash format: {error}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};
    use crate::error::AppError;

    #[test]
    fn password_roundtrip_works() {
        let password = "S3cure-Password!";
        let hashed = hash_password(password).expect("hash password");

        assert_ne!(hashed, password);
        assert!(verify_password(password, &hashed).expect("verify password"));
        assert!(!verify_password("wrong-password", &hashed).expect("verify wrong password"));
    }

    #[test]
    fn empty_password_is_rejected_on_hash() {
        let error = hash_password("   ").expect_err("空密码应被拒绝");
        assert!(matches!(error, AppError::BadRequest(_)));
    }

    #[test]
    fn empty_password_is_rejected_on_verify() {
        let error = verify_password("", "not-a-hash").expect_err("空密码应被拒绝");
        assert!(matches!(error, AppError::BadRequest(_)));
    }

    #[test]
    fn malformed_hash_is_rejected() {
        let error = verify_password("S3cure-Password!", "not-a-argon2-hash")
            .expect_err("无效哈希格式应被拒绝");
        assert!(
            matches!(error, AppError::BadRequest(message) if message.contains("invalid password hash format"))
        );
    }
}
