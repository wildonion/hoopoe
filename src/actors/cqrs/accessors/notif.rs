


use std::str::FromStr;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context, Handler};
use chrono::{DateTime, FixedOffset};
use deadpool_redis::{Connection, Manager, Pool};
use redis::{AsyncCommands, Commands};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement, Value};
use crate::{actors::consumers, models::event::NotifData};
use crate::types::RedisPoolConnection;
use crate::s3::Storage;
use crate::consts::PING_INTERVAL;
use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::consts;




#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "ResponseNotifData")]
pub struct RequestNotifData{}

#[derive(MessageResponse)]
pub struct ResponseNotifData(pub std::pin::Pin<Box<dyn std::future::Future<Output = Option<Vec<NotifData>>> + Send + Sync + 'static>>);

#[derive(Clone)]
pub struct NotifAccessorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
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

    pub fn new(app_storage: std::option::Option<Arc<Storage>>) -> Self{
        Self { app_storage }
    }

    
    pub async fn get(&self, report_info: RequestNotifData) -> Vec<NotifData>{

        // cache in redis to get the report in main api
        // ...

        vec![NotifData::default()]
        
    }
    
}


// a dead handler!
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
        // future objects must be in form of Box::pin(async move{})
        ResponseNotifData(
            Box::pin(
                async move{
                    let (tx, mut rx) 
                        = tokio::sync::mpsc::channel::<Vec<NotifData>>(1024);
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