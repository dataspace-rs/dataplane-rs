use crate::store::Tester;
use edc_dataplane_proxy::db::edr::EdrRepo;
use edc_dataplane_proxy::model::edr::EdrEntry;
use uuid::Uuid;

mod edr_sqlite;

pub fn create_token(id: &str) -> EdrEntry {
    EdrEntry::builder()
        .transfer_id(id.to_string())
        .token_id(Uuid::new_v4())
        .refresh_token_id(Uuid::new_v4())
        .build()
}

pub async fn save<T: EdrRepo>(tester: impl Tester<T>) {
    let store = tester.store();

    let id = Uuid::new_v4().to_string();

    let transfer = create_token(&id);

    store.save(transfer.clone()).await.unwrap();

    let saved = store.fetch_by_id(&id).await.unwrap().unwrap();

    assert_eq!(saved, transfer);
}

pub async fn update<T: EdrRepo>(tester: impl Tester<T>) {
    let store = tester.store();

    let token = create_token("1");
    let mut updated_token = token.clone();

    updated_token.token_id = Uuid::new_v4().into();
    updated_token.refresh_token_id = Uuid::new_v4().into();

    store.save(token.clone()).await.unwrap();
    store.save(updated_token.clone()).await.unwrap();

    let token = store.fetch_by_id("1").await.unwrap().unwrap();

    assert_eq!(token, updated_token);
}

pub async fn delete<T: EdrRepo>(tester: impl Tester<T>) {
    let store = tester.store();

    let transfer = create_token(&Uuid::new_v4().to_string());
    let transfer_2 = create_token(&Uuid::new_v4().to_string());

    store.save(transfer.clone()).await.unwrap();
    store.save(transfer_2.clone()).await.unwrap();

    store.delete(&transfer_2.transfer_id).await.unwrap();

    let transfers = store.fetch_by_id(&transfer_2.transfer_id).await.unwrap();

    assert!(transfers.is_none());
}

#[macro_export]
macro_rules! generate_token_store_tests {
    ($tester:ident) => {
        macro_rules! test {
            ($title: ident, $func: path) => {
                crate::declare_test_fn!($tester, $title, $func);
            };
        }

        test!(save, crate::store::edr::save);
        test!(update, crate::store::edr::update);
        test!(delete, crate::store::edr::delete);
    };
}
