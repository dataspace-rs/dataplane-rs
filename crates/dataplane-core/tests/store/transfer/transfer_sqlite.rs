use async_trait::async_trait;
use edc_dataplane_core::core::db::sqlite::transfer::SqliteTransferRepo;

use crate::{generate_transfer_store_tests, store::Tester};

pub struct SqliteTester(SqliteTransferRepo);

#[async_trait]
impl Tester<SqliteTransferRepo> for SqliteTester {
    async fn create() -> Self {
        let store = SqliteTransferRepo::connect("sqlite::memory:")
            .await
            .unwrap();

        store.migrate().await.unwrap();
        SqliteTester(store)
    }

    fn store(&self) -> &SqliteTransferRepo {
        &self.0
    }
}

generate_transfer_store_tests!(SqliteTester);
