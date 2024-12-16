use axum::async_trait;

mod transfer;

#[async_trait]
pub trait Tester<T> {
    async fn create() -> Self;
    fn store(&self) -> &T;
}

#[macro_export]
macro_rules! declare_test_fn {
    ($storage: ident, $title: ident, $func: path) => {
        #[tokio::test]
        async fn $title() {
            let storage = $storage::create().await;

            $func(storage).await;
        }
    };
}
