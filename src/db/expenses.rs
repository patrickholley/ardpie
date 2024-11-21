use warp::Filter;
use sqlx::postgres::PgPoolOptions;
use crate::utils::{json_body, with_db};
use serde::{Deserialize, Serialize};
use bigdecimal::BigDecimal;
use time::Date;

#[derive(Deserialize, Debug)]
struct BudgetIdQuery {
    budgetid: i32,
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
            .and(warp::get())
            .and(warp::query::<BudgetIdQuery>())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_expenses_total);

        let get_expenses = warp::path("expenses")
            .and(warp::get())
            .and(warp::query::<BudgetIdQuery>())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_expenses);

        let get_expense = warp::path!("expenses" / i32)
            .and(warp::get())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_get_expense);

        let create_expense = warp::path("expenses")
            .and(warp::post())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_create_expense);

        let update_expense = warp::path!("expenses" / i32)
            .and(warp::put())
            .and(json_body())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_update_expense);

        let delete_expense = warp::path!("expenses" / i32)
            .and(warp::delete())
            .and(with_db(pool.clone()))
            .and_then(Self::handle_delete_expense);

        get_expenses_total
            .or(get_expenses)
            .or(get_expense)
            .or(create_expense)
            .or(update_expense)
            .or(delete_expense)
    }

    async fn handle_get_expenses_total(query: BudgetIdQuery, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let result = sqlx::query!("SELECT COALESCE(SUM(amount), 0) as total FROM expenses WHERE budgetid = $1", query.budgetid)
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        let total = result.total.unwrap_or_else(|| BigDecimal::from(0));

        Ok(warp::reply::json(&total))
    }

    async fn handle_get_expenses(query: BudgetIdQuery, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let expenses = sqlx::query_as!(Expense, "SELECT * FROM expenses WHERE budgetid = $1", query.budgetid)
            .fetch_all(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&expenses))
    }

    async fn handle_get_expense(id: i32, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        let expense = sqlx::query_as!(Expense, "SELECT * FROM expenses WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&expense))
    }

    async fn handle_create_expense(new_expense: NewExpense, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
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
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&expense))
    }

    async fn handle_update_expense(id: i32, new_expense: NewExpense, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
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
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&expense))
    }

    async fn handle_delete_expense(id: i32, pool: sqlx::PgPool) -> Result<impl warp::Reply, warp::Rejection> {
        sqlx::query!("DELETE FROM expenses WHERE id = $1", id)
            .execute(&pool)
            .await
            .map_err(|_| warp::reject())?;

        Ok(warp::reply::json(&format!("Expense with id {} deleted", id)))
    }
}