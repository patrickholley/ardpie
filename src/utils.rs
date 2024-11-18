use warp::Filter;

pub fn with_db(pool: sqlx::PgPool) -> impl Filter<Extract = (sqlx::PgPool,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

pub fn json_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
where
    T: Send + serde::de::DeserializeOwned + 'static,
{
    warp::body::json()
}
