pub use sea_orm_migration::prelude::*;

mod m20240429_082043_hyper_locations;
mod m20240503_190902_hoops;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240429_082043_hyper_locations::Migration),
            Box::new(m20240503_190902_hoops::Migration),
        ]
    }
}
