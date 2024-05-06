
use chrono::{DateTime, FixedOffset, Local};
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, Statement, TryIntoModel, Value};
use serde::{Serialize, Deserialize};
use actix::prelude::*;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context};
use crate::actors::producers::zerlog::ZerLogProducerActor;
use crate::entities::hoops;
use crate::models::event::LocationEventMessage;
use crate::s3::Storage;
use crate::consts::{self, PING_INTERVAL};
use serde_json::json;

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StoreLocationEvent{
    pub message: LocationEventMessage
}



#[derive(Clone)]
pub struct LocationMutatorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
    pub zerlog_producer_actor: Addr<ZerLogProducerActor>
}

impl Actor for LocationMutatorActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 LocationMutatorActor has started, let's mutate baby!");

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

impl LocationMutatorActor{

    pub fn new(app_storage: std::option::Option<Arc<Storage>>, zerlog_producer_actor: Addr<ZerLogProducerActor>) -> Self{
        Self { app_storage, zerlog_producer_actor }
    }

    pub async fn raw_store(&mut self, message: LocationEventMessage){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let zerlog_producer_actro = self.clone().zerlog_producer_actor;

        // build chrono::DateTime<FixedOffset> from chrono::NaiveDateTime
        let naive_dt = message.clone().date.unwrap();
        let fixed_offset = FixedOffset::east_opt(0).unwrap();
        let dt_fixed_offset = DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_dt, fixed_offset);

        /* -ˋˏ✄┈┈┈┈ saving using raw statement */
        let db_backend = db.get_database_backend(); // postgres in our case
        let insert_stmt = Statement::from_sql_and_values(db_backend, 
                consts::queries::INSERT_QUERY, 
                [
                    Value::String(Some(Box::new(message.clone().imei.unwrap_or_default()))),
                    Value::String(Some(Box::new(message.clone().timestamp.unwrap_or_default()))),
                    Value::String(Some(Box::new(message.clone().correlationId.unwrap_or_default()))),
                    Value::String(Some(Box::new(serde_json::to_string(&message.clone().deviceId.unwrap_or_default()).unwrap()))),
                    Value::String(Some(Box::new(message.clone().imei.unwrap_or_default()))),
                    Value::Double(Some(message.clone().latitude.unwrap_or_default())),
                    Value::Double(Some(message.clone().longitude.unwrap_or_default())),
                    Value::ChronoDateTimeWithTimeZone(Some(Box::new(dt_fixed_offset))),
                    Value::Bool(Some(message.clone().positionStatus.unwrap_or_default())),
                    Value::Double(Some(message.clone().speed.unwrap_or_default())),
                    Value::Double(Some(message.clone().heading.unwrap_or_default())),
                    Value::Double(Some(message.clone().altitude.unwrap_or_default())),
                    Value::BigInt(Some(message.clone().satellites.unwrap_or_default())),
                    Value::BigInt(Some(message.clone().gsmSignal.unwrap_or_default())),
                    Value::Double(Some(message.clone().odometer.unwrap_or_default())),
                    Value::Double(Some(message.clone().hdop.unwrap_or_default())),
                    Value::BigInt(Some(message.clone().mobileCountryCode.unwrap_or_default())),
                    Value::BigInt(Some(message.clone().mobileNetworkCode.unwrap_or_default())),
                    Value::BigInt(Some(message.clone().cellId.unwrap_or_default())),
                    Value::BigInt(Some(message.clone().locationAreaCode.unwrap_or_default())),
                ]
            );

        match db.execute(insert_stmt).await{
            Ok(exec_res) => {
                
                // if we're here means that query has been executed successfully
                // ...

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
                    Some(&zerlog_producer_actro)
                ).await;

                return; // terminate the caller
            }
        }

    }

    pub async fn raw_update(&mut self, message: LocationEventMessage){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();

        // ...

    }

    pub async fn raw_delete(&mut self, location_id: i32){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();

        // ...
        
    }

}

impl Handler<StoreLocationEvent> for LocationMutatorActor{
    
    type Result = ();
    fn handle(&mut self, msg: StoreLocationEvent, ctx: &mut Self::Context) -> Self::Result {

        // unpacking the consumed data
        let StoreLocationEvent { 
                message,
            } = msg.clone(); // the unpacking pattern is always matched so if let ... is useless
        
        let mut this = self.clone();
        tokio::spawn(async move{
            this.raw_store(message.clone()).await;
        });
        
        return;
    }

}