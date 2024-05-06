

use crate::*;
use clap::Parser;



#[derive(Parser, Debug)]
#[command(about = "hooper servers", long_about = None)]
#[command(version)]
pub struct ServerKind {

    /// Type of the server you wish to run
    #[arg(short, long, default_value_t = String::from("grpc"))]
    pub server: String,

    /// Fresh all migrations (drop all tables from the database, then reapply all migrations)
    #[arg(short, long, default_value_t = false)]
    pub fresh: bool,

}