


// engine supports ram and hard based storages like redis and postgres


use crate::*;
use config::EnvExt;
use redis_async::client::PubsubConnection;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use rslock::LockManager;
use deadpool_lapin::lapin::{Connection as LapinConnection, ConnectionProperties};
use deadpool_lapin::{Config, Manager, Pool as LapinDeadPool, Runtime};
use deadpool_lapin::lapin::{
    options::BasicPublishOptions,
    BasicProperties,
};
use deadpool_redis::{Config as DeadpoolRedisConfig, Runtime as DeadPoolRedisRuntime};
use self::types::{LapinPoolConnection, RedisPoolConnection};
use crate::config::{Env as ConfigEnv, Context};


/*  --------------------------
   | shared state storage (s3)
   |--------------------------
   | redis async pubsub conn
   | redis distlock (locker)
   | redis pool
   | seaorm postgres pool
   | rmq pool
   |
*/


#[derive(Default)]
pub struct Bucket{
    pub mode: Mode,
    pub redis_async_pubsub_conn: Option<Arc<PubsubConnection>>,
    pub locker: Option<lockers::dlm::DistLock>,
    pub seaorm_pool: Option<DatabaseConnection>,
    pub lapin_pool: Option<std::sync::Arc<LapinPoolConnection>>,
    pub redis_pool: Option<std::sync::Arc<RedisPoolConnection>>,
}

#[derive(Default)]
pub struct Storage{
    pub id: Uuid,
    pub bucket: Option<Bucket>, // all db buckets
}


#[derive(Copy, Clone, Debug, Default)]
pub enum Mode{ // enum uses 8 bytes (usize which is 64 bits on 64 bits arch) tag which is a pointer pointing to the current variant - the total size of this enum is 8 bytes tag + the largest variant size = 8 + 0 = 8 bytes; cause in our case On and Off variant both have 0 size
    #[default]
    On, // zero byte size
    Off, // zero byte size
}

impl Storage{

    pub async fn new() -> Option<std::sync::Arc<Self>>{

        let env = ConfigEnv::default();
        let ctx_env = env.get_vars();
        let configs = Some(
            std::sync::Arc::new(ctx_env)
        );
        
        let environment = configs.as_ref().unwrap().vars.clone().ENVIRONMENT;
        let db_url = configs.as_ref().unwrap().vars.clone().DATABASE_URL;
        let db_name = configs.as_ref().unwrap().vars.clone().DB_NAME;
        let db_engine = configs.as_ref().unwrap().vars.clone().DB_ENGINE;
        let db_host = &configs.as_ref().unwrap().vars.POSTGRES_HOST;
        let db_port = &configs.as_ref().unwrap().vars.POSTGRES_PORT;
        let db_username = &configs.as_ref().unwrap().vars.POSTGRES_USER;
        let db_password = &configs.as_ref().unwrap().vars.POSTGRES_PASSWORD;
        let redis_password = &configs.as_ref().unwrap().vars.REDIS_PASSWORD;
        let redis_username = &configs.as_ref().unwrap().vars.REDIS_USERNAME;
        let redis_host = &configs.as_ref().unwrap().vars.REDIS_HOST;
        let redis_port = &configs.as_ref().unwrap().vars.REDIS_PORT.parse::<u16>().unwrap();
        let rmq_port = &configs.as_ref().unwrap().vars.AMQP_PORT;
        let rmq_host = &configs.as_ref().unwrap().vars.AMQP_HOST;
        let rmq_username = &configs.as_ref().unwrap().vars.AMQP_USERNAME;
        let rmq_password = &configs.as_ref().unwrap().vars.AMQP_PASSWORD;


        let redis_conn_url = if !redis_password.is_empty(){
            format!("redis://:{}@{}:{}", redis_password, redis_host, redis_port)
        } else if !redis_password.is_empty() && !redis_username.is_empty(){
            format!("redis://{}:{}@{}:{}", redis_username, redis_password, redis_host, redis_port)
        } else{
            format!("redis://{}:{}", redis_host, redis_port)
        };

        /* redis async, none async and actor setup */
        // let none_async_redis_client = redis::Client::open(redis_conn_url.as_str()).unwrap();
        // let redis_actor = RedisActor::start(redis_actor_conn_url.as_str());
        let mut redis_conn_builder = ConnectionBuilder::new(redis_host, redis_port.to_owned()).unwrap();
        redis_conn_builder.password(redis_password.to_owned());
        let async_redis_pubsub_conn = Arc::new(redis_conn_builder.pubsub_connect().await.unwrap());

        let rl = LockManager::new(vec![redis_conn_url.clone()]);

        let mut opt = ConnectOptions::new(db_url);
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

        // ---------------------- rmq lapin pool
        let rmq_addr = format!("amqp://{}:{}@{}:{}", rmq_username, rmq_password, rmq_host, rmq_port);
        let mut cfg = Config::default();
        cfg.url = Some(rmq_addr);
        let lapin_pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();
        // ----------------------

        // ---------------------- redis lapin pool
        let redis_pool_cfg = DeadpoolRedisConfig::from_url(&redis_conn_url);
        let redis_pool = redis_pool_cfg.create_pool(Some(DeadPoolRedisRuntime::Tokio1)).unwrap(); 
        // ----------------------

        let empty_app_storage = Some( // putting the Arc-ed db inside the Option
            Arc::new( // cloning app_storage to move it between threads it's an atomic reader and can be safely move between threads for reading operations
                Storage{ // defining db context 
                    id: Uuid::new_v4(),
                    bucket: Some(
                        Bucket{
                            mode: Mode::Off,
                            redis_async_pubsub_conn: None,
                            locker: None,
                            seaorm_pool: None,
                            lapin_pool: None,
                            redis_pool: None
                        }
                    ),
                }
            )
        );
        let app_storage = if db_engine.as_str() == "postgres"{
            info!("🛢️ switching to postgres on address: [{}:{}]", db_host, db_port);             
            let db_addr = if environment == "dev"{
                format!("{}://{}:{}", db_engine, db_host, db_port)
            } else if environment == "prod"{
                format!("{}://{}:{}@{}:{}/{}", db_engine, db_username, db_password, db_host, db_port, db_name)
            } else{
                "".to_string()
            };
           
            Some( // putting the Arc-ed db inside the Option
                Arc::new( // cloning app_storage to move it between threads
                    Storage{ // defining db context 
                        id: Uuid::new_v4(),
                        bucket: Some(
                            Bucket{
                                mode: Mode::On,
                                redis_async_pubsub_conn: Some(async_redis_pubsub_conn.clone()),
                                locker: Some(lockers::dlm::DistLock::new_redlock(Some(std::sync::Arc::new(rl)))),
                                seaorm_pool: Some(seaorm_pg_db),
                                lapin_pool: Some(Arc::new(lapin_pool)),
                                redis_pool: Some(Arc::new(redis_pool))
                            }
                        ),
                    }
                )
            )

        } else{
            empty_app_storage
        };

        app_storage // returning the created app_storage
        
    }

    pub async fn get_seaorm_pool(&self) -> Option<&DatabaseConnection>{
        match self.bucket.as_ref().unwrap().mode{
            Mode::On => Some(self.bucket.as_ref().unwrap().seaorm_pool.as_ref().unwrap()),
            Mode::Off => None,
        }
    }

    pub async fn get_lapin_pool(&self) -> Option<std::sync::Arc<LapinPoolConnection>>{
        match self.bucket.as_ref().unwrap().mode{
            Mode::On => self.bucket.as_ref().unwrap().lapin_pool.clone(),
            Mode::Off => None,
        }
    }

    pub async fn get_redis_pool(&self) -> Option<std::sync::Arc<RedisPoolConnection>>{
        match self.bucket.as_ref().unwrap().mode{
            Mode::On => self.bucket.as_ref().unwrap().redis_pool.clone(),
            Mode::Off => None,
        }
    }

    pub async fn get_async_redis_pubsub_conn(&self) -> Option<Arc<PubsubConnection>>{
        match self.bucket.as_ref().unwrap().mode{
            Mode::On => self.bucket.as_ref().unwrap().redis_async_pubsub_conn.clone(), 
            Mode::Off => None,
        }
    }

    pub fn get_locker_manager(&self) -> Option<lockers::dlm::DistLock>{
        match self.bucket.as_ref().unwrap().mode{
            Mode::On => self.bucket.as_ref().unwrap().locker.clone(), 
            Mode::Off => None,
        }
    } 

}
