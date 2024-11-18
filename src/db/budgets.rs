use warp::Filter;
use serde::{Serialize, Deserialize};
use sqlx::postgres::PgPoolOptions;

#[derive(Serialize, Deserialize, Debug)]
struct Budget {
    id: i32,
    userid: String,
    name: String,
    addl_users: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct NewBudget {
    userid: String,
    name: String,
    addl_users: Option<Vec<String>>,
}

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
            .and(warp::path::end())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_budgets);

        let get_budget_with_id = warp::path!("budgets" / i32)
            .and(warp::get())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_budget_with_id);

        let create_budget = warp::path("budgets")
            .and(warp::post())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_create_budget);

        let update_budget = warp::path!("budgets" / i32)
            .and(warp::put())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_update_budget);

        let delete_budget = warp::path!("budgets" / i32)
            .and(warp::delete())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_delete_budget);

        get_budgets
            .or(get_budget_with_id)
            .or(create_budget)
            .or(update_budget)
            .or(delete_budget)
    }

    async fn handle_get_budgets(pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let budgets = sqlx::query_as!(Budget, "SELECT * FROM budgets")
            .fetch_all(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&budgets))
    }

    async fn handle_get_budget_with_id(id: i32, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let budget = sqlx::query_as!(Budget, "SELECT * FROM budgets WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&budget))
    }

    async fn handle_create_budget(new_budget: NewBudget, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let budget = sqlx::query_as!(
            Budget,
            "INSERT INTO budgets (userid, name, addl_users) VALUES ($1, $2, $3) RETURNING id, userid, name, addl_users",
            new_budget.userid,
            new_budget.name,
            new_budget.addl_users.as_deref()
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&budget))
    }

    async fn handle_update_budget(id: i32, new_budget: NewBudget, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let budget = sqlx::query_as!(
            Budget,
            "UPDATE budgets SET userid = $1, name = $2, addl_users = $3 WHERE id = $4 RETURNING id, userid, name, addl_users",
            new_budget.userid,
            new_budget.name,
            new_budget.addl_users.as_deref(),
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&budget))
    }

    async fn handle_delete_budget(id: i32, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        sqlx::query!("DELETE FROM budgets WHERE id = $1", id)
            .execute(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&format!("Budget with id {} deleted", id)))
    }
}

fn with_db(pool: sqlx::PgPool) -> impl Filter<Extract = (sqlx::PgPool,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

fn json_body() -> impl Filter<Extract = (NewBudget,), Error = warp::Rejection> + Clone {
    warp::body::json()
}
