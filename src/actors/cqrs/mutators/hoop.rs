
use chrono::{DateTime, FixedOffset, Local};
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, Statement, TryIntoModel, Value};
use serde::{Serialize, Deserialize};
use actix::prelude::*;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context};
use crate::actors::producers::zerlog::ZerLogProducerActor;
use crate::entities::hoops;
use crate::models::event::HoopEvent;
use crate::s3::Storage;
use crate::consts::{self, PING_INTERVAL};
use serde_json::json;

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StoreHoopEvent{
    pub hoop: HoopEvent,
    pub local_spawn: bool
}



#[derive(Clone)]
pub struct HoopMutatorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
    pub zerlog_producer_actor: Addr<ZerLogProducerActor>
}

impl Actor for HoopMutatorActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 HoopMutatorActor has started, let's mutate baby!");

        ctx.run_interval(PING_INTERVAL, |actor, ctx|{
            
            let this = actor.clone();

            tokio::spawn(async move{

                // check something constantly, schedule to be executed 
                // repeatedly at a certain time in the background
                // ...
                
            });

        });

    }
}

impl HoopMutatorActor{

    pub fn new(app_storage: std::option::Option<Arc<Storage>>, zerlog_producer_actor: Addr<ZerLogProducerActor>) -> Self{
        Self { app_storage, zerlog_producer_actor }
    }
    
    pub async fn store(&mut self, hoop: HoopEvent){
        
        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;

        /* -ˋˏ✄┈┈┈┈ saving using active model */
        let mut hoop_active_model: hoops::ActiveModel = Default::default();
        let _ = match hoop_active_model.set_from_json(
            json!({
                "etype": hoop.etype,
                "manager": hoop.manager,
                "entrance_fee": hoop.entrance_fee,
            })
        ){
            Ok(_) => {},
            Err(e) => {
                use crate::error::{ErrorKind, HoopoeErrorResponse};
                let error_content = &e.to_string();
                let error_content = error_content.as_bytes().to_vec();
                let mut error_instance = HoopoeErrorResponse::new(
                    *consts::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                    "hoop_active_model.set_from_json", // method
                    Some(&zerlog_producer_actor)
                ).await;

                return; // terminate the caller
            }
        };
        
        if hoop_active_model.is_changed(){
            log::info!("active model has changed");
        }

        /* -ˋˏ✄┈┈┈┈ saving hoop event active model
            An ActiveModel has all the attributes of Model wrapped in ActiveValue, an ActiveValue 
            is a wrapper structand to capture the changes made to ActiveModel attributes like it has
            Set and NotSet struct to change the state of the actual model (row), it's a model or row 
            that is about to be inserted into db by eiter calling save() or insert() methods
            when saving an ActiveModel, it will perform either insert or update depending 
            on the primary key attribute:
                insert if primary key is NotSet
                update if primary key is Set or Unchanged
        */
        match hoop_active_model.save(db).await{
            Ok(active_model) => {

                let get_model = active_model.try_into_model();
                match get_model{
                    Ok(model) => {

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
                            "hoop_active_model.save.try_into_model", // method
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
                    "hoop_active_model.save", // method
                    Some(&zerlog_producer_actor)
                ).await;

                return; // terminate the caller   
            }
        }

    }

    pub async fn update(&mut self, hoop: HoopEvent){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();

        // ...

    }

    pub async fn delete(&mut self, hoop_id: i32){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();

        // ...
        
    }

}

impl Handler<StoreHoopEvent> for HoopMutatorActor{
    
    type Result = ();
    fn handle(&mut self, msg: StoreHoopEvent, ctx: &mut Self::Context) -> Self::Result {

        // unpacking the consumed data
        let StoreHoopEvent { 
                hoop,
                local_spawn
            } = msg.clone(); // the unpacking pattern is always matched so if let ... is useless
        
        let mut this = self.clone();

        if local_spawn{
            async move{
                this.store(hoop.clone()).await;
            }
            .into_actor(self)
            .spawn(ctx); // spawn the future object into this actor context thread
        } else{
            tokio::spawn(async move{
                this.store(hoop.clone()).await;
            });
        }
        
        return;
    }

}