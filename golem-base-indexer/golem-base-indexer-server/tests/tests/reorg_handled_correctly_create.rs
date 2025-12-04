use crate::helpers;

use blockscout_service_launcher::test_server;
use golem_base_indexer_logic::Indexer;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_create_reorg_handling_works() {
    let db = helpers::init_db("test", "create_reorg_handling").await;
    let client = db.client();
    let indexer = Indexer::new(db.client(), Default::default());
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;

    helpers::load_data(&*client, include_str!("../fixtures/reorg_create_step1.sql")).await;
    indexer.tick().await.unwrap();
    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities?status=ACTIVE").await;
    let expected: serde_json::Value = serde_json::json!([
            {
      "key": "0x286fcdad6145b162147a7055adbc98976128c3577bc004cf8764d15415e8990f",
      "content_type": "text/plain",
      "data": "0x746869732077696c6c2062652064726f7070656420696e2072656f7267",
      "status": "ACTIVE",
      "created_at_tx_hash": "0xd50097b0a75a8b254407ece5be421a332f50f7b640b870f745cc83266aed1703",
      "last_updated_at_tx_hash": "0xd50097b0a75a8b254407ece5be421a332f50f7b640b870f745cc83266aed1703",
      "expires_at_block_number": "102",
      "cost": "0"
            }
        ]
    );
    assert_eq!(
        response.as_object().unwrap().get("items").unwrap(),
        &expected
    );

    helpers::load_data(&*client, include_str!("../fixtures/reorg_create_step2.sql")).await;
    indexer.tick().await.unwrap();
    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/entities?status=ACTIVE").await;
    let expected: serde_json::Value = serde_json::json!(
        [
            {
      "key": "0x2d8eeaf460fddbc21ab54560edfa5db27bf24914264fe9a61265d5d93e41ce5c",
      "content_type": "text/plain",
      "data": "0x746869732077696c6c20737461792061667465722072656f7267",
      "status": "ACTIVE",
      "created_at_tx_hash": "0xa2be32cb84f0aea1d409c785176292053e6e02208574ba81fe4d07f5459abc43",
      "last_updated_at_tx_hash": "0xa2be32cb84f0aea1d409c785176292053e6e02208574ba81fe4d07f5459abc43",
      "expires_at_block_number": "102",
      "cost": "0"
            }
        ]
    );
    assert_eq!(
        response.as_object().unwrap().get("items").unwrap(),
        &expected
    );
}
