use crate::{
    pagination::{paginate, paginate_try_from},
    repository::sql,
    types::{
        CurrencyAmount, EntityWithExpTimestamp, LeaderboardBiggestSpendersItem,
        LeaderboardDataOwnedItem, LeaderboardEffectivelyLargestEntitiesItem,
        LeaderboardEntitiesCreatedItem, LeaderboardEntitiesOwnedItem,
        LeaderboardLargestEntitiesItem, PaginationMetadata, PaginationParams,
    },
};
use anyhow::{anyhow, Context, Result};
use golem_base_indexer_entity::{
    golem_base_entities, sea_orm_active_enums::GolemBaseEntityStatusType,
};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, QueryOrder, Statement};
use tracing::instrument;

#[derive(Debug, FromQueryResult)]
struct DbBiggestSpendersItem {
    rank: i64,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    address: Vec<u8>,
    total_fees: String,
}

impl TryFrom<DbBiggestSpendersItem> for LeaderboardBiggestSpendersItem {
    type Error = anyhow::Error;

    fn try_from(value: DbBiggestSpendersItem) -> Result<Self> {
        Ok(Self {
            rank: value.rank as u64,
            address: value.address.as_slice().try_into()?,
            total_fees: value
                .total_fees
                .parse::<CurrencyAmount>()
                .context("Failed to convert transaction_fees to CurrencyAmount")?,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbLeaderboardEntitiesCreatedItem {
    pub rank: i64,
    pub address: Vec<u8>,
    pub entities_created_count: i64,
}

impl TryFrom<DbLeaderboardEntitiesCreatedItem> for LeaderboardEntitiesCreatedItem {
    type Error = anyhow::Error;

    fn try_from(v: DbLeaderboardEntitiesCreatedItem) -> Result<Self> {
        Ok(Self {
            rank: v.rank.try_into()?,
            address: v.address.as_slice().try_into()?,
            entities_created_count: v.entities_created_count.try_into()?,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbLeaderboardEntitiesOwnedItem {
    pub rank: i64,
    pub address: Vec<u8>,
    pub entities_count: i64,
}

impl TryFrom<DbLeaderboardEntitiesOwnedItem> for LeaderboardEntitiesOwnedItem {
    type Error = anyhow::Error;

    fn try_from(value: DbLeaderboardEntitiesOwnedItem) -> Result<Self> {
        Ok(Self {
            rank: value.rank.try_into()?,
            address: value.address.as_slice().try_into()?,
            entities_count: value.entities_count.try_into()?,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbLeaderboardDataOwnedItem {
    pub rank: i64,
    pub address: Vec<u8>,
    pub data_size: i64,
}

impl TryFrom<DbLeaderboardDataOwnedItem> for LeaderboardDataOwnedItem {
    type Error = anyhow::Error;

    fn try_from(value: DbLeaderboardDataOwnedItem) -> Result<Self> {
        Ok(Self {
            rank: value.rank.try_into()?,
            address: value.address.as_slice().try_into()?,
            data_size: value.data_size.try_into()?,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbLeaderboardLargestEntitiesItem {
    pub rank: i64,
    pub entity_key: Vec<u8>,
    pub data_size: i32,
}

impl TryFrom<DbLeaderboardLargestEntitiesItem> for LeaderboardLargestEntitiesItem {
    type Error = anyhow::Error;

    fn try_from(value: DbLeaderboardLargestEntitiesItem) -> Result<Self> {
        Ok(Self {
            rank: value.rank.try_into()?,
            entity_key: value.entity_key.as_slice().try_into()?,
            data_size: value.data_size as u64,
        })
    }
}

#[derive(Debug, FromQueryResult)]
struct DbLeaderboardEffectivelyLargestEntitiesItem {
    pub rank: i64,
    pub entity_key: Vec<u8>,
    pub data_size: i32,
    pub lifespan: i64,
}

impl TryFrom<DbLeaderboardEffectivelyLargestEntitiesItem>
    for LeaderboardEffectivelyLargestEntitiesItem
{
    type Error = anyhow::Error;

    fn try_from(value: DbLeaderboardEffectivelyLargestEntitiesItem) -> Result<Self> {
        Ok(Self {
            rank: value.rank.try_into()?,
            entity_key: value.entity_key.as_slice().try_into()?,
            data_size: value.data_size.try_into()?,
            lifespan: value.lifespan.try_into()?,
        })
    }
}

#[instrument(skip(db))]
pub async fn leaderboard_biggest_spenders<T: ConnectionTrait>(
    db: &T,
    pagination: PaginationParams,
) -> Result<(Vec<LeaderboardBiggestSpendersItem>, PaginationMetadata)> {
    let stmt = Statement::from_string(db.get_database_backend(), sql::LEADERBOARD_BIGGEST_SPENDERS);

    let paginator =
        DbBiggestSpendersItem::find_by_statement(stmt).paginate(db, pagination.page_size);

    paginate_try_from(paginator, pagination)
        .await
        .context("Failed to fetch biggest spenders")
}

#[instrument(skip(db))]
pub async fn leaderboard_entities_created<T: ConnectionTrait>(
    db: &T,
    filter: PaginationParams,
) -> Result<(Vec<LeaderboardEntitiesCreatedItem>, PaginationMetadata)> {
    let paginator =
        DbLeaderboardEntitiesCreatedItem::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql::LEADERBOARD_ENTITIES_CREATED,
            [],
        ))
        .paginate(db, filter.page_size);

    paginate_try_from(paginator, filter).await
}

#[instrument(skip(db))]
pub async fn leaderboard_entities_owned<T: ConnectionTrait>(
    db: &T,
    filter: PaginationParams,
) -> Result<(Vec<LeaderboardEntitiesOwnedItem>, PaginationMetadata)> {
    let paginator = DbLeaderboardEntitiesOwnedItem::find_by_statement(
        Statement::from_sql_and_values(DbBackend::Postgres, sql::LEADERBOARD_ENTITIES_OWNED, []),
    )
    .paginate(db, filter.page_size);

    paginate_try_from(paginator, filter).await
}

#[instrument(skip(db))]
pub async fn leaderboard_data_owned<T: ConnectionTrait>(
    db: &T,
    filter: PaginationParams,
) -> Result<(Vec<LeaderboardDataOwnedItem>, PaginationMetadata)> {
    let paginator = DbLeaderboardDataOwnedItem::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql::LEADERBOARD_DATA_OWNED,
        [],
    ))
    .paginate(db, filter.page_size);

    paginate_try_from(paginator, filter).await
}

#[instrument(skip(db))]
pub async fn leaderboard_largest_entities<T: ConnectionTrait>(
    db: &T,
    filter: PaginationParams,
) -> Result<(Vec<LeaderboardLargestEntitiesItem>, PaginationMetadata)> {
    let paginator = DbLeaderboardLargestEntitiesItem::find_by_statement(
        Statement::from_sql_and_values(DbBackend::Postgres, sql::LEADERBOARD_LARGEST_ENTITIES, []),
    )
    .paginate(db, filter.page_size);

    paginate_try_from(paginator, filter).await
}

#[instrument(skip(db))]
pub async fn leaderboard_effectively_largest_entities<T: ConnectionTrait>(
    db: &T,
    filter: PaginationParams,
) -> Result<(
    Vec<LeaderboardEffectivelyLargestEntitiesItem>,
    PaginationMetadata,
)> {
    let paginator = DbLeaderboardEffectivelyLargestEntitiesItem::find_by_statement(
        Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql::LEADERBOARD_EFFECTIVELY_LARGEST_ENTITIES,
            [],
        ),
    )
    .paginate(db, filter.page_size);

    paginate_try_from(paginator, filter).await
}

// NOTE: This leaderboard does not use materialized view and queries `golem_base_entities` and
// `blocks` directly. If performance becomes an issue this can possibly be refactored for some
// improvement.
#[instrument(skip(db))]
pub async fn leaderboard_entities_by_btl<T: ConnectionTrait>(
    db: &T,
    filter: PaginationParams,
) -> Result<(Vec<EntityWithExpTimestamp>, PaginationMetadata)> {
    let paginator = golem_base_entities::Entity::find()
        .filter(golem_base_entities::Column::Status.eq(GolemBaseEntityStatusType::Active))
        .order_by_desc(golem_base_entities::Column::ExpiresAtBlockNumber)
        .paginate(db, filter.page_size);

    let reference_block = super::blockscout::get_current_block(db)
        .await?
        .ok_or(anyhow!("No blocks indexed yet"))?;

    let (entities, pagination_metadata) = paginate(paginator, filter).await?;

    Ok((
        entities
            .into_iter()
            .map(|v| EntityWithExpTimestamp::try_new(v, &reference_block))
            .collect::<Result<Vec<_>>>()?,
        pagination_metadata,
    ))
}
