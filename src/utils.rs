use warp::Filter;
use std::fmt;

#[derive(Debug)]
pub enum ServiceError {
    Unauthorized,
    DatabaseError(sqlx::Error),
    BadRequest(String),
    InternalServerError,
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::Unauthorized => write!(f, "Unauthorized access"),
            ServiceError::DatabaseError(_) => write!(f, "Database error occurred"),
            ServiceError::BadRequest(detail) => write!(f, "Bad request: {}", detail),
            ServiceError::InternalServerError => write!(f, "Internal server error"),
        }
    }
}

impl warp::reject::Reject for ServiceError {}

pub fn with_db(pool: sqlx::PgPool) -> impl Filter<Extract = (sqlx::PgPool,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || pool.clone())
}


pub fn json_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
where
    T: Send + serde::de::DeserializeOwned + 'static,
{
    warp::body::json()
}

pub async fn user_owns_budget<E>(
    user_id: i32,
    budget_id: i32,
    pool: &sqlx::PgPool,
    error: E
) -> Result<bool, warp::Rejection>
where
    E: warp::reject::Reject + Send + Sync + 'static,
{
    let result = sqlx::query!(
        "SELECT 1 as exists FROM user_budgets WHERE userid = $1 AND budgetid = $2",
        user_id,
        budget_id
    )
        .fetch_optional(pool)
        .await
        .map_err(|_| warp::reject::custom(error))?;

    Ok(result.is_some())
}
