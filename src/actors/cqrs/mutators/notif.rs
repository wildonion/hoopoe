
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, Statement, TryIntoModel, Value};
use serde::{Serialize, Deserialize};
use actix::prelude::*;
use std::future::IntoFuture;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context};
use crate::actors::consumers::notif;
use crate::actors::producers::zerlog::ZerLogProducerActor;
use crate::entities::hoops;
use crate::actors::producers::notif::ProduceNotif;
use crate::models::event::NotifData;
use crate::interfaces::passport;
use crate::s3::Storage;
use crate::consts::{self, PING_INTERVAL};
use serde_json::json;
use crate::entities::*;



#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StoreNotifEvent{
    pub message: NotifData,
    pub local_spawn: bool
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NotifInfo{
    pub notif_data: NotifData,
}

#[derive(Clone)]
pub struct NotifMutatorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
    pub zerlog_producer_actor: Addr<ZerLogProducerActor>
}

impl Actor for NotifMutatorActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 NotifMutatorActor has started, let's mutate baby!");

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

impl NotifMutatorActor{

    pub fn new(app_storage: std::option::Option<Arc<Storage>>, zerlog_producer_actor: Addr<ZerLogProducerActor>) -> Self{
        Self { app_storage, zerlog_producer_actor }
    }

    pub async fn store(&mut self, message: NotifInfo){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;

        let notif_data = message.notif_data;
        let naive_datetime = DateTime::from_timestamp(notif_data.fired_at, 0).unwrap().naive_local();
        
        let mut new_notif: notifs::ActiveModel = Default::default();
        let _ = match new_notif.set_from_json(json!(
            {
                "receiver_info": notif_data.receiver_info,
                "nid": notif_data.id,
                "action_data": notif_data.action_data,
                "actioner_info": notif_data.actioner_info,
                "action_type": format!("{:?}", notif_data.action_type),
                "fired_at": naive_datetime,
                "is_seen": notif_data.is_seen
            }
        )){
            Ok(ok) => {},
            Err(e) => {
                use crate::error::{ErrorKind, HoopoeErrorResponse};
                let error_content = &e.to_string();
                let error_content = error_content.as_bytes().to_vec();
                let mut error_instance = HoopoeErrorResponse::new(
                    *consts::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                    "notif_active_model.set_from_json", // method
                    Some(&zerlog_producer_actor)
                ).await;

                return; // terminate the caller
            }
        };

        if new_notif.is_changed(){
            log::info!("notif active model has changed");
        }

        /* -ˋˏ✄┈┈┈┈ saving notif event active model
            An ActiveModel has all the attributes of Model wrapped in ActiveValue, an ActiveValue 
            is a wrapper structand to capture the changes made to ActiveModel attributes like it has
            Set and NotSet struct to change the state of the actual model (row), it's a model or row 
            that is about to be inserted into db by eiter calling save() or insert() methods
            when saving an ActiveModel, it will perform either insert or update depending 
            on the primary key attribute:
                insert if primary key is NotSet
                update if primary key is Set or Unchanged
        */
        match new_notif.save(db).await{
            Ok(active_model) => {

                let get_model = active_model.try_into_model();
                match get_model{
                    Ok(model) => {

                        log::info!("inerted notif id : {}", model.id);

                    },
                    Err(e) => {
                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                        let error_content = &e.to_string();
                        let error_content = error_content.as_bytes().to_vec();
                        let mut error_instance = HoopoeErrorResponse::new(
                            *consts::STORAGE_IO_ERROR_CODE, // error code
                            error_content, // error content
                            ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                            "noif_active_model.save.try_into_model", // method
                            Some(&zerlog_producer_actor)
                        ).await;

                        return; // terminate the caller
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
                    "noif_active_model.save", // method
                    Some(&zerlog_producer_actor)
                ).await;

                return; // terminate the caller   
            }
        }

    }

    pub async fn update(&mut self, notif_id: i32){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();

        // probably true the is_seen flag
        // ...
        
    }

    pub async fn delete(&mut self, notif_id: i32){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();

        // ...
        
    }

}

impl Handler<StoreNotifEvent> for NotifMutatorActor{
    
    type Result = ();
    fn handle(&mut self, msg: StoreNotifEvent, ctx: &mut Self::Context) -> Self::Result {

        let this_address = ctx.address();

        // unpacking the consumed data
        let StoreNotifEvent { 
                message,
                local_spawn
            } = msg.clone(); // the unpacking pattern is always matched so if let ... is useless
        
        let mut this = self.clone();
        
        if local_spawn{
            async move{
                this.store(NotifInfo{
                    notif_data: message.clone()
                }).await;
            }
            .into_actor(self)
            .spawn(ctx); // spawn the future object into this actor context thread
        } else{
            tokio::spawn(async move{
                this.store(NotifInfo{
                    notif_data: message.clone()
                }).await;
            });
        }
        
        return;
    }

}