use golem_base_indexer_entity::sea_orm_active_enums::{
    GolemBaseEntityStatusType, GolemBaseOperationType,
};
use sea_orm::{entity::prelude::*, DeriveRelation};

#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "golem_base_entity_history")]
pub struct Model {
    #[sea_orm(
        primary_key,
        auto_increment = false,
        column_type = "VarBinary(StringLen::None)"
    )]
    pub entity_key: Vec<u8>,
    pub op_index: i64,
    pub block_number: i32,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub block_hash: Vec<u8>,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub transaction_hash: Vec<u8>,
    pub tx_index: i32,
    pub block_timestamp: DateTime,
    #[sea_orm(column_type = "VarBinary(StringLen::None)")]
    pub sender: Vec<u8>,
    #[sea_orm(nullable)]
    pub operation: GolemBaseOperationType,
    #[sea_orm(column_type = "Decimal(Some((21, 0)))", nullable)]
    pub btl: Option<Decimal>,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    pub data: Option<Vec<u8>>,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    pub prev_data: Option<Vec<u8>>,
    pub status: GolemBaseEntityStatusType,
    pub prev_status: Option<GolemBaseEntityStatusType>,
    #[sea_orm(column_type = "Decimal(Some((21, 0)))")]
    pub expires_at_block_number: Decimal,
    #[sea_orm(column_type = "Decimal(Some((21, 0)))", nullable)]
    pub prev_expires_at_block_number: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
