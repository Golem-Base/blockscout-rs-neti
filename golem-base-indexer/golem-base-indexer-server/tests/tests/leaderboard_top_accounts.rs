use crate::helpers;

use blockscout_service_launcher::test_server;
use helpers::utils::refresh_leaderboards;
use pretty_assertions::assert_eq;

#[tokio::test]
#[ignore = "Needs database to run"]
async fn test_leaderboard_top_accounts() {
    let db = helpers::init_db("test", "leaderboard_top_accounts").await;
    let client = db.client();
    let base = helpers::init_golem_base_indexer_server(db, |x| x).await;
    helpers::load_data(&*client, include_str!("../fixtures/addresses.sql")).await;
    refresh_leaderboards(client).await.unwrap();

    let response: serde_json::Value =
        test_server::send_get_request(&base, "/api/v1/leaderboard/top-accounts?page_size=3").await;

    let expected = serde_json::json!([
        {
            "rank": "1",
            "address": "0x009596456753150e12e4Eaf98e1a46B2c16c1D22",
            "balance": "9999996258075792903",
            "tx_count": "0",
        },
        {
            "rank": "2",
            "address": "0xF46E23f6a6F6336D4C64D5D1c95599bF77a536f0",
            "balance": "999999978514607890",
            "tx_count": "1",
        },
        {
            "rank": "3",
            "address": "0xceE72E1328f212F3f8aaC39766568e63EB2aB457",
            "balance": "1999975489848040",
            "tx_count": "0",
        },
    ]);

    assert_eq!(response["items"], expected);
}
