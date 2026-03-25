use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[allow(clippy::cast_possible_truncation)]
pub fn create_token(sub: &str, secret: &[u8]) -> jsonwebtoken::errors::Result<String> {
    let iat = chrono::Utc::now().timestamp() as usize;
    let exp = iat + (24 * 3600); // 1 day expiry
    let claims = Claims {
        sub: sub.to_string(),
        exp,
        iat,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))
}

pub fn verify_token(token: &str, secret: &[u8]) -> jsonwebtoken::errors::Result<Claims> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation).map(|data| data.claims)
}
