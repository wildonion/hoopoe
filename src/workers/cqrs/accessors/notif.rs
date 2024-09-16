

use deadpool_redis::redis::AsyncCommands;
use std::error::Error;
use std::f32::consts::E;
use std::net;
use std::str::FromStr;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context, Handler};
use chrono::{DateTime, FixedOffset};
use constants::STORAGE_IO_ERROR_CODE;
use deadpool_redis::{Connection, Manager, Pool};
use sea_orm::{ConnectionTrait, QueryResult, Statement, Value};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use crate::entities::notifs::{Column, Model as NotifModel, Entity as NotifEntity, ActiveModel as NotifActiveModel};
use sea_orm::ActiveModelTrait;
use crate::models::event::DbNotifData;
use crate::workers::zerlog::ZerLogProducerActor;
use crate::{models::event::NotifData};
use crate::types::RedisPoolConnection;
use crate::storage::engine::Storage;
use crate::constants::PING_INTERVAL;
use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::{constants, entities};
use sea_orm::ActiveValue::Set;


#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "ResponseNotifDataByNotifId")]
pub struct SeenNotif{
    pub notif_id: i32
}

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "ResponseAllNotifData")]
pub struct RequestAllNotifData{
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "ResponseNotifData")]
pub struct RequestNotifData{
    pub owner: Option<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "ResponseNotifDataByNotifId")]
pub struct RequestNotifDataByNotifId{
    pub notif_id: i32
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct NotifDataResponse{
    pub notifs: Vec<NotifModel>,
    pub items: u64,
    pub pages: u64, 
    pub current_page: u64
}

/*
    future trait types as separate objects must be pinned into the ram 
    to break their cycle of self-ref types and add some indirection to 
    them, the return type would be: Box::pin(async move{});
*/
#[derive(MessageResponse)]
pub struct ResponseNotifData(pub std::pin::Pin<Box<dyn std::future::Future<Output = Option<Option<NotifDataResponse>>> + Send + Sync + 'static>>);

#[derive(MessageResponse)]
pub struct ResponseNotifDataByNotifId(pub std::pin::Pin<Box<dyn std::future::Future<Output = Option<Option<DbNotifData>>> + Send + Sync + 'static>>);

#[derive(MessageResponse)]
pub struct ResponseAllNotifData(pub std::pin::Pin<Box<dyn std::future::Future<Output = Option<Option<NotifDataResponse>>> + Send + Sync + 'static>>);

#[derive(Clone)]
pub struct NotifAccessorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
    pub zerlog_producer_actor: Addr<ZerLogProducerActor>
}

impl Actor for NotifAccessorActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 NotifAccessorActor has started");

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

impl NotifAccessorActor{

    pub fn new(app_storage: std::option::Option<Arc<Storage>>, zerlog_producer_actor: Addr<ZerLogProducerActor>) -> Self{
        Self { app_storage, zerlog_producer_actor }
    }

    pub async fn setSeenNotif(&self, notif_id: i32) -> Option<DbNotifData>{

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        

        let notif: Option<NotifModel> = NotifEntity::find_by_id(notif_id).one(db).await.unwrap();
        let mut notifActiveModel: NotifActiveModel = notif.unwrap().into();
        notifActiveModel.is_seen = Set(true);
        let notif: NotifModel = notifActiveModel.clone().update(db).await.unwrap();
        
        Some(
            // building and returning struct in-place, this gets allocated in the caller space on the stack 
            DbNotifData { 
                id: notif.id, 
                nid: notif.nid, 
                receiver_info: notif.receiver_info, 
                action_data: notif.action_data, 
                actioner_info: notif.actioner_info,
                action_type: notif.action_type, 
                fired_at: notif.fired_at, 
                is_seen: notif.is_seen 
            }
        )
        
    }

    pub async fn get_by_notif_id(&self, notif_id: i32) -> Option<DbNotifData>{

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        
        let db_backend = db.get_database_backend();
        let stmt = Statement::from_sql_and_values(
            db_backend, 
            constants::queries::SELECT_BY_NOTIF_ID,
            [
                notif_id.into() // Value::Int(Some())
            ]
        );

        match db.query_one(stmt).await{
            Ok(res) => {

                let res = res.unwrap();
                Some(
                    DbNotifData{
                        id: res.try_get("", "id").unwrap(),
                        receiver_info: res.try_get("", "receiver_info").unwrap(),
                        nid: res.try_get("", "nid").unwrap(),
                        action_data: res.try_get("", "action_data").unwrap(),
                        actioner_info: res.try_get("", "actioner_info").unwrap(),
                        action_type: res.try_get("", "action_type").unwrap(),
                        fired_at: res.try_get("", "fired_at").unwrap(),
                        is_seen: res.try_get("", "is_seen").unwrap(),
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
                    "get_by_notif_id.db.query_one", // method
                    Some(&zerlog_producer_actor)
                ).await;
                return None;
            }
        }

    }
    
    pub async fn get_for_owner(&self, report_info: RequestNotifData) -> Option<NotifDataResponse>{

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;

        let page_size = report_info.page_size.unwrap_or(10);
        let owner = report_info.owner.unwrap_or_default();
        let from = report_info.from.unwrap_or(0);
        let to = report_info.to.unwrap_or(10);

        if from > to{
            return None;
        }

        // fetch unseen notifs for owner
        let mut notifs = NotifEntity::find()
            .filter(Column::ReceiverInfo.contains(&owner))
            .filter(Column::IsSeen.eq(false))
            .order_by_desc(Column::FiredAt)
            .limit((to - from) + 1)
            .offset(from)
            .paginate(db, page_size);

        let items_pages = match notifs.num_items_and_pages().await{
            Ok(itpg) => itpg,
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // the actual source of the error caused at runtime
                    &String::from("NotifAccessorActor.get.num_items_and_pages"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
                return None;
            }
        };

        let mut notif_data = vec![];
        while let Some(notifs) = notifs.fetch_and_next().await.unwrap(){
            notif_data = notifs;
        }

        match redis_pool.get().await{
            Ok(mut redis_conn) => {

                let resp = Some(
                    NotifDataResponse{
                        notifs: notif_data,
                        items: items_pages.number_of_items,
                        pages: items_pages.number_of_pages,
                        current_page: notifs.cur_page()
                    }
                );

                let redis_notif_key = format!("notif_owner_api_resp:{}", &owner);
                let is_key_there: bool = redis_conn.exists(&redis_notif_key).await.unwrap();
                if is_key_there{
                    let _: () = redis_conn.set(redis_notif_key, serde_json::to_string(&resp).unwrap()).await.unwrap();
                } else{
                    let _: () = redis_conn.set_ex(
                        redis_notif_key, 
                        serde_json::to_string(&resp).unwrap(), 
                        std::env::var("REDIS_SESSION_EXP_KEY").unwrap().parse::<u64>().unwrap()
                    ).await.unwrap();
                }

                resp

            },
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                    &String::from("get_notif.redis_pool"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
                return None;
            }
        }

    }

    pub async fn get_all(&self, report_info: RequestAllNotifData) -> Option<NotifDataResponse>{

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        
        let page_size = report_info.page_size.unwrap_or(10);
        let from = report_info.from.unwrap_or(0);
        let to = report_info.to.unwrap_or(10);

        if from > to{
            return None;
        }

        let mut notifs = NotifEntity::find()
            .order_by_desc(Column::FiredAt)
            .limit((to - from) + 1)
            .offset(from)
            .paginate(db, page_size);

        let items_pages = match notifs.num_items_and_pages().await{
            Ok(itpg) => itpg,
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // the actual source of the error caused at runtime
                    &String::from("NotifAccessorActor.get.num_items_and_pages"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
                return None;
            }
        };

        let mut notif_data = vec![];
        // streaming over the entire fetched data
        while let Some(notifs) = notifs.fetch_and_next().await.unwrap(){
            notif_data = notifs;
        }

        match redis_pool.get().await{
            Ok(mut redis_conn) => {

                let resp = Some(
                    NotifDataResponse{
                        notifs: notif_data,
                        items: items_pages.number_of_items,
                        pages: items_pages.number_of_pages,
                        current_page: notifs.cur_page()
                    }
                );

                let redis_notif_key = format!("all_notif_api_resp");
                let is_key_there: bool = redis_conn.exists(&redis_notif_key).await.unwrap();
                if is_key_there{
                    let _: () = redis_conn.set(redis_notif_key, serde_json::to_string(&resp).unwrap()).await.unwrap();
                } else{
                    let _: () = redis_conn.set_ex(
                        redis_notif_key, 
                        serde_json::to_string(&resp).unwrap(), 
                        std::env::var("REDIS_SESSION_EXP_KEY").unwrap().parse::<u64>().unwrap()
                    ).await.unwrap();
                }

                resp

            },
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                    &String::from("get_notif.redis_pool"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
                return None;
            }
        }

    }
    
}


impl Handler<RequestAllNotifData> for NotifAccessorActor{

    type Result = ResponseAllNotifData;

    fn handle(&mut self, msg: RequestAllNotifData, ctx: &mut Self::Context) -> Self::Result {
        
        let RequestAllNotifData{
            ..
        } = msg.clone();

        let this = self.clone();

        // since we need to use async channels to get the resp of this.get() method
        // we need to be inside an async context, that's why we're returning an async
        // object from the method. async objects are future objects they're self-ref types 
        // they need to be pinned into the ram to break the cycle in them and returning
        // future objects must be in form of Box::pin(async move{});
        ResponseAllNotifData(
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
                    in the caller scope we should await on the pinned boxed future
                    to get the result by suspending (not blocking) its execution 
                    in the thread.
                */
                async move{
                    let (tx, mut rx) 
                        = tokio::sync::mpsc::channel::<Option<NotifDataResponse>>(1024);
                    tokio::spawn(async move{
                        let notfis = this.get_all(msg).await;
                        tx.send(notfis).await;
                    });
                    while let Some(get_notifs) = rx.recv().await{
                        return Some(get_notifs);
                    }
                    return None
                }
            )
        )

    }
    
}

impl Handler<RequestNotifData> for NotifAccessorActor{

    type Result = ResponseNotifData;

    fn handle(&mut self, msg: RequestNotifData, ctx: &mut Self::Context) -> Self::Result {
        
        let RequestNotifData{
            ..
        } = msg.clone();

        let this = self.clone();

        // since we need to use async channels to get the resp of this.get() method
        // we need to be inside an async context, that's why we're returning an async
        // object from the method. async objects are future objects they're self-ref types 
        // they need to be pinned into the ram to break the cycle in them and returning
        // future objects must be in form of Box::pin(async move{});
        ResponseNotifData(
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
                    in the caller scope we should await on the pinned boxed future
                    to get the result by suspending (not blocking) its execution 
                    in the thread.
                */
                async move{
                    let (tx, mut rx) 
                        = tokio::sync::mpsc::channel::<Option<NotifDataResponse>>(1024);
                    tokio::spawn(async move{
                        let notfis = this.get_for_owner(msg).await;
                        tx.send(notfis).await;
                    });
                    while let Some(get_notifs) = rx.recv().await{
                        return Some(get_notifs);
                    }
                    return None
                }
            )
        )

    }
    
}

impl Handler<RequestNotifDataByNotifId> for NotifAccessorActor{
    type Result = ResponseNotifDataByNotifId;

    fn handle(&mut self, msg: RequestNotifDataByNotifId, ctx: &mut Self::Context) -> Self::Result {
        
        let RequestNotifDataByNotifId{
            notif_id
        } = msg.clone();

        let this = self.clone();

        // since we need to use async channels to get the resp of this.get() method
        // we need to be inside an async context, that's why we're returning an async
        // object from the method. async objects are future objects they're self-ref types 
        // they need to be pinned into the ram to break the cycle in them and returning
        // future objects must be in form of Box::pin(async move{});
        ResponseNotifDataByNotifId(
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
                    in the caller scope we should await on the pinned boxed future
                    to get the result by suspending (not blocking) its execution 
                    in the thread.
                */
                async move{
                    let (tx, mut rx) 
                        = tokio::sync::mpsc::channel::<Option<DbNotifData>>(1024);
                    tokio::spawn(async move{
                        let notif = this.get_by_notif_id(notif_id).await;
                        tx.send(notif).await;
                    });
                    while let Some(notif_data) = rx.recv().await{
                        return Some(notif_data);
                    }
                    return None;
                }
            )
        )
    }

}


impl Handler<SeenNotif> for NotifAccessorActor{
    type Result = ResponseNotifDataByNotifId;

    fn handle(&mut self, msg: SeenNotif, ctx: &mut Self::Context) -> Self::Result {

        let SeenNotif { notif_id } = msg.clone();
        let this = self.clone();

        // execute the updating seen field task in the background thread
        // since we need to return the updated field from the async contex
        // we should to this in the background means the caller of this 
        // handler must await on the result receiving from the function.
        // but the whole logic inside the async context has already been 
        // executed, we just need to await on it to suspend its execution
        // to receive the result.
        ResponseNotifDataByNotifId(
            /* 
                since we care about the result of the tokio spawn task hence 
                forces us to use channels, also we can't use none async channels
                in async context therefore the whole logic of executing the update
                seen task and returning the updated field would be inside an async
                context which forces us to return the async context from the 
                handler method.
            */
            Box::pin(async move{ // returning future as separate object requires to pin them into the ram
                let (tx, mut rx) = 
                    tokio::sync::mpsc::channel::<Option<DbNotifData>>(1024);

                tokio::spawn(async move{
                    let updated_notif = this.setSeenNotif(notif_id).await;
                    // need to get the updated notif from inside of the tokio spawn threadpool
                    // somehow so sending it through a channe would be a best option
                    tx.send(updated_notif).await;
                });

                while let Some(updated_notif) = rx.recv().await{
                    // return it once we receive it 
                    return Some(updated_notif);
                }
                return None;
            })
        )

    }

}