

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        manager
            .create_table(
                Table::create()
                    .table(HyperLocations::Table)
                    .if_not_exists()
                    // -ˋˏ✄┈┈┈┈ defining Timestamp column
                    .col(
                        ColumnDef::new(HyperLocations::Timestamp)
                            .string()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining CorrelationId column
                    .col(
                        ColumnDef::new(HyperLocations::CorrelationId)
                            .string()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining DeviceId column
                    .col(
                        ColumnDef::new(HyperLocations::DeviceId)
                            .string()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Imei column
                    .col(
                        ColumnDef::new(HyperLocations::Imei)
                            .string()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Latitude column
                    .col(
                        ColumnDef::new(HyperLocations::Latitude)
                            .double()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Longitude column
                    .col(
                        ColumnDef::new(HyperLocations::Longitude)
                            .double()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Date column
                    .col(
                        ColumnDef::new(HyperLocations::Date)
                            .timestamp_with_time_zone() // make sure the type is timestampz to query over a hypertable table in timescaledb
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining PositionStatus column
                    .col(
                        ColumnDef::new(HyperLocations::PositionStatus)
                            .boolean()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Speed column
                    .col(
                        ColumnDef::new(HyperLocations::Speed)
                            .double()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Heading column
                    .col(
                        ColumnDef::new(HyperLocations::Heading)
                            .double()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Altitude column
                    .col(
                        ColumnDef::new(HyperLocations::Altitude)
                            .double()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Satellites column
                    .col(
                        ColumnDef::new(HyperLocations::Satellites)
                            .big_integer()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining GsmSignal column
                    .col(
                        ColumnDef::new(HyperLocations::GsmSignal)
                            .big_integer()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Odometer column
                    .col(
                        ColumnDef::new(HyperLocations::Odometer)
                            .double()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining Hdop column
                    .col(
                        ColumnDef::new(HyperLocations::Hdop)
                            .double()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining MobileCountryCode column
                    .col(
                        ColumnDef::new(HyperLocations::MobileCountryCode)
                            .big_integer()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining MobileNetworkCode column
                    .col(
                        ColumnDef::new(HyperLocations::MobileNetworkCode)
                            .big_integer()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining CellId column
                    .col(
                        ColumnDef::new(HyperLocations::CellId)
                            .big_integer()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining LocationAreaCode column
                    .col(
                        ColumnDef::new(HyperLocations::LocationAreaCode)
                            .big_integer()
                            .not_null(),
                    )
                    // -ˋˏ✄┈┈┈┈ defining CumulativeMileage column
                    .col(
                        ColumnDef::new(HyperLocations::CumulativeMileage) // this is a cumulative value
                            .double()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await;

        // creating the hypertable for timescaledb queries, it automatically 
        // adds an index on date column for executing high performance queries
        let db = manager.get_connection();
        db.execute_unprepared("SELECT create_hypertable('hyper_locations', by_range('date'))").await.unwrap();
        db.execute_unprepared("CREATE EXTENSION IF NOT EXISTS postgis").await.unwrap();

        Ok(())

    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        manager
            .drop_table(Table::drop().table(HyperLocations::Table).to_owned())
            .await
    }
}

// defining identifiers that will be used in migration
#[derive(DeriveIden)]
enum HyperLocations{
    Table, // this will be mapped to the table name
    // column names
    Timestamp,
    CorrelationId,
    DeviceId,
    Imei, 
    Latitude,
    Longitude,
    Altitude,
    Date,
    PositionStatus,
    Speed,
    Heading,
    Satellites,
    Hdop,
    GsmSignal,
    Odometer,
    MobileCountryCode,
    MobileNetworkCode,
    LocationAreaCode,
    CellId,
    CumulativeMileage
}