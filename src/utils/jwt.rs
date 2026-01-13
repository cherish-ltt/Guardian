use std::time::Duration;

use anyhow::Result;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum TokenType {
    #[default]
    Access,
    Refresh,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub jti: String,
    pub sub: String,
    pub username: String,
    pub is_super_admin: bool,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Serialize, Deserialize, Default)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct AccessToken {
    pub access_token: String,
    pub expires_in: i64,
}

impl TokenType {
    fn duration(&self) -> Duration {
        match self {
            TokenType::Access => Duration::from_secs(15 * 60),
            TokenType::Refresh => Duration::from_secs(7 * 24 * 60 * 60),
        }
    }
}

fn get_secret() -> String {
    std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "ASDFGHJKLZXCVBNMQWERTYUIOP1234567890".to_string())
}

pub fn create_token_pair(
    admin_id: Uuid,
    username: String,
    is_super_admin: bool,
) -> Result<TokenPair> {
    let jti = Uuid::new_v4().to_string();

    let access_claims = Claims {
        jti: jti.clone(),
        sub: admin_id.to_string(),
        username: username.clone(),
        is_super_admin,
        exp: (chrono::Utc::now() + TokenType::Access.duration()).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(get_secret().as_ref()),
    )?;

    let refresh_jti = Uuid::new_v4().to_string();
    let refresh_claims = Claims {
        jti: refresh_jti,
        sub: admin_id.to_string(),
        username,
        is_super_admin,
        exp: (chrono::Utc::now() + TokenType::Refresh.duration()).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
    };

    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(get_secret().as_ref()),
    )?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        expires_in: 15 * 60,
    })
}

pub fn refresh_access_token(refresh_token: &str) -> Result<AccessToken> {
    let token_data = decode::<Claims>(
        refresh_token,
        &DecodingKey::from_secret(get_secret().as_ref()),
        &Validation::default(),
    )?;

    let claims = token_data.claims;

    let access_claims = Claims {
        jti: Uuid::new_v4().to_string(),
        sub: claims.sub.clone(),
        username: claims.username.clone(),
        is_super_admin: claims.is_super_admin,
        exp: (chrono::Utc::now() + TokenType::Access.duration()).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(get_secret().as_ref()),
    )?;

    Ok(AccessToken {
        access_token,
        expires_in: 15 * 60,
    })
}

pub fn verify_token(token: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(get_secret().as_ref()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn get_jti(token: &str) -> Result<String> {
    let claims = verify_token(token)?;
    Ok(claims.jti)
}
