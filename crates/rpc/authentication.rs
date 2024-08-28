use crate::RpcErr;
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use bytes::Bytes;
use jsonwebtoken::{decode, Algorithm, DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum AuthenticationError {
    InvalidIssuedAtClaim,
    TokenDecodingError,
    MissingAuthentication,
}

pub fn authenticate(
    secret: Bytes,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<(), RpcErr> {
    if auth_header.is_none() {
        return Err(RpcErr::AuthenticationError(
            AuthenticationError::MissingAuthentication,
        ));
    }
    let TypedHeader(auth_header) = auth_header.unwrap();
    let token = auth_header.token();
    validate_jwt_authentication(token, secret)
        .map_err(|auth_err| RpcErr::AuthenticationError(auth_err))
}

// JWT claims struct
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iat: usize,
    id: Option<String>,
    clv: Option<String>,
}

/// Authenticates bearer jwt to check that authrpc calls are sent by the consensus layer
pub fn validate_jwt_authentication(token: &str, secret: Bytes) -> Result<(), AuthenticationError> {
    let decoding_key = DecodingKey::from_secret(&secret);
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;
    validation.set_required_spec_claims(&["iat"]);
    match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            if invalid_issued_at_claim(token_data) {
                Err(AuthenticationError::InvalidIssuedAtClaim)
            } else {
                Ok(())
            }
        }
        Err(_) => Err(AuthenticationError::TokenDecodingError),
    }
}

/// Checks that the "iat" timestamp in the claim is less than 60 seconds from now
fn invalid_issued_at_claim(token_data: TokenData<Claims>) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    (now as isize - token_data.claims.iat as isize).abs() > 60
}
