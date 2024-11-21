use warp::Filter;
use sqlx::postgres::PgPoolOptions;
use crate::utils::{json_body, with_db};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct UserBudgetAssociation {
    userid: i32,
    budgetid: i32,
}

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
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_add_association)
            .with(warp::log("api::add_association"));

        let remove_association = warp::path("user_budgets")
            .and(warp::delete())
            .and(warp::query::<UserBudgetAssociation>())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_remove_association)
            .with(warp::log("api::remove_association"));

        add_association.or(remove_association)
    }

    async fn handle_add_association(association: UserBudgetAssociation, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        match sqlx::query!(
            "INSERT INTO user_budgets (userid, budgetid) VALUES ($1, $2)",
            association.userid,
            association.budgetid
        )
            .execute(&pool)
            .await {
            Ok(_) => Ok(warp::reply::json(&format!(
                "Associated user {} with budget {}",
                association.userid, association.budgetid
            ))),
            Err(e) => {
                eprintln!("Failed to insert association: {:?}", e);
                Err(warp::reject::custom(MyError::InsertFailed))
            },
        }
    }

    async fn handle_remove_association(query: UserBudgetAssociation, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        match sqlx::query!(
            "DELETE FROM user_budgets WHERE userid = $1 AND budgetid = $2",
            query.userid,
            query.budgetid
        )
            .execute(&pool)
            .await {
            Ok(_) => Ok(warp::reply::json(&format!(
                "Removed association of user {} with budget {}",
                query.userid, query.budgetid
            ))),
            Err(e) => {
                eprintln!("Failed to delete association: {:?}", e);
                Err(warp::reject::custom(MyError::DeleteFailed))
            },
        }
    }
}

// Custom error for more informative responses
#[derive(Debug)]
enum MyError {
    InsertFailed,
    DeleteFailed,
}

impl warp::reject::Reject for MyError {}
