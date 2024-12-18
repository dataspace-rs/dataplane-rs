use crate::store::Tester;
use edc_dataplane_core::core::db::transfer::TransferQuery;
use edc_dataplane_core::core::db::transfer::TransferStore;
use edc_dataplane_core::{
    core::model::transfer::{Transfer, TransferStatus},
    signaling::DataAddress,
};
use uuid::Uuid;

mod transfer_sqlite;

pub fn create_transfer(id: &str) -> Transfer {
    Transfer::builder()
        .id(id.to_string())
        .source(
            DataAddress::builder()
                .endpoint_type("type".to_string())
                .endpoint_properties(vec![])
                .build(),
        )
        .refresh_token_id(Uuid::new_v4().into())
        .token_id(Uuid::new_v4().into())
        .status(TransferStatus::Started)
        .build()
}

pub async fn save<T: TransferStore>(tester: impl Tester<T>) {
    let store = tester.store();

    let id = Uuid::new_v4().to_string();

    let transfer = create_transfer(&id);

    store.save(transfer.clone()).await.unwrap();

    let saved = store.fetch_by_id(&id).await.unwrap().unwrap();

    assert_eq!(saved, transfer);
}

pub async fn update<T: TransferStore>(tester: impl Tester<T>) {
    let store = tester.store();

    let transfer = create_transfer("1");
    let mut updated = transfer.clone();

    updated.status = TransferStatus::Suspended;

    store.save(transfer.clone()).await.unwrap();
    store.save(updated.clone()).await.unwrap();

    let transfers = store
        .query(TransferQuery::builder().id("1").build())
        .await
        .unwrap();

    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0], updated);
}

pub async fn delete<T: TransferStore>(tester: impl Tester<T>) {
    let store = tester.store();

    let transfer = create_transfer(&Uuid::new_v4().to_string());
    let transfer_2 = create_transfer(&Uuid::new_v4().to_string());

    store.save(transfer.clone()).await.unwrap();
    store.save(transfer_2.clone()).await.unwrap();

    let transfers = store.query(TransferQuery::builder().build()).await.unwrap();

    assert_eq!(transfers.len(), 2);

    store.delete(&transfer_2.id).await.unwrap();

    let transfers = store.query(TransferQuery::builder().build()).await.unwrap();

    assert_eq!(transfers.len(), 1);
}

pub async fn change_status<T: TransferStore>(tester: impl Tester<T>) {
    let store = tester.store();

    let transfer = create_transfer("1");
    let mut updated = transfer.clone();

    updated.status = TransferStatus::Suspended;

    store.save(transfer.clone()).await.unwrap();

    store
        .change_status(transfer.id, TransferStatus::Suspended)
        .await
        .unwrap();

    let transfers = store
        .query(TransferQuery::builder().id("1").build())
        .await
        .unwrap();

    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0], updated);
}

#[macro_export]
macro_rules! generate_transfer_store_tests {
    ($tester:ident) => {
        macro_rules! test {
            ($title: ident, $func: path) => {
                crate::declare_test_fn!($tester, $title, $func);
            };
        }

        test!(save, crate::store::transfer::save);
        test!(update, crate::store::transfer::update);
        test!(delete, crate::store::transfer::delete);
        test!(change_status, crate::store::transfer::change_status);
    };
}
