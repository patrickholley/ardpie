use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use warp::{Filter, http::StatusCode};
use crate::utils::{json_body, with_db};
use crate::auth::{with_auth, Claims};

#[derive(Serialize, Deserialize, Debug)]
struct Budget {
    id: i32,
    name: String,
    settings: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct NewBudget {
    name: String,
    settings: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct UserIdQuery {
    userid: i32,
}

#[derive(Debug)]
struct MyError;

impl warp::reject::Reject for MyError {}

pub struct BudgetService {
    pool: sqlx::PgPool,
}

impl BudgetService {
    pub async fn new(database_url: &str) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .expect("Failed to create pool");

        BudgetService { pool }
    }

    pub fn routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let pool = self.pool.clone();
        let get_budgets = warp::path("budgets")
            .and(warp::get())
            .and(with_auth())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_budgets);

        let get_budget = warp::path!("budgets" / i32)
            .and(warp::get())
            .and(with_auth())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_budget);

        let create_budget = warp::path("budgets")
            .and(warp::post())
            .and(json_body())
            .and(warp::query::<UserIdQuery>())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_create_budget);

        let update_budget = warp::path!("budgets" / i32)
            .and(warp::put())
            .and(with_auth())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_update_budget);

        let delete_budget = warp::path!("budgets" / i32)
            .and(warp::delete())
            .and(with_auth())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_delete_budget);

        get_budgets
            .or(get_budget)
            .or(create_budget)
            .or(update_budget)
            .or(delete_budget)
    }

    async fn handle_get_budgets(claims: Claims, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let budgets = sqlx::query_as!(
            Budget,
            "SELECT b.id, b.name, b.settings
             FROM budgets b
             JOIN user_budgets ub ON b.id = ub.budgetid
             WHERE ub.userid = $1",
            claims.user_id
        )
            .fetch_all(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        Ok(warp::reply::with_status(warp::reply::json(&budgets), StatusCode::OK))
    }

    async fn handle_get_budget(id: i32, claims: Claims, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let user_budget = sqlx::query!(
            "SELECT userid FROM user_budgets WHERE budgetid = $1",
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        if user_budget.userid != claims.user_id {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let budget = sqlx::query_as!(
            Budget,
            "SELECT id, name, settings FROM budgets WHERE id = $1",
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        Ok(warp::reply::with_status(warp::reply::json(&budget), StatusCode::OK))
    }

    async fn handle_create_budget(new_budget: NewBudget, query: UserIdQuery, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let mut tx = pool.begin().await.map_err(|_| warp::reject::custom(MyError))?;

        let budget = sqlx::query_as!(
            Budget,
            "INSERT INTO budgets (name, settings) VALUES ($1, $2)
             RETURNING id, name, settings",
            new_budget.name,
            new_budget.settings
        )
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        sqlx::query!(
            "INSERT INTO user_budgets (userid, budgetid) VALUES ($1, $2)",
            query.userid,
            budget.id
        )
            .execute(&mut *tx)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        tx.commit().await.map_err(|_| warp::reject::custom(MyError))?;

        Ok(warp::reply::with_status(warp::reply::json(&budget), StatusCode::CREATED))
    }

    async fn handle_update_budget(id: i32, claims: Claims, new_budget: NewBudget, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let user_budget = sqlx::query!(
            "SELECT userid FROM user_budgets WHERE budgetid = $1",
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        if user_budget.userid != claims.user_id {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let budget = sqlx::query_as!(
            Budget,
            "UPDATE budgets SET name = $1, settings = $2 WHERE id = $3
             RETURNING id, name, settings",
            new_budget.name,
            new_budget.settings,
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        Ok(warp::reply::with_status(warp::reply::json(&budget), StatusCode::OK))
    }

    async fn handle_delete_budget(id: i32, claims: Claims, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let user_budget = sqlx::query!(
            "SELECT userid FROM user_budgets WHERE budgetid = $1",
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        if user_budget.userid != claims.user_id {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let mut tx = pool.begin().await.map_err(|_| warp::reject::custom(MyError))?;

        sqlx::query!("DELETE FROM expenses WHERE budgetid = $1", id)
            .execute(&mut *tx)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        sqlx::query!("DELETE FROM user_budgets WHERE budgetid = $1", id)
            .execute(&mut *tx)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        sqlx::query!("DELETE FROM budgets WHERE id = $1", id)
            .execute(&mut *tx)
            .await
            .map_err(|_| warp::reject::custom(MyError))?;

        tx.commit().await.map_err(|_| warp::reject::custom(MyError))?;

        Ok(warp::reply::with_status(warp::reply::json(&format!("Budget with id {} deleted", id)), StatusCode::OK))
    }
}
