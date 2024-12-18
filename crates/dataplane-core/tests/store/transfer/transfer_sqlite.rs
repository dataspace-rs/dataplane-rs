use axum::async_trait;
use edc_dataplane_core::core::db::sqlx::transfer::sqlite::SqliteTransferStore;

use crate::{generate_transfer_store_tests, store::Tester};

pub struct SqliteTester(SqliteTransferStore);

#[async_trait]
impl Tester<SqliteTransferStore> for SqliteTester {
    async fn create() -> Self {
        let store = SqliteTransferStore::connect("sqlite::memory:")
            .await
            .unwrap();

        store.migrate().await.unwrap();
        SqliteTester(store)
    }

    fn store(&self) -> &SqliteTransferStore {
        &self.0
    }
}

generate_transfer_store_tests!(SqliteTester);
