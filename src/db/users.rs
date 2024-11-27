use warp::{Filter};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use crate::utils::{json_body, with_db};
use bcrypt::{hash, verify};
use jsonwebtoken::{encode, Header, EncodingKey};
use warp::http::StatusCode;
use std::convert::Infallible;
use std::env;
use crate::auth::{with_auth, Claims};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: i32,
    name: String,
    password: Option<String>,
}

#[derive(Deserialize, Debug)]
struct NewUser {
    name: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserResponse {
    id: i32,
    name: String,
}

#[derive(Deserialize, Debug)]
struct LoginRequest {
    name: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LoginResponse {
    id: i32,
    name: String,
    token: String,
}

#[derive(Debug)]
struct MyError;

impl warp::reject::Reject for MyError {}

pub struct UserService {
    pool: sqlx::PgPool,
}

impl UserService {
    pub async fn new(database_url: &str) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .expect("Failed to create pool");

        UserService { pool }
    }

    pub fn routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let pool = self.pool.clone();

        let create_user = warp::path("users")
            .and(warp::post())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_create_user);

        let update_user = warp::path!("users" / i32)
            .and(warp::put())
            .and(with_auth())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_update_user);

        let delete_user = warp::path!("users" / i32)
            .and(warp::delete())
            .and(with_auth())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_delete_user);

        let login = warp::path("login")
            .and(warp::post())
            .and(json_body())
            .and(with_db(pool))
            .and_then(Self::handle_login);

        create_user
            .or(update_user)
            .or(delete_user)
            .or(login)
    }

    async fn handle_create_user(new_user: NewUser, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let hashed_password = hash(&new_user.password, 4).map_err(|_| warp::reject::custom(MyError))?;

        let user = sqlx::query!(
        "INSERT INTO users (name, password) VALUES ($1, $2) RETURNING id, name",
        new_user.name,
        hashed_password
    )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        let user_response = UserResponse {
            id: user.id,
            name: user.name,
        };

        Ok(warp::reply::with_status(warp::reply::json(&user_response), StatusCode::CREATED))
    }

    async fn handle_update_user(id: i32, claims: Claims, new_user: NewUser, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        if id != claims.user_id {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let hashed_password = hash(&new_user.password, 4).map_err(|_| warp::reject::custom(MyError))?;

        let user = sqlx::query!(
        "UPDATE users SET name = $1, password = $2 WHERE id = $3 RETURNING id, name",
        new_user.name,
        hashed_password,
        id
    )
            .fetch_one(& pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        let user_response = UserResponse {
            id: user.id,
            name: user.name,
        };

        Ok(warp::reply::with_status(warp::reply::json(&user_response), StatusCode::OK))
    }

    async fn handle_delete_user(id: i32, claims: Claims, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        if id != claims.user_id {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        // Step 1: Fetch budget IDs associated with user from user_budgets table
        let budgetids: Vec<i32> = sqlx::query!("SELECT budgetid FROM user_budgets WHERE userid = $1", id)
            .fetch_all(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?
            .into_iter()
            .map(|record| record.budgetid)
            .collect();

        // Step 2: Delete budget expenses from expenses table
        for budgetid in budgetids.iter() {
            sqlx::query!("DELETE FROM expenses WHERE budgetid = $1", budgetid)
                .execute(&pool)
                .await
                .map_err(|_|  warp::reject::custom(MyError))?;
        }

        // Step 3: Delete budget from budgets table
        for budgetid in budgetids.iter() {
            sqlx::query!("DELETE FROM budgets WHERE id = $1", budgetid)
                .execute(&pool)
                .await
                .map_err(|_|  warp::reject::custom(MyError))?;
        }

        // Step 4: Delete user/budget associations from user_budgets table
        sqlx::query!("DELETE FROM user_budgets WHERE userid = $1", id)
            .execute(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        // Step 5: Delete user from users table
        sqlx::query!("DELETE FROM users WHERE id = $1", id)
            .execute(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        Ok(warp::reply::with_status(warp::reply::json(&format!("User with id {} deleted", id)), StatusCode::OK))
    }

    async fn handle_login(login: LoginRequest, pool: sqlx::PgPool) -> Result<impl warp::Reply, Infallible> {
        let result = sqlx::query!("SELECT id, name, password FROM users WHERE name = $1", login.name)
            .fetch_one(&pool)
            .await;

        match result {
            Ok(record) => {
                let hashed_password = record.password;
                if verify(&login.password, &hashed_password).is_ok() {
                    let claims = Claims {
                        user_id: record.id,
                        exp: get_expires_at(),
                    };
                    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
                    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).unwrap();

                    let login_response = LoginResponse {
                        id: record.id,
                        name: record.name,
                        token,
                    };

                    return Ok(warp::reply::with_status(warp::reply::json(&login_response), StatusCode::OK));
                }
                Ok(warp::reply::with_status(warp::reply::json(&json!({"error": "Invalid credentials"})), StatusCode::UNAUTHORIZED))
            }
            _ => Ok(warp::reply::with_status(warp::reply::json(&json!({"error": "Invalid credentials"})), StatusCode::UNAUTHORIZED))
        }
    }
}

fn get_expires_at() -> usize {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    (since_the_epoch + Duration::from_secs(60 * 60)).as_secs() as usize
}