mod db;
mod utils;

use std::env;
use dotenv::dotenv;
use db::{budgets, expenses, users, user_budgets};
use warp::Filter;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://trickyaudin@localhost:5432/ardcheese".to_string()
    });
    let auden_sylens = env::var("AUDEN_SYLENS").unwrap_or_else(|_| {
        "http://localhost".to_string()
    });

    let budget_service = budgets::BudgetService::new(&database_url).await;
    let expense_service = expenses::ExpenseService::new(&database_url).await;
    let user_service = users::UserService::new(&database_url).await;
    let user_budget_service = user_budgets::UserBudgetService::new(&database_url).await;

    let cors = warp::cors()
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allow_headers(vec!["Content-Type", "Authorization"])
        .allow_origin("https://ardfudge.ardmore.us")
        .allow_origin("http://localhost")
        .allow_origin("http://127.0.0.1")
        .allow_origin(auden_sylens.as_str());

    let routes = budget_service.routes()
        .or(expense_service.routes()
            .or(user_service.routes()
                .or(user_budget_service.routes())))
        .with(cors)
        .with(warp::log("api"));

    warp::serve(routes)
        .run(([0, 0, 0, 0], 2345))
        .await;
}
