use crate::{
    pagination::paginate_try_from,
    repository::sql,
    types::{
        Address, BlockHash, ConsensusTx, EventMetadata, FullEvent, FullWithdrawal, Log,
        PaginationMetadata, PaginationParams, TxHash, WithdrawalFinalizedEvent,
        WithdrawalProvenEvent,
    },
};
use alloy_primitives::{Bytes, B256, U256};
use anyhow::Result;
use optimism_children_indexer_entity::{
    optimism_children_withdrawal_finalized_events, optimism_children_withdrawal_proven_events,
};
use sea_orm::{prelude::*, ActiveValue::Set, FromQueryResult, Statement};
use std::str::FromStr;
use tracing::instrument;

#[derive(FromQueryResult, Debug)]
struct DbWithdrawal {
    // L3 initiating transaction (MessagePassed event)
    chain_id: i64,
    l3_block_number: i64,
    l3_block_hash: Vec<u8>,
    l3_tx_hash: Vec<u8>,
    nonce: BigDecimal,
    sender: Vec<u8>,
    target: Vec<u8>,
    value: BigDecimal,
    gas_limit: BigDecimal,
    data: Vec<u8>,
    withdrawal_hash: Vec<u8>,

    // L2 proving transaction (WithdrawalProven event)
    proven_tx_hash: Option<Vec<u8>>,
    proven_block_hash: Option<Vec<u8>>,
    proven_block_number: Option<i32>,
    proven_log_index: Option<i32>,
    proven_from: Option<Vec<u8>>,
    proven_to: Option<Vec<u8>>,
    proven_tx_from: Option<Vec<u8>>,
    proven_tx_to: Option<Vec<u8>>,

    // L2 finalizing transaction (WithdrawalFinalized event)
    finalized_tx_hash: Option<Vec<u8>>,
    finalized_block_hash: Option<Vec<u8>>,
    finalized_block_number: Option<i32>,
    finalized_log_index: Option<i32>,
    finalized_success: Option<bool>,
    finalized_tx_from: Option<Vec<u8>>,
    finalized_tx_to: Option<Vec<u8>>,
}

impl TryFrom<DbWithdrawal> for FullWithdrawal {
    type Error = anyhow::Error;

    fn try_from(value: DbWithdrawal) -> Result<Self> {
        let proving_tx = match (
            value.proven_tx_hash,
            value.proven_block_hash,
            value.proven_block_number,
            value.proven_log_index,
            value.proven_from,
            value.proven_to,
            value.proven_tx_from,
            value.proven_tx_to,
        ) {
            (
                Some(tx_hash),
                Some(block_hash),
                Some(block_number),
                Some(index),
                Some(from),
                Some(to),
                Some(tx_from),
                Some(tx_to),
            ) => Some(FullEvent {
                metadata: EventMetadata {
                    from: Address::from_slice(&tx_from),
                    to: Address::from_slice(&tx_to),
                    transaction_hash: TxHash::from_slice(&tx_hash),
                    block_hash: BlockHash::from_slice(&block_hash),
                    index: index.try_into()?,
                    block_number: block_number.try_into()?,
                },
                event: WithdrawalProvenEvent {
                    withdrawal_hash: B256::from_slice(&value.withdrawal_hash),
                    from: Address::from_slice(&from),
                    to: Address::from_slice(&to),
                },
            }),
            _ => None,
        };

        let finalizing_tx = match (
            value.finalized_tx_hash,
            value.finalized_block_hash,
            value.finalized_block_number,
            value.finalized_log_index,
            value.finalized_success,
            value.finalized_tx_from,
            value.finalized_tx_to,
        ) {
            (
                Some(tx_hash),
                Some(block_hash),
                Some(block_number),
                Some(index),
                Some(success),
                Some(tx_from),
                Some(tx_to),
            ) => Some(FullEvent {
                metadata: EventMetadata {
                    from: Address::from_slice(&tx_from),
                    to: Address::from_slice(&tx_to),
                    transaction_hash: TxHash::from_slice(&tx_hash),
                    block_hash: BlockHash::from_slice(&block_hash),
                    index: index.try_into()?,
                    block_number: block_number.try_into()?,
                },
                event: WithdrawalFinalizedEvent {
                    withdrawal_hash: B256::from_slice(&value.withdrawal_hash),
                    success,
                },
            }),
            _ => None,
        };

        Ok(FullWithdrawal {
            chain_id: value.chain_id.try_into()?,
            l3_block_number: value.l3_block_number.try_into()?,
            l3_block_hash: BlockHash::from_slice(&value.l3_block_hash),
            l3_tx_hash: TxHash::from_slice(&value.l3_tx_hash),
            nonce: U256::from_str(&value.nonce.to_string())?,
            sender: Address::from_slice(&value.sender),
            target: Address::from_slice(&value.target),
            value: U256::from_str(&value.value.to_string())?,
            gas_limit: U256::from_str(&value.gas_limit.to_string())?,
            data: Bytes::from(value.data),
            withdrawal_hash: B256::from_slice(&value.withdrawal_hash),
            proving_tx,
            finalizing_tx,
        })
    }
}

#[instrument(skip(db))]
pub async fn store_withdrawal_proven<T: ConnectionTrait>(
    db: &T,
    tx: ConsensusTx,
    log: Log,
    event: WithdrawalProvenEvent,
) -> Result<()> {
    let model = optimism_children_withdrawal_proven_events::ActiveModel {
        transaction_hash: Set(tx.hash.as_slice().into()),
        block_hash: Set(tx.block_hash.as_slice().into()),
        block_number: Set(tx.block_number.try_into()?),
        index: Set(log.index.try_into()?),
        withdrawal_hash: Set(event.withdrawal_hash.as_slice().into()),
        from: Set(event.from.as_slice().into()),
        to: Set(event.to.as_slice().into()),
    };

    optimism_children_withdrawal_proven_events::Entity::insert(model)
        .exec(db)
        .await?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn store_withdrawal_finalized<T: ConnectionTrait>(
    db: &T,
    tx: ConsensusTx,
    log: Log,
    event: WithdrawalFinalizedEvent,
) -> Result<()> {
    let model = optimism_children_withdrawal_finalized_events::ActiveModel {
        transaction_hash: Set(tx.hash.as_slice().into()),
        block_hash: Set(tx.block_hash.as_slice().into()),
        block_number: Set(tx.block_number.try_into()?),
        index: Set(log.index.try_into()?),
        withdrawal_hash: Set(event.withdrawal_hash.as_slice().into()),
        success: Set(event.success),
    };

    optimism_children_withdrawal_finalized_events::Entity::insert(model)
        .exec(db)
        .await?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn list_withdrawals<T: ConnectionTrait>(
    db: &T,
    pagination: PaginationParams,
) -> Result<(Vec<FullWithdrawal>, PaginationMetadata)> {
    let q = DbWithdrawal::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        sql::LIST_WITHDRAWALS_WITH_TX,
    ))
    .paginate(db, pagination.page_size);
    paginate_try_from(q, pagination).await
}
