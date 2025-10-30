use crate::types::{ConsensusTx, EventMetadata, FullEvent, Log, TransactionDepositedEvent};
use anyhow::{ensure, Result};
use optimism_children_indexer_entity::optimism_children_transaction_deposited_events;
use sea_orm::{prelude::*, ActiveValue::Set, QueryOrder};
use tracing::instrument;

impl TryFrom<optimism_children_transaction_deposited_events::Model>
    for FullEvent<TransactionDepositedEvent>
{
    type Error = anyhow::Error;

    fn try_from(value: optimism_children_transaction_deposited_events::Model) -> Result<Self> {
        ensure!(
            value.version.scale() == 0,
            "Failed to process version - contains decimals"
        );

        Ok(Self {
            metadata: EventMetadata {
                transaction_hash: value.transaction_hash.as_slice().try_into()?,
                block_hash: value.block_hash.as_slice().try_into()?,
                index: value.index.try_into()?,
                block_number: value.block_number.try_into()?,
            },
            event: TransactionDepositedEvent {
                from: value.from.as_slice().try_into()?,
                to: value.to.as_slice().try_into()?,
                version: value.version.mantissa().try_into()?,
                data: value.data.into(),
            },
        })
    }
}

#[instrument(skip(db))]
pub async fn store_transaction_deposited<T: ConnectionTrait>(
    db: &T,
    tx: ConsensusTx,
    log: Log,
    event: TransactionDepositedEvent,
) -> Result<()> {
    let version: u128 = event.version.try_into()?; // Decimal used by SeaORM only stores up to 96
                                                   // bits of precision, so it's OK we don't support full u256
    let model = optimism_children_transaction_deposited_events::ActiveModel {
        transaction_hash: Set(tx.hash.as_slice().into()),
        block_hash: Set(tx.block_hash.as_slice().into()),
        block_number: Set(tx.block_number.try_into()?),
        index: Set(log.index.try_into()?),
        from: Set(event.from.as_slice().into()),
        to: Set(event.to.as_slice().into()),
        version: Set(version.into()),
        data: Set(event.data.to_vec()),
    };

    optimism_children_transaction_deposited_events::Entity::insert(model)
        .exec(db)
        .await?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn find_transaction_deposited<T: ConnectionTrait>(
    db: &T,
) -> Result<Vec<FullEvent<TransactionDepositedEvent>>> {
    optimism_children_transaction_deposited_events::Entity::find()
        .order_by_asc(optimism_children_transaction_deposited_events::Column::BlockNumber)
        // FIXME for chronological order we should have a tx index here as well
        .order_by_asc(optimism_children_transaction_deposited_events::Column::Index)
        .all(db)
        .await?
        .into_iter()
        .map(FullEvent::<TransactionDepositedEvent>::try_from)
        .collect()
}
