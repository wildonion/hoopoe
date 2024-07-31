use sea_orm_migration::prelude::*;
use crate::m20240503_190902_hoops::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        manager
            .create_table(
                Table::create()
                    .table(UsersHoops::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UsersHoops::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UsersHoops::UserId).integer().not_null())
                    .col(ColumnDef::new(UsersHoops::HoopId).integer().not_null())
                    .col(ColumnDef::new(UsersHoops::IsInvited).boolean().not_null())
                    .col(ColumnDef::new(UsersHoops::JoinedAt).big_integer().not_null())
                    .col(ColumnDef::new(UsersHoops::LeftAt).big_integer().not_null())
                    .to_owned(),
            )
            .await;

        manager.create_foreign_key(
            sea_query::ForeignKey::create()
                .name("FK_hoop_id")
                // HoopId from users_hoops table is a fk to the hoops table
                .from(UsersHoops::Table, UsersHoops::HoopId) 
                .to(Hoops::Table, Hoops::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
                .to_owned()
        ).await;

        Ok(())

    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UsersHoops::Table).to_owned())
            .await;

        Ok(())

    }
}

#[derive(DeriveIden)]
enum UsersHoops {
    Table,
    Id,
    UserId,
    HoopId,
    IsInvited,
    JoinedAt,
    LeftAt,
}
