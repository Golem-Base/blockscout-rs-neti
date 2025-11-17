use anyhow::Result;
use arkiv_storage_tx::{NumericAttribute, StorageTransaction, StringAttribute, Update};
use golem_base_indexer_logic::{
    Indexer, IndexerSettings,
    test_utils::*,
    types::{Address, EntityKey},
};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use std::{collections::HashMap, sync::Arc};
use tokio::task::JoinSet;

async fn queues_empty(db: Arc<DatabaseConnection>) -> Result<bool> {
    let ops_count: i64 = db
        .query_one(Statement::from_string(
            db.get_database_backend(),
            r#"select (select count(*) from golem_base_pending_transaction_operations) + (select count(*) from golem_base_pending_logs_operations) a"#,
        ))
        .await?
        .unwrap()
        .try_get_by_index(0)
        .unwrap();
    println!("Current op queue: {ops_count}");
    Ok(ops_count == 0)
}

pub(super) async fn test(
    db: DatabaseConnection,
    entities: usize,
    ops_per_entity: usize,
    ops_per_tx: usize,
) -> Result<()> {
    let db = Arc::new(db);
    let indexer = Indexer::new(
        db.clone(),
        IndexerSettings {
            concurrency: 50,
            ..Default::default()
        },
    );

    let mut active_entities: HashMap<EntityKey, usize> =
        (0..entities).fold(HashMap::new(), |mut acc, i| {
            let key = [[0u8; 24].as_slice(), i.to_be_bytes().as_slice()].concat();
            acc.insert(key.as_slice().try_into().unwrap(), ops_per_entity);
            acc
        });
    let mut bn: u64 = 0;
    let total_ops = entities * ops_per_entity;
    let mut blocks = vec![];
    println!(
        "Inserting {} test transactions, might take a while...",
        total_ops / ops_per_tx
    );
    while !active_entities.is_empty() {
        let hash = [[0u8; 24].as_slice(), bn.to_be_bytes().as_slice()].concat();
        let hash = hash.as_slice().try_into()?;
        let mut updates = vec![];
        let mut deletes = vec![];
        active_entities
            .iter_mut()
            .take(ops_per_tx)
            .for_each(|(key, remaining_updates)| {
                *remaining_updates -= 1;
                if *remaining_updates > 0 {
                    updates.push(Update {
                        entity_key: *key,
                        content_type: "Text/plain".into(),
                        btl: 1000,
                        payload: [0u8; 1024].as_slice().into(),
                        string_attributes: vec![StringAttribute {
                            key: "key".into(),
                            value: "value".into(),
                        }],
                        numeric_attributes: vec![NumericAttribute {
                            key: "key".into(),
                            value: 15,
                        }],
                    })
                } else {
                    // FIXME will never be true
                    deletes.push(*key)
                }
            });
        active_entities.retain(|_, v| *v > 0);
        let transactions = vec![Transaction {
            hash: Some(hash),
            sender: Address::ZERO,
            to: None,
            operations: StorageTransaction {
                creates: vec![],
                updates,
                deletes,
                extensions: vec![],
                change_owners: vec![],
            },
        }];
        blocks.push(Block {
            hash: Some(hash),
            number: bn,
            transactions,
            ..Default::default()
        });
        bn += 1;
    }
    let mut set = JoinSet::new();
    for chunk in blocks.chunks(blocks.len() / 10) {
        let chunk: Vec<Block> = chunk.to_owned();
        let db = db.clone();
        set.spawn(async move { insert_data_multi(&*db, chunk).await });
    }
    set.join_all().await.into_iter().collect::<Result<()>>()?;
    println!("Inserted {bn} transactions.");
    println!("Starting benchmark.");

    use std::time::Instant;
    let now = Instant::now();

    {
        while !queues_empty(db.clone()).await? {
            indexer.tick().await?;
        }
    }

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
    println!(
        "Avg: {:.2?} op/s",
        (total_ops as f64) / elapsed.as_secs_f64()
    );
    Ok(())
}
