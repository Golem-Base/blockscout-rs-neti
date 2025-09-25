use sea_orm::{entity::prelude::*, DeriveRelation};

#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "golem_base_entity_data_size_histogram")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub bucket: i32,
    pub bin_start: i64,
    pub bin_end: i64,
    pub count: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
