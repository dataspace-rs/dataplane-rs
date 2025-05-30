use sqlx::{QueryBuilder, SqlitePool};

use crate::core::{
    db::transfer::{TransferQuery, TransferRepo},
    model::transfer::{Transfer, TransferStatus},
};

#[derive(Clone)]
pub struct SqliteTransferRepo {
    pool: SqlitePool,
}

impl SqliteTransferRepo {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePool::connect(url).await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl TransferRepo for SqliteTransferRepo {
    async fn save(&self, transfer: Transfer) -> anyhow::Result<()> {
        if self.fetch_by_id(&transfer.id).await?.is_none() {
            self.internal_save(transfer).await?;
        } else {
            self.internal_update(transfer).await?;
        }
        Ok(())
    }
    async fn fetch_by_id(&self, transfer_id: &str) -> anyhow::Result<Option<Transfer>> {
        sqlx::query_as::<_, Transfer>(
            r#"
            SELECT * FROM transfers where id = $1
            "#,
        )
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await
        .map(Ok)?
    }

    async fn query(&self, query: TransferQuery) -> anyhow::Result<Vec<Transfer>> {
        let mut q = QueryBuilder::new("SELECT * FROM transfers");

        if query.id.is_some() {
            q.push(" WHERE ");
        }

        if let Some(id) = query.id {
            q.push(" id = ").push_bind(id);
        }

        q.push(" LIMIT ")
            .push_bind(query.limit)
            .push(" OFFSET ")
            .push_bind(query.offset);

        q.build_query_as().fetch_all(&self.pool).await.map(Ok)?
    }

    async fn delete(&self, transfer_id: &str) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM transfers where id = $1
            "#,
        )
        .bind(transfer_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn change_status(&self, id: String, status: TransferStatus) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE transfers SET status=$1
            WHERE id = $2
            "#,
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

impl SqliteTransferRepo {
    async fn internal_save(&self, transfer: Transfer) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO transfers (id, status, source, participant_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(transfer.id)
        .bind(transfer.status)
        .bind(transfer.source)
        .bind(transfer.participant_id)
        .bind(transfer.created_at)
        .bind(transfer.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn internal_update(&self, transfer: Transfer) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE transfers SET updated_at=$1, status=$2
            WHERE id = $3
            "#,
        )
        .bind(transfer.updated_at)
        .bind(transfer.status)
        .bind(transfer.id)
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
