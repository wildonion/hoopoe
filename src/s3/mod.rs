



use crate::*;
use redis_async::client::PubsubConnection;
use actix::Addr;
use actix_redis::RedisActor;
use std::sync::Arc;
use redis::Client as RedisClient;
use uuid::Uuid;
use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use rslock::LockManager;
use lapin::{Connection as LapinConnection, ConnectionProperties};
use deadpool_lapin::{Config, Manager, Pool as LapinDeadPool, Runtime};
use deadpool_lapin::lapin::{
    options::BasicPublishOptions,
    BasicProperties,
};
use deadpool_redis::{Config as DeadpoolRedisConfig, Runtime as DeadPoolRedisRuntime};
use self::types::{LapinPoolConnection, RedisPoolConnection};


/*  ----------------------
   | shared state storage 
   |----------------------
   | redis
   | redis async
   | redis actor
   | redis distlock (locker)
   | diesel postgres
   | seaorm
   | rmq
   |
*/


pub struct Db{
    pub mode: Mode,
    pub engine: Option<String>,
    pub url: Option<String>,
    pub redis: Option<RedisClient>,
    pub redis_async_pubsub_conn: Option<Arc<PubsubConnection>>,
    pub redis_actix_actor: Option<Addr<RedisActor>>,
    pub locker: Option<lockers::dlm::DistLock>,
    pub seaorm_pool: Option<DatabaseConnection>,
    pub lapin_pool: Option<std::sync::Arc<LapinPoolConnection>>,
    pub redis_pool: Option<std::sync::Arc<RedisPoolConnection>>,
}

impl Default for Db{
    fn default() -> Db {
        Db{
            mode: self::Mode::Off,
            engine: None,
            url: None,
            redis: None,
            redis_async_pubsub_conn: None,
            redis_actix_actor: None,
            locker: None,
            seaorm_pool: None,
            lapin_pool: None,
            redis_pool: None
        }
    }
}

impl Db{
    
    pub async fn new() -> Result<Db, Box<dyn std::error::Error>>{

        Ok(
            Db{ // building an instance with generic type C which is the type of the db client instance
                mode: Mode::On, // 1 means is on 
                engine: None, 
                url: None,
                redis: None,
                redis_async_pubsub_conn: None,
                redis_actix_actor: None,
                locker: None,
                seaorm_pool: None,
                lapin_pool: None,
                redis_pool: None
            }
        )
    }

}

#[derive(Default)]
pub struct Storage{
    pub id: Uuid,
    pub db: Option<Db>, // we could have no db at all
}

impl Storage{

    pub async fn get_seaorm_pool(&self) -> Option<&DatabaseConnection>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => Some(self.db.as_ref().unwrap().seaorm_pool.as_ref().unwrap()),
            Mode::Off => None,
        }
    }

    pub async fn get_lapin_pool(&self) -> Option<std::sync::Arc<LapinPoolConnection>>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().lapin_pool.clone(),
            Mode::Off => None,
        }
    }

    pub async fn get_redis_pool(&self) -> Option<std::sync::Arc<RedisPoolConnection>>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().redis_pool.clone(),
            Mode::Off => None,
        }
    }

    pub fn get_seaorm_pool_none_async(&self) -> Option<&DatabaseConnection>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => Some(self.db.as_ref().unwrap().seaorm_pool.as_ref().unwrap()),
            Mode::Off => None,
        }
    }

    pub async fn get_redis(&self) -> Option<&RedisClient>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().redis.as_ref(), 
            Mode::Off => None,
        }
    }

    pub fn get_redis_none_async(&self) -> Option<&RedisClient>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().redis.as_ref(), 
            Mode::Off => None,
        }
    }

    pub async fn get_async_redis_pubsub_conn(&self) -> Option<Arc<PubsubConnection>>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().redis_async_pubsub_conn.clone(), 
            Mode::Off => None,
        }
    }

    pub fn get_async_redis_pubsub_conn_none_async(&self) -> Option<Arc<PubsubConnection>>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().redis_async_pubsub_conn.clone(), 
            Mode::Off => None,
        }
    }

    pub async fn get_redis_actix_actor(&self) -> Option<Addr<RedisActor>>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().redis_actix_actor.clone(), 
            Mode::Off => None,
        }
    }

    pub fn get_redis_actix_actor_none_async(&self) -> Option<Addr<RedisActor>>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().redis_actix_actor.clone(), 
            Mode::Off => None,
        }
    }

    pub fn get_locker_manager(&self) -> Option<lockers::dlm::DistLock>{
        match self.db.as_ref().unwrap().mode{
            Mode::On => self.db.as_ref().unwrap().locker.clone(), 
            Mode::Off => None,
        }
    } 

}

#[derive(Copy, Clone, Debug)]
pub enum Mode{ // enum uses 8 bytes (usize which is 64 bits on 64 bits arch) tag which is a pointer pointing to the current variant - the total size of this enum is 8 bytes tag + the largest variant size = 8 + 0 = 8 bytes; cause in our case On and Off variant both have 0 size
    On, // zero byte size
    Off, // zero byte size
}


#[macro_export]
macro_rules! storage {

    ($name:expr, $engine:expr, $host:expr, $port:expr, $username:expr, $password:expr) => {
                
        async { // this is the key! this curly braces is required to use if let statement, use libs and define let inside macro
            
            use super::*;
            use crate::s3::{Storage, Mode, Db};
            use sea_orm::{Database, DatabaseConnection, ConnectOptions};
            use rslock::LockManager;
            use lapin::{Connection as LapinConnection, ConnectionProperties};
            use deadpool_lapin::{Config, Manager, Pool as LapinDeadPool, Runtime};
            use deadpool_lapin::lapin::{
                options::BasicPublishOptions,
                BasicProperties,
            };
            use deadpool_redis::{Config as DeadpoolRedisConfig, Runtime as DeadPoolRedisRuntime};

            dotenv::dotenv().expect("expected .env file be there!");
            
            let redis_password = std::env::var("REDIS_PASSWORD").unwrap_or("".to_string());
            let redis_username = std::env::var("REDIS_USERNAME").unwrap_or("".to_string());
            let redis_host = std::env::var("REDIS_HOST").unwrap_or("localhost".to_string());
            let redis_port = std::env::var("REDIS_PORT").unwrap_or("6379".to_string()).parse::<u64>().unwrap();
            let redis_actor_conn_url = format!("{redis_host}:{redis_port}");

            let redis_conn_url = if !redis_password.is_empty(){
                format!("redis://:{}@{}:{}", redis_password, redis_host, redis_port)
            } else if !redis_password.is_empty() && !redis_username.is_empty(){
                format!("redis://{}:{}@{}:{}", redis_username, redis_password, redis_host, redis_port)
            } else{
                format!("redis://{}:{}", redis_host, redis_port)
            };

            /* redis async, none async and actor setup */
            let none_async_redis_client = redis::Client::open(redis_conn_url.as_str()).unwrap();
            let redis_actor = RedisActor::start(redis_actor_conn_url.as_str());
            let mut redis_conn_builder = ConnectionBuilder::new(redis_host, redis_port as u16).unwrap();
            redis_conn_builder.password(redis_password);
            let async_redis_pubsub_conn = Arc::new(redis_conn_builder.pubsub_connect().await.unwrap());

            let rl = LockManager::new(vec![redis_conn_url.clone()]);

            let pg_url = std::env::var("DATABASE_URL").unwrap();
            let mut opt = ConnectOptions::new(pg_url);
            opt.max_connections(100)
                .min_connections(5)
                .connect_timeout(std::time::Duration::from_secs(8))
                .acquire_timeout(std::time::Duration::from_secs(8))
                .idle_timeout(std::time::Duration::from_secs(8))
                .max_lifetime(std::time::Duration::from_secs(8))
                .sqlx_logging(true)
                .sqlx_logging_level(log::LevelFilter::Info)
                .set_schema_search_path("public"); // postgres default schema is public
            let seaorm_pg_db = Database::connect(opt).await.unwrap();

            let rmq_port = std::env::var("AMQP_PORT").unwrap();
            let rmq_host = std::env::var("AMQP_HOST").unwrap();
            let rmq_username = std::env::var("AMQP_USERNAME").unwrap();
            let rmq_password = std::env::var("AMQP_PASSWORD").unwrap();
            let rmq_addr = format!("amqp://{}:{}@{}:{}", rmq_username, rmq_password, rmq_host, rmq_port);
            let mut cfg = Config::default();
            cfg.url = Some(rmq_addr);
            let lapin_pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();

            let redis_pool_cfg = DeadpoolRedisConfig::from_url(&redis_conn_url);
            let redis_pool = redis_pool_cfg.create_pool(Some(DeadPoolRedisRuntime::Tokio1)).unwrap(); 
            
            let empty_app_storage = Some( // putting the Arc-ed db inside the Option
                Arc::new( // cloning app_storage to move it between threads it's an atomic reader and can be safely move between threads for reading operations
                    Storage{ // defining db context 
                        id: Uuid::new_v4(),
                        db: Some(
                            Db{
                                mode: Mode::Off,
                                engine: None,
                                url: None,
                                redis: None,
                                redis_async_pubsub_conn: None,
                                redis_actix_actor: None,
                                locker: None,
                                seaorm_pool: None,
                                lapin_pool: None,
                                redis_pool: None
                            }
                        ),
                    }
                )
            );
            let app_storage = if $engine.as_str() == "postgres"{
                info!("➔ 🛢️ switching to postgres on address: [{}:{}]", $host, $port);
                let environment = env::var("ENVIRONMENT").expect("⚠️ no environment variable set");                
                let db_addr = if environment == "dev"{
                    format!("{}://{}:{}", $engine, $host, $port)
                } else if environment == "prod"{
                    format!("{}://{}:{}@{}:{}/{}", $engine, $username, $password, $host, $port, $name)
                } else{
                    "".to_string()
                };
                match Db::new().await{
                    Ok(mut init_db) => { // init_db instance must be mutable since we want to mutate its fields
                        init_db.engine = Some($engine);
                        init_db.url = Some(db_addr);
                        Some( // putting the Arc-ed db inside the Option
                            Arc::new( // cloning app_storage to move it between threads
                                Storage{ // defining db context 
                                    id: Uuid::new_v4(),
                                    db: Some(
                                        Db{
                                            mode: init_db.mode,
                                            engine: init_db.engine,
                                            url: init_db.url,
                                            redis: Some(none_async_redis_client.clone()),
                                            redis_async_pubsub_conn: Some(async_redis_pubsub_conn.clone()),
                                            redis_actix_actor: Some(redis_actor.clone()),
                                            locker: Some(lockers::dlm::DistLock::new_redlock(Some(std::sync::Arc::new(rl)))),
                                            seaorm_pool: Some(seaorm_pg_db),
                                            lapin_pool: Some(Arc::new(lapin_pool)),
                                            redis_pool: Some(Arc::new(redis_pool))
                                        }
                                    ),
                                }
                            )
                        )
                    },
                    Err(e) => {
                        error!("init db error due to - {}", e);
                        empty_app_storage // whatever the error is we have to return and empty app storage instance 
                    }
                }
            } else{
                empty_app_storage
            };

            app_storage // returning the created app_storage

        }
    };

}