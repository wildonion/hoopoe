pub use sea_orm_migration::prelude::*;

mod m20240503_190902_hoops;
mod m20240508_213126_notifs;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240503_190902_hoops::Migration),
            Box::new(m20240508_213126_notifs::Migration),
        ]
    }
}