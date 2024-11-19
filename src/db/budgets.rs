use warp::Filter;
use serde::{Serialize, Deserialize};
use sqlx::postgres::PgPoolOptions;
use crate::utils::{json_body, with_db};

#[derive(Serialize, Deserialize, Debug)]
struct Budget {
    id: i32,
    name: String,
}

#[derive(Deserialize, Debug)]
struct NewBudget {
    name: String,
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
        let budgets = sqlx::query_as!(Budget, "SELECT id, name FROM budgets")
            .fetch_all(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&budgets))
    }

    async fn handle_get_budget_with_id(id: i32, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let budget = sqlx::query_as!(Budget, "SELECT id, name FROM budgets WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&budget))
    }

    async fn handle_create_budget(new_budget: NewBudget, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let budget = sqlx::query_as!(
            Budget,
            "INSERT INTO budgets (name) VALUES ($1) RETURNING id, name",
            new_budget.name
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&budget))
    }

    async fn handle_update_budget(id: i32, new_budget: NewBudget, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let budget = sqlx::query_as!(
            Budget,
            "UPDATE budgets SET name = $1 WHERE id = $2 RETURNING id, name",
            new_budget.name,
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&budget))
    }

    async fn handle_delete_budget(id: i32, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        sqlx::query!(
        "DELETE FROM budgets WHERE id = $1",
        id
    )
            .execute(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&format!("Budget with id {} deleted", id)))
    }
}
