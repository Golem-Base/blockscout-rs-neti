use crate::helpers;

use alloy_primitives::Address;
use arkiv_storage_tx::{ChangeOwner, Create, StorageTransaction};
use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

use crate::helpers::sample::{Block, Transaction};

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_creator_field_is_populated_and_unchanged_after_change_owner() {
    let db = helpers::init_db("test", "creator_field_works").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    let creator = Address::random();
    let new_owner = Address::random();

    // Create an entity with a specific creator
    helpers::sample::insert_data(
        &*client,
        Block {
            number: 1,
            transactions: vec![Transaction {
                sender: creator,
                operations: StorageTransaction {
                    creates: vec![Create {
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let indexer = Indexer::new(client.clone(), Default::default());
    indexer.tick().await.unwrap();

    // Get the entity key from the first entity
    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities?status=ACTIVE").await;
    let entity_key = response["items"][0]["key"].as_str().unwrap();

    // Verify creator is set correctly on creation
    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entity/{}", entity_key)).await;
    assert_eq!(
        response["creator"].as_str().unwrap(),
        creator.to_checksum(None)
    );
    assert_eq!(
        response["owner"].as_str().unwrap(),
        creator.to_checksum(None)
    );

    // Change the owner
    helpers::sample::insert_data(
        &*client,
        Block {
            number: 2,
            transactions: vec![Transaction {
                sender: creator,
                operations: StorageTransaction {
                    change_owners: vec![ChangeOwner {
                        entity_key: entity_key.parse().unwrap(),
                        new_owner,
                    }],
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .await
    .unwrap();

    indexer.tick().await.unwrap();

    // Verify creator remains the same, but owner changed
    let response: serde_json::Value =
        test_server::send_get_request(&base, &format!("/api/v1/entity/{}", entity_key)).await;
    assert_eq!(
        response["creator"].as_str().unwrap(),
        creator.to_checksum(None),
        "Creator should remain unchanged after change owner"
    );
    assert_eq!(
        response["owner"].as_str().unwrap(),
        new_owner.to_checksum(None),
        "Owner should be updated after change owner"
    );
    assert_ne!(
        response["creator"].as_str().unwrap(),
        response["owner"].as_str().unwrap(),
        "Creator and owner should be different after change owner"
    );
}
