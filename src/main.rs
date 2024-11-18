mod db;

use dotenv::dotenv;
use db::budgets;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let budget_service = budgets::BudgetService::new(&database_url).await;
    let routes = budget_service.routes();

    warp::serve(routes)
        .run(([127, 0, 0, 1], 2345))
        .await;
}