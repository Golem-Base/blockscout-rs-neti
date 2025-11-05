use crate::types::{
    ConsensusTx, DepositV0, EventMetadata, FullEvent, Log, TransactionDepositedEvent,
};
use anyhow::Result;
use optimism_children_indexer_entity::optimism_children_transaction_deposited_events_v0;
use sea_orm::{prelude::*, ActiveValue::Set, QueryOrder};
use tracing::instrument;

impl TryFrom<optimism_children_transaction_deposited_events_v0::Model>
    for FullEvent<TransactionDepositedEvent<DepositV0>>
{
    type Error = anyhow::Error;

    fn try_from(value: optimism_children_transaction_deposited_events_v0::Model) -> Result<Self> {
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
                source_hash: value.source_hash.as_slice().try_into()?,
                deposit: DepositV0 {
                    // these conversions to u128 are fine, as Decimal only has 96 bits of precision
                    // anyway
                    mint: u128::try_from(value.mint)?.try_into()?,
                    value: u128::try_from(value.value)?.try_into()?,
                    gas_limit: value.gas_limit.try_into()?,
                    is_creation: value.is_creation,
                    calldata: value.calldata.into(),
                },
            },
        })
    }
}

#[instrument(skip(db))]
pub async fn store_transaction_deposited<T: ConnectionTrait>(
    db: &T,
    tx: ConsensusTx,
    log: Log,
    event: TransactionDepositedEvent<DepositV0>,
) -> Result<()> {
    let mint: u128 = event.deposit.mint.try_into()?;
    let value: u128 = event.deposit.value.try_into()?;

    let model = optimism_children_transaction_deposited_events_v0::ActiveModel {
        transaction_hash: Set(tx.hash.as_slice().into()),
        block_hash: Set(tx.block_hash.as_slice().into()),
        block_number: Set(tx.block_number.try_into()?),
        index: Set(log.index.try_into()?),
        source_hash: Set(event.source_hash.as_slice().into()),
        from: Set(event.from.as_slice().into()),
        to: Set(event.to.as_slice().into()),
        mint: Set(mint.into()),
        value: Set(value.into()),
        gas_limit: Set(event.deposit.gas_limit.into()),
        is_creation: Set(event.deposit.is_creation),
        calldata: Set(event.deposit.calldata.into()),
    };

    optimism_children_transaction_deposited_events_v0::Entity::insert(model)
        .exec(db)
        .await?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn find_transaction_deposited<T: ConnectionTrait>(
    db: &T,
) -> Result<Vec<FullEvent<TransactionDepositedEvent<DepositV0>>>> {
    optimism_children_transaction_deposited_events_v0::Entity::find()
        .order_by_asc(optimism_children_transaction_deposited_events_v0::Column::BlockNumber)
        // FIXME for chronological order we should have a tx index here as well
        .order_by_asc(optimism_children_transaction_deposited_events_v0::Column::Index)
        .all(db)
        .await?
        .into_iter()
        .map(FullEvent::<TransactionDepositedEvent<DepositV0>>::try_from)
        .collect()
}
