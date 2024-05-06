


use std::str::FromStr;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context, Handler};
use chrono::{DateTime, FixedOffset};
use deadpool_redis::{Connection, Manager, Pool};
use redis::{AsyncCommands, Commands};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement, Value};
use crate::actors::consumers;
use crate::types::RedisPoolConnection;
use crate::{models::event::FetchLocationBasicReport};
use crate::s3::Storage;
use crate::consts::PING_INTERVAL;
use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::consts;


#[derive(Message, Clone, Serialize, Deserialize, Debug, Default)]
#[rtype(result = "()")]
pub struct RequestLocationBasicReport{
    pub device_imei: String,
    pub from: String,
    pub to: String
}

#[derive(Clone)]
pub struct LocationAccessorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
}

impl Actor for LocationAccessorActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 LocationAccessorActor has started, let's read baby!");

        ctx.run_interval(PING_INTERVAL, |actor, ctx|{
            
            let this = actor.clone();

            tokio::spawn(async move{

                // check something constantly, schedule to be executed 
                // at a certain time in the background
                // ...
                
            });

        });

    }
}

impl LocationAccessorActor{

    pub fn new(app_storage: std::option::Option<Arc<Storage>>) -> Self{
        Self { app_storage }
    }

    pub async fn raw_get_general_report_none_associated(redis_exp_time: u64,
        imei: &str, from: &str, to: &str, db: &DatabaseConnection,
        redis_pool: std::sync::Arc<RedisPoolConnection>){
            
            // start executing asyncly in a thread worker in the background
            tokio::spawn(async move{

                //...

            });

    }

    pub async fn raw_get_basic_report_none_associated(redis_exp_time: u64,
        imei: &str, from: &str, to: &str, db: &DatabaseConnection, redis_pool: std::sync::Arc<RedisPoolConnection>, 
        producer_actor: Addr<crate::actors::producers::zerlog::ZerLogProducerActor>) -> Option<FetchLocationBasicReport>{

        // -ˋˏ✄┈┈┈┈ validating `from`
        // from => build chrono::DateTime<FixedOffset> from chrono::NaiveDateTime
        let from_get_naive_dt = chrono::NaiveDateTime::parse_from_str(from, "%Y-%m-%dT%H:%M:%S");
        if from_get_naive_dt.is_err(){
            let e = from_get_naive_dt.unwrap_err();
            use crate::error::{ErrorKind, HoopoeErrorResponse};
            let error_content = &e.to_string();
            let error_content_ = error_content.as_bytes().to_vec();
            let mut error_instance = HoopoeErrorResponse::new(
                *consts::CHRONO_ERROR_CODE, // error code
                error_content_, // error content
                ErrorKind::Time(crate::error::TimeError::Chrono(e)), // error kind
                "LocationAccessorActor.raw_get_basic_report_none_associated.NaiveDateTime::parse_from_str|From", // method
                Some(&producer_actor)
            ).await;
            return None; // terminate the caller
        }
        let naive_dt = from_get_naive_dt.unwrap();
        let fixed_offset = FixedOffset::east_opt(0).unwrap();
        let from_dt_fixed_offset = DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_dt, fixed_offset);

        // -ˋˏ✄┈┈┈┈ validating `to`
        // from => build chrono::DateTime<FixedOffset> from chrono::NaiveDateTime
        let to_get_naive_dt = chrono::NaiveDateTime::parse_from_str(to, "%Y-%m-%dT%H:%M:%S");
        if to_get_naive_dt.is_err(){
            let e = to_get_naive_dt.unwrap_err();
            use crate::error::{ErrorKind, HoopoeErrorResponse};
            let error_content = &e.to_string();
            let error_content_ = error_content.as_bytes().to_vec();
            let mut error_instance = HoopoeErrorResponse::new(
                *consts::CHRONO_ERROR_CODE, // error code
                error_content_, // error content
                ErrorKind::Time(crate::error::TimeError::Chrono(e)), // error kind
                "LocationAccessorActor.raw_get_basic_report_none_associated.NaiveDateTime::parse_from_str|to", // method
                Some(&producer_actor)
            ).await;
            return None; // terminate the caller
        }
        let naive_dt = to_get_naive_dt.unwrap();
        let fixed_offset = FixedOffset::east_opt(0).unwrap();
        let to_dt_fixed_offset = DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_dt, fixed_offset);

        // executing report query on hypertable
        let db_backend = db.get_database_backend(); // postgres in our case
        let retrieve_stmt = Statement::from_sql_and_values(db_backend,
                consts::queries::BASIC_REPORT_QUERY1, 
                [
                    imei.into(),
                    Value::ChronoDateTimeWithTimeZone(Some(Box::new(from_dt_fixed_offset))),
                    Value::ChronoDateTimeWithTimeZone(Some(Box::new(to_dt_fixed_offset)))
                ]
            );
              
        match db.query_one(retrieve_stmt).await{
            Ok(query_res) => {
                
                let res = query_res.unwrap();
                let basic_report = FetchLocationBasicReport{
                    // if the column doesn't exist inside the table then we sould 
                    // leave it empty we could instead use a prefix if the column 
                    // is a cumulative column like avg, min or max
                    avg: res.try_get("speed_max", "").unwrap(),
                    min: res.try_get("speed_min", "").unwrap(),
                    max: res.try_get("speed_avg", "").unwrap(),
                    mileage: res.try_get("mileage", "").unwrap(),
                };

                match redis_pool.get().await{
                    Ok(mut redis_conn) => {

                        // cache response in redis so we could retrieve it in other components 
                        // easily during the lifetime of the key without fetching data from db
                        let basic_report_key = format!("basic_report_for:{}_from:{}_to:{}", imei, from, to);
                        let string_data = serde_json::to_string(&basic_report).unwrap(); // passing by reference, it won't move data from the heap

                        /*  IMPORTANT:
                            it take times for redis to store data on the ram
                            make sure you won't get the following error on set_ex():
                            called `Result::unwrap()` on an `Err` value: MISCONF: Redis is configured to 
                            save RDB snapshots, but it's currently unable to persist to disk. Commands that
                            may modify the data set are disabled, because this instance is configured to 
                            report errors during writes if RDB snapshotting fails (stop-writes-on-bgsave-error option). 
                            Please check the Redis logs for details about the RDB error. 
                            SOLUTION: restart redis :)
                        */                     
                        let _: () = redis_conn.set_ex(basic_report_key, string_data, redis_exp_time).await.unwrap();

                        return Some(basic_report);
                    },
                    Err(e) => {
                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                        let error_content = &e.to_string();
                        let error_content_ = error_content.as_bytes().to_vec();
                        let mut error_instance = HoopoeErrorResponse::new(
                            *consts::STORAGE_IO_ERROR_CODE, // error code
                            error_content_, // error content
                            ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // error kind
                            "LocationAccessorActor.redis_pool", // method
                            Some(&producer_actor)
                        ).await;
                        return None; // terminate the caller
                    }
                }
            },
            Err(e) => {
                use crate::error::{ErrorKind, HoopoeErrorResponse};
                let error_content = &e.to_string();
                let error_content = error_content.as_bytes().to_vec();
                let mut error_instance = HoopoeErrorResponse::new(
                    *consts::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                    "LocationMutatorActor.store.db.execute", // method
                    Some(&producer_actor)
                ).await;
                return None; // terminate the caller
            }
        }
        
    }
    
}


// a dead handler!
impl Handler<RequestLocationBasicReport> for LocationAccessorActor{

    type Result = ();

    fn handle(&mut self, msg: RequestLocationBasicReport, ctx: &mut Self::Context) -> Self::Result {
        
        let RequestLocationBasicReport{
            device_imei,
            from,
            to
        } = msg;

        let this = self.clone();
        
        tokio::spawn(async move{
            
            // ...

        });

    }
    
}