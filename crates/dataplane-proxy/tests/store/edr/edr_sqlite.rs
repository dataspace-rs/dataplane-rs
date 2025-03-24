use async_trait::async_trait;
use edc_dataplane_proxy::db::sqlite::edr::SqliteEdrRepo;

use crate::{generate_token_store_tests, store::Tester};

pub struct SqliteTester(SqliteEdrRepo);

#[async_trait]
impl Tester<SqliteEdrRepo> for SqliteTester {
    async fn create() -> Self {
        let store = SqliteEdrRepo::connect("sqlite::memory:").await.unwrap();

        store.migrate().await.unwrap();
        SqliteTester(store)
    }

    fn store(&self) -> &SqliteEdrRepo {
        &self.0
    }
}

generate_token_store_tests!(SqliteTester);
