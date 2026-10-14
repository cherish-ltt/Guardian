use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};

use crate::{
    domain::{
        error::AppError,
        models::auth::{AccessClaims, AccessToken},
    },
    infrastructure::config::JwtConfig,
};

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    access_token_ttl_minutes: i64,
}

impl JwtService {
    pub fn new(config: JwtConfig) -> Result<Self> {
        if config.secret.len() < 32 {
            return Err(anyhow!("JWT_SECRET must be at least 32 characters long"));
        }

        Ok(Self {
            encoding_key: EncodingKey::from_secret(config.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
            issuer: config.issuer,
            audience: config.audience,
            access_token_ttl_minutes: config.access_token_ttl_minutes,
        })
    }

    pub fn issue_access_token(
        &self,
        user_id: &str,
        username: &str,
        roles: &[String],
    ) -> Result<AccessToken> {
        let now = Utc::now();
        let expires_at = now + Duration::minutes(self.access_token_ttl_minutes);
        let claims = AccessClaims {
            sub: user_id.to_string(),
            username: username.to_string(),
            roles: roles.to_vec(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now.timestamp() as usize,
            exp: expires_at.timestamp() as usize,
        };

        let access_token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)?;

        Ok(AccessToken {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in_seconds: self.access_token_ttl_minutes * 60,
        })
    }

    pub fn verify_access_token(&self, token: &str) -> Result<AccessClaims, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[self.audience.as_str()]);
        validation.set_issuer(&[self.issuer.as_str()]);

        decode::<AccessClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| AppError::unauthorized("access token is invalid or expired"))
    }
}

#[cfg(test)]
mod tests {
    use super::JwtService;
    use crate::{domain::error::AppError, infrastructure::config::JwtConfig};

    const TEST_SECRET: &str = "jwt-unit-test-secret-that-is-long-enough";

    fn config(secret: &str, ttl_minutes: i64) -> JwtConfig {
        JwtConfig {
            secret: secret.to_string(),
            issuer: "test-issuer".to_string(),
            audience: "test-audience".to_string(),
            access_token_ttl_minutes: ttl_minutes,
        }
    }

    #[test]
    fn short_secret_is_rejected() {
        assert!(JwtService::new(config("short", 120)).is_err());
    }

    #[test]
    fn issued_token_verifies_roundtrip() {
        let service = JwtService::new(config(TEST_SECRET, 120)).expect("构建 JWT 服务");
        let token = service
            .issue_access_token("user-1", "alice", &["admin".to_string()])
            .expect("签发令牌");

        assert_eq!(token.token_type, "Bearer");
        assert_eq!(token.expires_in_seconds, 120 * 60);

        let claims = service
            .verify_access_token(&token.access_token)
            .expect("校验令牌");
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.roles, vec!["admin".to_string()]);
        assert_eq!(claims.iss, "test-issuer");
        assert_eq!(claims.aud, "test-audience");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn invalid_token_is_unauthorized() {
        let service = JwtService::new(config(TEST_SECRET, 120)).expect("构建 JWT 服务");

        let error = service
            .verify_access_token("not-a-jwt")
            .expect_err("无效令牌应被拒绝");
        assert!(matches!(error, AppError::Unauthorized(_)));
    }

    #[test]
    fn token_signed_with_other_secret_is_rejected() {
        let service = JwtService::new(config(TEST_SECRET, 120)).expect("构建 JWT 服务");
        let other = JwtService::new(config("another-secret-that-is-long-enough!!!", 120))
            .expect("构建另一个 JWT 服务");
        let token = other
            .issue_access_token("user-1", "alice", &[])
            .expect("签发令牌");

        assert!(service.verify_access_token(&token.access_token).is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        // 负 TTL 生成已过期令牌（jsonwebtoken 默认 60 秒 leeway，因此回退 5 分钟）。
        let service = JwtService::new(config(TEST_SECRET, -5)).expect("构建 JWT 服务");
        let token = service
            .issue_access_token("user-1", "alice", &[])
            .expect("签发令牌");

        let error = service
            .verify_access_token(&token.access_token)
            .expect_err("过期令牌应被拒绝");
        assert!(matches!(error, AppError::Unauthorized(_)));
    }
}
