use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        let etype_index = Index::create()
            .if_not_exists()
            .index_type(sea_query::IndexType::BTree)
            .name("idx-le-etype")
            .table(Hoops::Table)
            .col(Hoops::Etype)
            .to_owned();

        let manager_index = Index::create()
            .if_not_exists()
            .index_type(sea_query::IndexType::BTree)
            .name("idx-le-longitude")
            .table(Hoops::Table)
            .col(Hoops::Manager)
            .to_owned();

        manager
            .create_table(
                Table::create()
                    .table(Hoops::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Hoops::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Hoops::Etype)
                            .not_null()
                            .char_len(10)
                    )
                    .col(
                        ColumnDef::new(Hoops::Manager)
                            .integer()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Hoops::EntranceFee)
                            .big_integer()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Hoops::CreatedAt)
                            .timestamp_with_time_zone()
                            .default(Expr::current_timestamp())
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Hoops::UpdatedAt)
                            .timestamp_with_time_zone()
                            .default(Expr::current_timestamp())
                            .not_null()
                    )
                    .to_owned(),
            )
            .await;

            // put these right after create table transaction otherwise you get:
            // current transaction is aborted, commands ignored until end of transaction block
            // first crate table then add its indexes
            manager
                .create_index(etype_index).await;
            manager
                .create_index(manager_index).await;

            // try seeding data with entity in here 
            // https://www.sea-ql.org/SeaORM/docs/migration/writing-migration/#tip-3-seed-data-with-entity
            // ...

        Ok(())

    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Hoops::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Hoops {
    Table, // reserved for table name
    Id, // primary key to bring the entity into life
    Etype,
    Manager,
    EntranceFee,
    CreatedAt,
    UpdatedAt
}
