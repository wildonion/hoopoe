


use std::error::Error;
use std::net;
use std::str::FromStr;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context, Handler};
use actix_web::ResponseError;
use chrono::{DateTime, FixedOffset};
use consts::STORAGE_IO_ERROR_CODE;
use deadpool_redis::{Connection, Manager, Pool};
use redis::{AsyncCommands, Commands};
use sea_orm::{ConnectionTrait, QueryResult, Statement, Value};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use crate::models::event::DbNotifData;
use crate::workers::producers::zerlog::ZerLogProducerActor;
use crate::{workers::consumers, models::event::NotifData};
use crate::types::RedisPoolConnection;
use crate::s3::Storage;
use crate::consts::PING_INTERVAL;
use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::{consts, entities};
use crate::entities::notifs::{
    self, Model as NotifModel, Column as NotifColumn,
    ActiveModel as NotifActiveModel, 
    Entity as NotifEntity
}; // import notif itself and the Entity model


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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NotifDataResponse{
    pub notifs: Vec<notifs::Model>,
    pub items: u64,
    pub pages: u64, 
    pub current_page: u64
}

/*
    future types as separate objects must be pinned into the ram 
    to break their cycle of self-ref types and add some indirection
    so the return type would be: Box::pin(async move{});
*/
#[derive(MessageResponse)]
pub struct ResponseNotifData(pub std::pin::Pin<Box<dyn std::future::Future<Output = Option<Option<NotifDataResponse>>> + Send + Sync + 'static>>);

#[derive(MessageResponse)]
pub struct ResponseNotifDataByNotifId(pub std::pin::Pin<Box<dyn std::future::Future<Output = Option<Option<DbNotifData>>> + Send + Sync + 'static>>);

#[derive(Clone)]
pub struct NotifAccessorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
    pub zerlog_producer_actor: Addr<ZerLogProducerActor>
}

impl Actor for NotifAccessorActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 NotifAccessorActor has started, let's read baby!");

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

    pub async fn get_by_notif_id(&self, notif_id: i32) -> Option<DbNotifData>{

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        
        let db_backend = db.get_database_backend();
        let stmt = Statement::from_sql_and_values(
            db_backend, 
            consts::queries::SELECT_BY_NOTIF_ID,
            [
                Value::Int(Some(notif_id))
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
                    *consts::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // error kind
                    "store_geo.db.execute", // method
                    Some(&zerlog_producer_actor)
                ).await;
                return None;
            }
        }

    }
    
    pub async fn get(&self, report_info: RequestNotifData) -> Option<NotifDataResponse>{

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

        let mut notifs = NotifEntity::find()
            .filter(NotifColumn::ReceiverInfo.contains(&owner))
            .order_by_desc(NotifColumn::FiredAt)
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
            Box::pin(
                // don't use blocking channles in async context, we've used
                // async version of mpsc which requires an async context
                async move{
                    let (tx, mut rx) 
                        = tokio::sync::mpsc::channel::<Option<NotifDataResponse>>(1024);
                    tokio::spawn(async move{
                        let notfis = this.get(msg).await;
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
            Box::pin(
                // don't use blocking channles in async context, we've used
                // async version of mpsc which requires an async context
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