


use std::str::FromStr;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context, Handler};
use chrono::{DateTime, FixedOffset};
use deadpool_redis::{Connection, Manager, Pool};
use redis::{AsyncCommands, Commands};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement, Value};
use crate::actors::consumers;
use crate::types::RedisPoolConnection;
use crate::s3::Storage;
use crate::consts::PING_INTERVAL;
use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::consts;


#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RequestNotifData{}

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

    
    pub async fn get(&self, report_info: RequestNotifData){

        // cache in redis to get the report in main api
        // ...
        
    }
    
}


// a dead handler!
impl Handler<RequestNotifData> for LocationAccessorActor{

    type Result = ();

    fn handle(&mut self, msg: RequestNotifData, ctx: &mut Self::Context) -> Self::Result {
        
        let RequestNotifData{
            ..
        } = msg.clone();

        let this = self.clone();
        
        tokio::spawn(async move{
            
            this.get(msg).await;

        });

    }
    
}