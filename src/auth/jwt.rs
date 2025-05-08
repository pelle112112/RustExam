use chrono::{Duration, Utc};
use jsonwebtoken::{self, DecodingKey, EncodingKey, Header, Validation};
use poem_grants::error::AccessError::UnauthorizedRequest;
use serde::{Deserialize, Serialize};

const JWT_EXPIRATION_HOURS: i64 = 24;
const SECRET: &str = "totallySecureMegaHDPassword";

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub username: String,
    pub permissions: Vec<String>,
    pub exp: i64,
}

impl Claims {
    pub fn new(username: String, permissions: Vec<String>) -> Self {
        Self {
            username,
            permissions,
            exp: (Utc::now() + Duration::try_hours(JWT_EXPIRATION_HOURS).unwrap()).timestamp(),
        }
    }
}

pub fn create_jwt(claims: Claims) -> poem::Result<String> {
    let encoding_key = EncodingKey::from_secret(SECRET.as_bytes());
    let result = jsonwebtoken::encode(&Header::default(), &claims, &encoding_key);

    match result {
        Ok(token) => Ok(token),
        Err(_err) => Err(UnauthorizedRequest.into())
    }
}

pub fn decode_jwt(token: &str) -> poem::Result<Claims>{
    let decoding_key = DecodingKey::from_secret(SECRET.as_bytes());
    jsonwebtoken::decode::<Claims>(token, &decoding_key, &Validation::default());
    let result = jsonwebtoken::decode::<Claims>(token, &decoding_key, &Validation::default());

    match result {
        Ok(token_data) => {
            Ok(token_data.claims)
        }
        Err(_err) => {
            Err(UnauthorizedRequest.into())
        }
    }
}