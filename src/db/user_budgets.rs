use warp::{Filter, http::StatusCode};
use sqlx::postgres::PgPoolOptions;
use crate::utils::{json_body, with_db};
use serde::{Deserialize, Serialize};
use crate::auth::{with_auth, Claims};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug)]
struct UserBudgetAssociation {
    userid: i32,
    budgetid: i32,
}

#[derive(Debug)]
enum MyError {
    InsertFailed,
    DeleteFailed,
    Unauthorized,
}

impl warp::reject::Reject for MyError {}

pub struct UserBudgetService {
    pool: sqlx::PgPool,
}

impl UserBudgetService {
    pub async fn new(database_url: &str) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .expect("Failed to create pool");

        UserBudgetService { pool }
    }

    pub fn routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let pool = self.pool.clone();

        let add_association = warp::path("user_budgets")
            .and(warp::post())
            .and(with_auth())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_add_association)
            .with(warp::log("api::add_association"));

        let remove_association = warp::path("user_budgets")
            .and(warp::delete())
            .and(with_auth())
            .and(warp::query::<UserBudgetAssociation>())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_remove_association)
            .with(warp::log("api::remove_association"));

        add_association.or(remove_association)
    }

    async fn handle_add_association(claims: Claims, association: UserBudgetAssociation, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        if !Self::user_owns_budget(claims.user_id, association.budgetid, &pool).await? {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        match sqlx::query!(
            "INSERT INTO user_budgets (userid, budgetid) VALUES ($1, $2)",
            association.userid,
            association.budgetid
        )
            .execute(&pool)
            .await {
            Ok(_) => Ok(warp::reply::with_status(
                warp::reply::json(&format!(
                    "Associated user {} with budget {}",
                    association.userid, association.budgetid
                )),
                StatusCode::CREATED,
            )),
            Err(e) => {
                eprintln!("Failed to insert association: {:?}", e);
                Err(warp::reject::custom(MyError::InsertFailed))
            },
        }
    }

    async fn handle_remove_association(claims: Claims, query: UserBudgetAssociation, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        if !Self::user_owns_budget(claims.user_id, query.budgetid, &pool).await? {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        match sqlx::query!(
            "DELETE FROM user_budgets WHERE userid = $1 AND budgetid = $2",
            query.userid,
            query.budgetid
        )
            .execute(&pool)
            .await {
            Ok(_) => Ok(warp::reply::with_status(
                warp::reply::json(&format!(
                    "Removed association of user {} with budget {}",
                    query.userid, query.budgetid
                )),
                StatusCode::OK,
            )),
            Err(e) => {
                eprintln!("Failed to delete association: {:?}", e);
                Err(warp::reject::custom(MyError::DeleteFailed))
            },
        }
    }

    async fn user_owns_budget(user_id: i32, budget_id: i32, pool: &sqlx::PgPool) -> Result<bool, warp::Rejection> {
        let result = sqlx::query!(
            "SELECT 1 as exists FROM user_budgets WHERE userid = $1 AND budgetid = $2",
            user_id,
            budget_id
        )
            .fetch_optional(pool)
            .await
            .map_err(|_| warp::reject::custom(MyError::Unauthorized))?;

        Ok(result.is_some())
    }
}
