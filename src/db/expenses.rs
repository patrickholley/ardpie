use warp::{Filter, http::StatusCode};
use sqlx::postgres::PgPoolOptions;
use crate::utils::{json_body, with_db, user_owns_budget, ServiceError};
use serde::{Deserialize, Serialize};
use bigdecimal::BigDecimal;
use time::Date;
use crate::auth::{with_auth, Claims};
use serde_json::json;

#[derive(Deserialize, Debug)]
struct BudgetIdQuery {
    budgetid: i32,
}

#[derive(Deserialize, Debug)]
struct GetExpenseQuery {
    budgetid: i32,
    start_date: Date,
    end_date: Option<Date>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Expense {
    id: i32,
    budgetid: i32,
    date: Date,
    description: String,
    amount: BigDecimal,
}

#[derive(Deserialize, Debug)]
struct NewExpense {
    budgetid: i32,
    date: Date,
    description: String,
    amount: BigDecimal,
}

pub struct ExpenseService {
    pool: sqlx::PgPool,
}

impl ExpenseService {
    pub async fn new(database_url: &str) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .expect("Failed to create pool");

        ExpenseService { pool }
    }

    pub fn routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let pool = self.pool.clone();

        let get_expenses_total = warp::path!("expenses" / "total")
            .and(warp::query::<BudgetIdQuery>())
            .and(with_auth())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_expenses_total);

        let get_expenses = warp::path("expenses")
            .and(warp::get())
            .and(with_auth())
            .and(warp::query::<GetExpenseQuery>())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_expenses);

        let get_expense = warp::path!("expenses" / i32)
            .and(warp::get())
            .and(with_auth())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_expense);

        let create_expense = warp::path("expenses")
            .and(warp::post())
            .and(with_auth())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_create_expense);

        let update_expense = warp::path!("expenses" / i32)
            .and(warp::put())
            .and(with_auth())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_update_expense);

        let delete_expense = warp::path!("expenses" / i32)
            .and(warp::delete())
            .and(with_auth())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_delete_expense);

        get_expenses_total
            .or(get_expenses)
            .or(get_expense)
            .or(create_expense)
            .or(update_expense)
            .or(delete_expense)
    }

    async fn handle_get_expenses_total(
        query: BudgetIdQuery,
        claims: Claims,
        pool: sqlx::PgPool
    ) -> Result<impl warp::Reply, warp::Rejection> {
        if !user_owns_budget(claims.user_id, query.budgetid, &pool, ServiceError::Unauthorized).await? {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let result = sqlx::query!("SELECT COALESCE(SUM(amount), 0) as total FROM expenses WHERE budgetid = $1", query.budgetid)
            .fetch_one(&pool)
            .await
            .map_err(|e| warp::reject::custom(ServiceError::DatabaseError(e)))?;

        let total: BigDecimal = result.total.unwrap_or_else(|| BigDecimal::from(0));

        Ok(warp::reply::with_status(warp::reply::json(&total), StatusCode::OK))
    }

    async fn handle_get_expenses(claims: Claims, query: GetExpenseQuery, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        if !user_owns_budget(claims.user_id, query.budgetid, &pool, ServiceError::Unauthorized).await? {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let expenses = sqlx::query_as!(
                Expense,
                r#"
                SELECT * FROM expenses
                WHERE budgetid = $1
                  AND date >= $2
                  AND ($3::DATE IS NULL OR date <= $3)
                ORDER BY date DESC
                "#,
                query.budgetid,
                query.start_date,
                query.end_date
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| warp::reject::custom(ServiceError::DatabaseError(e)))?;

        Ok(warp::reply::with_status(warp::reply::json(&expenses), StatusCode::OK))
    }

    async fn handle_get_expense(id: i32, claims: Claims, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let expense = sqlx::query_as!(Expense, "SELECT * FROM expenses WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .map_err(|e| warp::reject::custom(ServiceError::DatabaseError(e)))?;

        if !user_owns_budget(claims.user_id, expense.budgetid, &pool, ServiceError::Unauthorized).await? {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        Ok(warp::reply::with_status(warp::reply::json(&expense), StatusCode::OK))
    }

    async fn handle_create_expense(claims: Claims, new_expense: NewExpense, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        if !user_owns_budget(claims.user_id, new_expense.budgetid, &pool, ServiceError::Unauthorized).await? {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let expense = sqlx::query_as!(
            Expense,
            "INSERT INTO expenses (budgetid, date, description, amount) VALUES ($1, $2, $3, $4) RETURNING id, budgetid, date, description, amount",
            new_expense.budgetid,
            new_expense.date,
            new_expense.description,
            new_expense.amount
        )
            .fetch_one(&pool)
            .await
            .map_err(|e| warp::reject::custom(ServiceError::DatabaseError(e)))?;

        Ok(warp::reply::with_status(warp::reply::json(&expense), StatusCode::CREATED))
    }

    async fn handle_update_expense(id: i32, claims: Claims, new_expense: NewExpense, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        if !user_owns_budget(claims.user_id, new_expense.budgetid, &pool, ServiceError::Unauthorized).await? {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        let expense = sqlx::query_as!(
            Expense,
            "UPDATE expenses SET budgetid = $1, date = $2, description = $3, amount = $4 WHERE id = $5 RETURNING id, budgetid, date, description, amount",
            new_expense.budgetid,
            new_expense.date,
            new_expense.description,
            new_expense.amount,
            id
        )
            .fetch_one(&pool)
            .await
            .map_err(|e| warp::reject::custom(ServiceError::DatabaseError(e)))?;

        Ok(warp::reply::with_status(warp::reply::json(&expense), StatusCode::OK))
    }

    async fn handle_delete_expense(id: i32, claims: Claims, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let expense = sqlx::query_as!(Expense, "SELECT * FROM expenses WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .map_err(|e| warp::reject::custom(ServiceError::DatabaseError(e)))?;

        if !user_owns_budget(claims.user_id, expense.budgetid, &pool, ServiceError::Unauthorized).await? {
            return Ok(warp::reply::with_status(
                warp::reply::json(&json!({"error": "Unauthorized"})),
                StatusCode::UNAUTHORIZED,
            ));
        }

        sqlx::query!("DELETE FROM expenses WHERE id = $1", id)
            .execute(&pool)
            .await
            .map_err(|e| warp::reject::custom(ServiceError::DatabaseError(e)))?;

        Ok(warp::reply::with_status(warp::reply::json(&format!("Expense with id {} deleted", id)), StatusCode::OK))
    }


}
