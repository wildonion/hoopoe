


use std::collections::HashMap;
use crate::error;

pub type HoopoeHttpResponse = Result<actix_web::HttpResponse, error::HoopoeErrorResponse>;
pub type RamDb = std::sync::Arc<tokio::sync::Mutex<HashMap<String, String>>>;
pub type LapinPoolConnection = deadpool_lapin::Pool;
pub type RedisPoolConnection = deadpool_redis::Pool;