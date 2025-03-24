use sqlx::SqlitePool;

use crate::{db::edr::EdrRepo, model::edr::EdrEntry};

#[derive(Clone)]
pub struct SqliteEdrRepo {
    pool: SqlitePool,
}

impl SqliteEdrRepo {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePool::connect(url).await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl EdrRepo for SqliteEdrRepo {
    async fn save(&self, edr: EdrEntry) -> anyhow::Result<()> {
        if self.fetch_by_id(&edr.transfer_id).await?.is_none() {
            self.internal_save(edr).await?;
        } else {
            self.internal_update(edr).await?;
        }
        Ok(())
    }

    async fn fetch_by_id(&self, transfer_id: &str) -> anyhow::Result<Option<EdrEntry>> {
        sqlx::query_as::<_, EdrEntry>(
            r#"
            SELECT * FROM tokens where transfer_id = $1
            "#,
        )
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await
        .map(Ok)?
    }

    async fn delete(&self, transfer_id: &str) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM tokens where transfer_id = $1
            "#,
        )
        .bind(transfer_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

impl SqliteEdrRepo {
    async fn internal_save(&self, edr: EdrEntry) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tokens (transfer_id, token_id, refresh_token_id)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(edr.transfer_id)
        .bind(edr.token_id)
        .bind(edr.refresh_token_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn internal_update(&self, edr: EdrEntry) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE tokens SET token_id=$1, refresh_token_id=$2
            WHERE transfer_id = $3
            "#,
        )
        .bind(edr.token_id)
        .bind(edr.refresh_token_id)
        .bind(edr.transfer_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::migrate!("./migrations/sqlite")
            .run(&self.pool)
            .await?;
        Ok(())
    }
}
