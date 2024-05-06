


use std::collections::HashMap;
use crate::error;

pub type HoopoeHttpResponse = Result<actix_web::HttpResponse, error::HoopoeErrorResponse>;
pub type RamDb = std::sync::Arc<tokio::sync::Mutex<HashMap<String, String>>>;
pub type LapinPoolConnection = deadpool::managed::Pool<deadpool_lapin::Manager>;
pub type RedisPoolConnection = deadpool::managed::Pool<deadpool_redis::Manager, deadpool_redis::Connection>;
