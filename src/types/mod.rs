
use std::collections::{BTreeMap, HashMap};

// a dynamic dispatch approach for app error, will be trigerred at runtime
// for any type that implements the Error trait. using ? operator on the 
// type causes the error at runtime requires the type to implement From 
// trait.
pub type AppError = Box<dyn std::error::Error + Send + Sync + 'static>; 
pub type AppResult<T> = Result<T, AppError>;
pub type GenericAppResult<T, E> = Result<T, E>;
pub type Job<O> = std::pin::Pin<Box<dyn std::future::Future<Output = O> + Send + Sync + 'static>>; // a future as a separate type must be pinned into the ram
pub type KVan = std::sync::Arc<tokio::sync::Mutex<BTreeMap<String, String>>>;
pub type LapinPoolConnection = deadpool_lapin::Pool;
pub type RedisPoolConnection = deadpool_redis::Pool;