use std::fmt;
use warp::{Rejection, reject, Filter};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, DecodingKey, Validation, errors::ErrorKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i32,
    pub exp: usize,
}

#[derive(Debug)]
enum AuthError {
    MissingToken,
    InvalidToken,
    ExpiredToken,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Authorization token is missing"),
            AuthError::InvalidToken => write!(f, "Authorization token is invalid"),
            AuthError::ExpiredToken => write!(f, "Authorization token is expired"),
        }
    }
}

impl reject::Reject for AuthError {}

pub fn with_auth() -> impl Filter<Extract = (Claims,), Error = Rejection> + Clone {
    warp::header::optional::<String>("authorization")
        .and_then(|authorization: Option<String>| async move {
            let token = match authorization {
                Some(token) => token.replace("Bearer ", ""),
                None => return Err(reject::custom(AuthError::MissingToken)),
            };

            let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret".to_string());

            match decode::<Claims>(&token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default()) {
                Ok(data) => Ok(data.claims),
                Err(err) => match *err.kind() {
                    ErrorKind::ExpiredSignature => Err(reject::custom(AuthError::ExpiredToken)),
                    _ => Err(reject::custom(AuthError::InvalidToken)),
                },
            }
        })
}
