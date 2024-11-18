use sqlx::{PgPool, Error};

#[derive(Debug)]
pub struct Budget {
    pub id: i32,
    pub name: String,
}

pub struct BudgetRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> BudgetRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> Result<Vec<Budget>, Error> {
        sqlx::query_as!(
            Budget,
            r#"
            SELECT id, name
            FROM budgets
            "#
        )
            .fetch_all(self.pool)
            .await
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Budget, Error> {
        sqlx::query_as!(
            Budget,
            r#"
            SELECT id, name
            FROM budgets
            WHERE id = $1
            "#,
            id
        )
            .fetch_one(self.pool)
            .await
    }

    pub async fn create(&self, name: String) -> Result<Budget, Error> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO budgets (name)
            VALUES ($1)
            RETURNING id, name
            "#,
            name
        )
            .fetch_one(self.pool)
            .await?;

        Ok(Budget {
            id: rec.id,
            name: rec.name,
        })
    }

    pub async fn update(&self, id: i32, name: String) -> Result<Budget, Error> {
        let rec = sqlx::query!(
            r#"
            UPDATE budgets
            SET name = $1
            WHERE id = $2
            RETURNING id, name
            "#,
            name, id
        )
            .fetch_one(self.pool)
            .await?;

        Ok(Budget {
            id: rec.id,
            name: rec.name,
        })
    }

    pub async fn delete(&self, id: i32) -> Result<u64, Error> {
        let affected_rows = sqlx::query!(
            r#"
            DELETE FROM budgets
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
