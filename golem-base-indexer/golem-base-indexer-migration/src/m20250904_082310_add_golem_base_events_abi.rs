use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
INSERT INTO 
    smart_contracts (
        name,
        abi,
        address_hash,
        inserted_at,
        updated_at,
        compiler_version,
        optimization,
        contract_source_code,
        contract_code_md5
    )
VALUES (
  'GolemBaseSystem',
  '[
    {
      "type":"event",
      "name":"GolemBaseStorageEntityCreated",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":false,"name":"expirationBlock","type":"uint256"}
      ],
      "anonymous":false
    },
    {
      "type":"event",
      "name":"GolemBaseStorageEntityUpdated",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":false,"name":"newExpirationBlock","type":"uint256"}
      ],
      "anonymous":false
    },
    {
      "type":"event",
      "name":"GolemBaseStorageEntityDeleted",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"}
      ],
      "anonymous":false
    },
    {
      "type":"event",
      "name":"GolemBaseStorageEntityBTLExtended",
      "inputs":[
        {"indexed":true,"name":"entityKey","type":"uint256"},
        {"indexed":false,"name":"oldExpirationBlock","type":"uint256"},
        {"indexed":false,"name":"newExpirationBlock","type":"uint256"}
      ],
      "anonymous":false
    }
  ]'::jsonb,
  decode('0000000000000000000000000000000060138453','hex'),
  NOW(), 
  NOW(),
  '0.0.0', 
  false, 
  'GolemBaseSystem', 
  '0x00'
);
        "#;
        crate::from_sql(manager, sql).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DELETE FROM smart_contracts WHERE name = 'GolemBaseSystem';
        "#;
        crate::from_sql(manager, sql).await
    }
}
