use crate::{
    pagination::paginate_try_from,
    repository::sql,
    types::{
        ConsensusTx, DepositV0, EventMetadata, ExecutionTransaction, FullDeposit, FullEvent, Log,
        PaginationMetadata, PaginationParams, TransactionDepositedEvent,
    },
};
use anyhow::Result;
use optimism_children_indexer_entity::optimism_children_transaction_deposited_events_v0;
use sea_orm::{prelude::*, ActiveValue::Set, FromQueryResult, Statement};
use tracing::instrument;

#[derive(FromQueryResult, Debug)]
struct DbDeposit {
    tx_from: Vec<u8>,
    tx_to: Vec<u8>,
    tx_hash: Vec<u8>,
    block_hash: Vec<u8>,
    index: i32,
    block_number: i32,
    deposit_from: Vec<u8>,
    deposit_to: Vec<u8>,
    source_hash: Vec<u8>,
    mint: Decimal,
    value: Decimal,
    gas_limit: Decimal,
    is_creation: bool,
    calldata: Vec<u8>,
    chain_id: Option<Decimal>,
    execution_tx_block_hash: Option<Vec<u8>>,
    execution_tx_block_number: Option<Decimal>,
    execution_tx_to: Option<Vec<u8>>,
    execution_tx_from: Option<Vec<u8>>,
    execution_tx_hash: Option<Vec<u8>>,
    execution_tx_success: Option<bool>,
}

impl TryFrom<DbDeposit> for FullDeposit<DepositV0> {
    type Error = anyhow::Error;

    fn try_from(value: DbDeposit) -> Result<Self> {
        let execution_tx = value
            .execution_tx_block_hash
            .map(|execution_tx_block_hash| -> Result<ExecutionTransaction> {
                assert!(value.execution_tx_block_number.is_some());
                assert!(value.execution_tx_from.is_some());
                assert!(value.execution_tx_to.is_some());
                assert!(value.execution_tx_hash.is_some());
                assert!(value.execution_tx_success.is_some());

                Ok(ExecutionTransaction {
                    block_hash: execution_tx_block_hash.as_slice().try_into()?,
                    block_number: value.execution_tx_block_number.unwrap().try_into()?,
                    hash: value.execution_tx_hash.unwrap().as_slice().try_into()?,
                    from: value.execution_tx_from.unwrap().as_slice().try_into()?,
                    to: value.execution_tx_to.unwrap().as_slice().try_into()?,
                    success: value.execution_tx_success.unwrap(),
                })
            })
            .transpose()?;

        Ok(Self {
            execution_tx,
            chain_id: value.chain_id.map(TryInto::try_into).transpose()?,
            event: FullEvent {
                metadata: EventMetadata {
                    from: value.tx_from.as_slice().try_into()?,
                    to: value.tx_to.as_slice().try_into()?,
                    transaction_hash: value.tx_hash.as_slice().try_into()?,
                    block_hash: value.block_hash.as_slice().try_into()?,
                    index: value.index.try_into()?,
                    block_number: value.block_number.try_into()?,
                },
                event: TransactionDepositedEvent {
                    from: value.deposit_from.as_slice().try_into()?,
                    to: value.deposit_to.as_slice().try_into()?,
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
pub async fn list_deposits<T: ConnectionTrait>(
    db: &T,
    pagination: PaginationParams,
) -> Result<(Vec<FullDeposit<DepositV0>>, PaginationMetadata)> {
    let q = DbDeposit::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        sql::LIST_DEPOSITS_WITH_TX,
    ))
    .paginate(db, pagination.page_size);
    paginate_try_from(q, pagination).await
}
