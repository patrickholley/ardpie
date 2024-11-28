use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use warp::{header, reject, Filter, Rejection};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i32,
    pub exp: usize,
}

pub fn with_auth() -> impl Filter<Extract = (Claims,), Error = Rejection> + Clone {
    header::header::<String>("authorization")
        .and_then(|token: String| async move {
            let token = token.replace("Bearer ", "");
            let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret".to_string());
            match decode::<Claims>(&token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default()) {
                Ok(data) => Ok(data.claims),
                Err(_) => Err(reject::custom(AuthError)),
            }
        })
}

#[derive(Debug)]
pub struct AuthError;

impl reject::Reject for AuthError {}
