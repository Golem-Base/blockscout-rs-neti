use anyhow::{Context, Result};
use sea_orm::{prelude::*, Statement};
use tracing::instrument;

use crate::types::TxHash;

#[instrument(skip(db))]
pub async fn finish_tx_processing<T: ConnectionTrait>(db: &T, tx_hash: TxHash) -> Result<()> {
    let tx_hash: Vec<u8> = tx_hash.as_slice().into();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "delete from golem_base_pending_transaction_operations where hash = $1",
        [tx_hash.into()],
    ))
    .await
    .context("Failed to finish tx processing")?;
    Ok(())
}
