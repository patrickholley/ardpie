use sqlx::{PgPool, Error};

#[derive(Debug)]
pub struct Expense {
    pub id: i32,
    pub date: String,
    pub description: String,
    pub cost: f64,
    pub budget_id: i32,
}

pub struct ExpenseRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> ExpenseRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> Result<Vec<Expense>, Error> {
        sqlx::query_as!(
            Expense,
            r#"
            SELECT id, date, description, cost, budget_id
            FROM expenses
            "#
        )
            .fetch_all(self.pool)
            .await
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Expense, Error> {
        sqlx::query_as!(
            Expense,
            r#"
            SELECT id, date, description, cost, budget_id
            FROM expenses
            WHERE id = $1
            "#,
            id
        )
            .fetch_one(self.pool)
            .await
    }

    pub async fn create(&self, date: String, description: String, cost: f64, budget_id: i32) -> Result<Expense, Error> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO expenses (date, description, cost, budget_id)
            VALUES ($1, $2, $3, $4)
            RETURNING id, date, description, cost, budget_id
            "#,
            date, description, cost, budget_id
        )
            .fetch_one(self.pool)
            .await?;

        Ok(Expense {
            id: rec.id,
            date: rec.date,
            description: rec.description,
            cost: rec.cost,
            budget_id: rec.budget_id,
        })
    }

    pub async fn update(&self, id: i32, date: String, description: String, cost: f64) -> Result<Expense, Error> {
        let rec = sqlx::query!(
            r#"
            UPDATE expenses
            SET date = $1, description = $2, cost = $3
            WHERE id = $4
            RETURNING id, date, description, cost, budget_id
            "#,
            date, description, cost, id
        )
            .fetch_one(self.pool)
            .await?;

        Ok(Expense {
            id: rec.id,
            date: rec.date,
            description: rec.description,
            cost: rec.cost,
            budget_id: rec.budget_id,
        })
    }

    pub async fn delete(&self, id: i32) -> Result<u64, Error> {
        let affected_rows = sqlx::query!(
            r#"
            DELETE FROM expenses
            WHERE id = $1
            "#,
            id
        )
            .execute(self.pool)
            .await?
            .rows_affected();

        Ok(affected_rows)
    }
}
