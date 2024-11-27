use jsonwebtoken::{DecodingKey, Validation};
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
            match jsonwebtoken::decode::<Claims>(&token, &DecodingKey::from_secret("secret".as_ref()), &Validation::default()) {
                Ok(c) => Ok(c.claims),
                Err(_) => Err(reject::custom(AuthError)),
            }
        })
}

#[derive(Debug)]
pub struct AuthError;

impl reject::Reject for AuthError {}
