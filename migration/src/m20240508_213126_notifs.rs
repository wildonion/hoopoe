use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        let db = manager.get_connection();
        db.execute_unprepared("
            SELECT CURRENT_DATE;
        ").await.unwrap();

        let fired_at_index = Index::create()
            .if_not_exists()
            .index_type(sea_query::IndexType::BTree)
            .name("idx-le-fired-at")
            .table(Notifs::Table)
            .col(Notifs::FiredAt)
            .to_owned();

        manager
            .create_table(
                Table::create()
                    .table(Notifs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Notifs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Notifs::ReceiverInfo)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Notifs::Nid)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Notifs::ActionData)
                            .json()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Notifs::ActionerInfo)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Notifs::ActionType)
                            .string()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Notifs::FiredAt)
                            .timestamp()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(Notifs::IsSeen)
                            .boolean()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await;

        manager.create_index(fired_at_index).await;

        Ok(())

    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        manager
            .drop_table(Table::drop().table(Notifs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Notifs {
    Table,
    Id,
    ReceiverInfo,
    Nid,
    ActionData,
    ActionerInfo,
    ActionType,
    FiredAt, 
    IsSeen,
}
