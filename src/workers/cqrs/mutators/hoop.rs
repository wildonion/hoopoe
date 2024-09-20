
use chrono::{DateTime, FixedOffset, Local};
use sea_orm::Set;
use salvo::rate_limiter::QuotaGetter;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Statement, TryIntoModel, Value};
use serde::{Serialize, Deserialize};
use actix::prelude::*;
use std::sync::Arc;
use std::thread;
use actix::{Actor, AsyncContext, Context};
use crate::workers::zerlog::ZerLogProducerActor;
use crate::entities::{self, hoops, users_hoops};
use crate::models::event::{DbHoopData, HoopEventFormForDb};
use crate::entities::hoops::{Column, Model as HoopModel, Entity as HoopEntity, ActiveModel as HoopActiveModel};
use crate::storage::engine::Storage;
use crate::constants::{self, PING_INTERVAL};
use serde_json::json;

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StoreHoopEvent{
    pub hoop: HoopEventFormForDb,
    pub local_spawn: bool
}

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "ResponseHoopData")]
pub struct UpdateIsFinished{
    pub hoop_title: String,
}

#[derive(MessageResponse)]
pub struct ResponseHoopData(pub std::pin::Pin<Box<dyn std::future::Future<Output = Option<Option<DbHoopData>>> + Send + Sync + 'static>>);

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RawStoreHoopEvent{
    pub hoop: HoopEventFormForDb,
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

        log::info!("🎬 HoopMutatorActor has started");

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

    // this would return the updated hoop field
    pub async fn RawFinishHoop(&mut self, hoop_title: String) -> Option<DbHoopData>{

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;

        let db_backend = db.get_database_backend();
        let stmt = Statement::from_sql_and_values(
            db_backend.clone(), 
            constants::queries::UPDATE_IS_FINISHED, 
            [
                hoop_title.clone().into()
            ]
        );

        // update query 
        match db.execute(stmt).await{
            Ok(query_res) => {

                let stmt = Statement::from_sql_and_values(
                    db_backend, 
                    constants::queries::SELECT_HOOP_BY_TITLE, 
                    [
                        hoop_title.clone().into()
                    ]
                );

                // fetch query
                match db.query_one(stmt).await{ // fetch the updated row
                    Ok(exec_res) => {
                        
                        let res = exec_res.unwrap(); // get the query result to extract the rows
                        Some(
                            DbHoopData{
                                id: res.try_get("", "id").unwrap(),
                                etype: res.try_get("", "etype").unwrap(),
                                manager: res.try_get("", "manager").unwrap(),
                                entrance_fee: res.try_get("", "entrance_fee").unwrap(),
                                cover: res.try_get("", "cover").unwrap(),
                                started_at: res.try_get("", "started_at").unwrap(),
                                end_at: res.try_get("", "end_at").unwrap(),
                                title: res.try_get("", "title").unwrap(),
                                description: res.try_get("", "description").unwrap(),
                                capacity: res.try_get("", "capacity").unwrap(),
                                duration: res.try_get("", "duration").unwrap(),
                                created_at: res.try_get("", "created_at").unwrap(),
                                updated_at: res.try_get("", "updated_at").unwrap(),
                            }
                        )
                    },
                    Err(e) => {
                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                        let error_content = &e.to_string();
                        let error_content = error_content.as_bytes().to_vec();
                        let mut error_instance = HoopoeErrorResponse::new(
                            *constants::STORAGE_IO_ERROR_CODE, // error code
                            error_content, // error content
                            ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                            "HoopMutatorActor.FinishHoop.db.query_one", // method
                            Some(&zerlog_producer_actor)
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
                    *constants::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                    "HoopMutatorActor.FinishHoop.db.execute", // method
                    Some(&zerlog_producer_actor)
                ).await;
                return None; // terminate the caller
            }
        }
    }

    // this would return the updated hoop field
    pub async fn FinishHoop(&mut self, hoop_title: String) -> Option<DbHoopData>{

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;

        let hoop: Option<HoopModel> = HoopEntity::find()
            .filter(Column::Title.contains(&hoop_title))
            .one(db)
            .await
            .unwrap();

        let mut hoop_active_model: HoopActiveModel = hoop.unwrap().into();
        hoop_active_model.is_finished = Set(Some(true));
        let hoop: HoopModel = hoop_active_model.clone().update(db).await.unwrap();

        Some(
            DbHoopData{
                id: hoop_active_model.id.take().unwrap(),
                etype: hoop_active_model.etype.take().unwrap(),
                manager: hoop_active_model.manager.take().unwrap(),
                entrance_fee: hoop_active_model.entrance_fee.take().unwrap(),
                title: hoop_active_model.title.take().unwrap(),
                description: hoop_active_model.description.take().unwrap(),
                started_at: hoop_active_model.started_at.take().unwrap().timestamp(),
                end_at: hoop_active_model.end_at.take().unwrap().timestamp(),
                duration: hoop_active_model.duration.take().unwrap(),
                capacity: hoop_active_model.capacity.take().unwrap(),
                cover: hoop_active_model.cover.take().unwrap(),
                created_at: hoop_active_model.created_at.take().unwrap(),
                updated_at: hoop_active_model.updated_at.take().unwrap(),
            }
        )

    }

    pub async fn raw_store(&mut self, hoop: HoopEventFormForDb) -> Option<DbHoopData>{

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;

        let db_backend = db.get_database_backend();
        let stmt = Statement::from_sql_and_values(
            db_backend, 
            constants::queries::INSERT_HOOP, 
            [ // pass the followings to the query
                hoop.etype.into(),
                hoop.manager.into(),
                hoop.entrance_fee.into(),
                hoop.title.into(),
                hoop.description.into(),
                hoop.duration.into(),
                hoop.capacity.into(),
                hoop.cover.into(),
                hoop.started_at.into(),
                hoop.end_at.into(),
            ]
        );

        match db.execute(stmt).await{ // execute the query
            Ok(query_res) => {

                let id = query_res.last_insert_id(); // get the inserted id           
                let stmt = Statement::from_sql_and_values(
                    db_backend, 
                    constants::queries::SELECT_HOOP_BY_ID, 
                    [
                        id.into()
                    ]
                );

                match db.query_one(stmt).await{ // fetch the last inserted row
                    Ok(exec_res) => {
                        
                        let res = exec_res.unwrap(); // get the query result to extract the rows
                        Some(
                            DbHoopData{
                                id: res.try_get("", "id").unwrap(),
                                etype: res.try_get("", "etype").unwrap(),
                                manager: res.try_get("", "manager").unwrap(),
                                entrance_fee: res.try_get("", "entrance_fee").unwrap(),
                                cover: res.try_get("", "cover").unwrap(),
                                started_at: res.try_get("", "started_at").unwrap(),
                                end_at: res.try_get("", "end_at").unwrap(),
                                title: res.try_get("", "title").unwrap(),
                                description: res.try_get("", "description").unwrap(),
                                capacity: res.try_get("", "capacity").unwrap(),
                                duration: res.try_get("", "duration").unwrap(),
                                created_at: res.try_get("", "created_at").unwrap(),
                                updated_at: res.try_get("", "updated_at").unwrap(),
                            }
                        )
                    },
                    Err(e) => {
                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                        let error_content = &e.to_string();
                        let error_content = error_content.as_bytes().to_vec();
                        let mut error_instance = HoopoeErrorResponse::new(
                            *constants::STORAGE_IO_ERROR_CODE, // error code
                            error_content, // error content
                            ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                            "HoopMutatorActor.raw_store.db.query_one", // method
                            Some(&zerlog_producer_actor)
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
                    *constants::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                    "HoopMutatorActor.raw_store.db.execute", // method
                    Some(&zerlog_producer_actor)
                ).await;
                return None; // terminate the caller
            }
        }

    }
    
    pub async fn store(&mut self, hoop: HoopEventFormForDb) -> Option<entities::hoops::Model>{
        
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
                "description": hoop.description,
                "title": hoop.title,
                "capacity": hoop.capacity,
                "duration": hoop.duration,
                "started_at": hoop.started_at,
                "end_at": hoop.end_at,
                "cover": hoop.cover,
            })
        ){
            Ok(_) => {},
            Err(e) => {
                use crate::error::{ErrorKind, HoopoeErrorResponse};
                let error_content = &e.to_string();
                let error_content = error_content.as_bytes().to_vec();
                let mut error_instance = HoopoeErrorResponse::new(
                    *constants::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                    "hoop_active_model.set_from_json", // method
                    Some(&zerlog_producer_actor)
                ).await;
                return None; // terminate the caller
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

                        // return inserted model
                        return Some(model);

                    },
                    Err(e) => {
                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                        let error_content = &e.to_string();
                        let error_content = error_content.as_bytes().to_vec();
                        let mut error_instance = HoopoeErrorResponse::new(
                            *constants::STORAGE_IO_ERROR_CODE, // error code
                            error_content, // error content
                            ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                            "hoop_active_model.save.try_into_model", // method
                            Some(&zerlog_producer_actor)
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
                    *constants::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                    "hoop_active_model.save", // method
                    Some(&zerlog_producer_actor)
                ).await;
                return None; // terminate the caller   
            }
        }

    }

    pub async fn update(&mut self, hoop: HoopEventFormForDb){

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

impl Handler<RawStoreHoopEvent> for HoopMutatorActor{
    type Result = ();
    fn handle(&mut self, msg: RawStoreHoopEvent, ctx: &mut Self::Context) -> Self::Result {
        
        // unpacking the consumed data
        let RawStoreHoopEvent{
            hoop,
            local_spawn
        } = msg.clone();

        let mut this = self.clone();
        
        // store the hoop in a light io thread since 
        // db operations are io thread must not get blocked
        if local_spawn{
            async move{
                this.raw_store(hoop).await;
            }.into_actor(self)
            .spawn(ctx);
        } else{
            tokio::spawn(async move{
                this.raw_store(hoop).await;
            });
        }

        return;

    }
}

impl Handler<UpdateIsFinished> for HoopMutatorActor{
    
    type Result = ResponseHoopData;
    fn handle(&mut self, msg: UpdateIsFinished, ctx: &mut Self::Context) -> Self::Result {
        
        let UpdateIsFinished{hoop_title} = msg;
        let mut this = self.clone();
        
        // since we need to use async channels to get the resp of this.get() method
        // we need to be inside an async context, that's why we're returning an async
        // object from the method. async objects are future objects they're self-ref types 
        // they need to be pinned into the ram to break the cycle in them and returning
        // future objects must be in form of Box::pin(async move{});
        ResponseHoopData(
            Box::pin( // future objects as separate type needs to get pinned
                // don't use blocking channles in async context, we've used
                // async version of mpsc which requires an async context
                /*
                    pinning a boxed future would be a great option if we need to 
                    execute async tasks like receiving from a an async channel inside 
                    a none async scope like actor message handler functions, we can 
                    wrap the async job around an async move{} and then pin the async 
                    move{} scope to return it from the function, later when we invoke 
                    the actual function we can catch the result which contains the 
                    async future object and get the result of the future object by 
                    awaiting on it which tells runtime to suspend the function execution 
                    in there but don’t block the thread, continuing executing other 
                    tasks in the current thread either in background (don’t await) or 
                    by suspending (await) the task. 
                */
                async move{
                    let (tx, mut rx) 
                        = tokio::sync::mpsc::channel::<Option<DbHoopData>>(1024);
                    // since we're executing the updating task inside 
                    // in the background light io threads we need to use
                    // channels to receive the data outside of the thread
                    // to return it as the return data of the async block 
                    tokio::spawn(async move{
                        let updated_hoop = this.FinishHoop(hoop_title).await;
                        tx.send(updated_hoop).await;
                    });
                    while let Some(updated_hoop) = rx.recv().await{
                        return Some(updated_hoop);
                    }
                    return None;
                }
            )
        )

    }
} 