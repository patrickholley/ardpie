use warp::Filter;
use sqlx::postgres::PgPoolOptions;
use crate::utils::{json_body, with_db};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: i32,
    name: String,
}

#[derive(Deserialize, Debug)]
struct NewUser {
    name: String,
}

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

        let get_user = warp::path!("users" / String)
            .and(warp::get())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_user);

        let create_user = warp::path("users")
            .and(warp::post())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_create_user);

        let update_user = warp::path!("users" / i32)
            .and(warp::put())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_update_user);

        let delete_user = warp::path!("users" / i32)
            .and(warp::delete())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_delete_user);

        get_user
            .or(create_user)
            .or(update_user)
            .or(delete_user)
    }

    async fn handle_get_user(name: String, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let user = sqlx::query_as!(User, "SELECT * FROM users WHERE name = $1", name)
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&user))
    }

    async fn handle_create_user(new_user: NewUser, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let user = sqlx::query_as!(
            User,
            "INSERT INTO users (name) VALUES ($1) RETURNING id, name",
            new_user.name
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&user))
    }

    async fn handle_update_user(id: i32, new_user: NewUser, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let user = sqlx::query_as!(
            User,
            "UPDATE users SET name = $1 WHERE id = $2 RETURNING id, name",
            new_user.name,
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&user))
    }

    async fn handle_delete_user(id: i32, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        // Step 1: Fetch all budget IDs associated with the user from the user_budgets table
        let budgetids: Vec<i32> = sqlx::query!("SELECT budgetid FROM user_budgets WHERE userid = $1", id)
            .fetch_all(&pool)
            .await
            .map_err(|_| warp::reject())?
            .into_iter()
            .map(|record| record.budgetid)
            .collect();

        // Step 2: Delete these budget entries from the budgets table
        for budgetid in budgetids.iter() {
            sqlx::query!("DELETE FROM budgets WHERE id = $1", budgetid)
                .execute(&pool)
                .await
                .map_err(|_|  warp::reject())?;
        }

        // Step 3: Delete entries from the user_budgets table
        sqlx::query!("DELETE FROM user_budgets WHERE userid = $1", id)
            .execute(&pool)
            .await
            .map_err(|_| warp::reject())?;

        // Step 4: Delete the user from the users table
        sqlx::query!("DELETE FROM users WHERE id = $1", id)
            .execute(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&format!("User with id {} deleted", id)))
    }
}
