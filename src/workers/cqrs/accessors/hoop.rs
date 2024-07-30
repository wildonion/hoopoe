


use std::str::FromStr;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context, Handler as ActixHandler};
use chrono::{DateTime, FixedOffset};
use deadpool_redis::{Connection, Manager, Pool};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement, Value};
use crate::workers::zerlog::ZerLogProducerActor;
use crate::types::RedisPoolConnection;
use crate::storage::engine::Storage;
use crate::constants::PING_INTERVAL;
use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::constants;




#[derive(Message, Clone, Serialize, Deserialize, Debug, Default)]
#[rtype(result = "()")]
pub struct GetHoop{
    pub id: Option<i32>,   
}

#[derive(Clone)]
pub struct HoopAccessorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
    pub zerlog_producer_actor: Addr<ZerLogProducerActor>
}

impl Actor for HoopAccessorActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 HoopAccessorActor has started");

        let borrowed = &(*self);

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

impl HoopAccessorActor{

    pub fn new(app_storage: std::option::Option<Arc<Storage>>, zerlog_producer_actor: Addr<ZerLogProducerActor>) -> Self{
        Self { app_storage, zerlog_producer_actor }
    }

    pub async fn get(&self, hoop_id: i32){

        let storage = self.app_storage.as_ref().clone().unwrap();
        let db = storage.get_seaorm_pool().await.unwrap();
        let redis_pool = storage.get_redis_pool().await.unwrap();
        
        // cache in redis with expirable key 
        // retrieve it in main api to return it
        // ...

    }

}

impl ActixHandler<GetHoop> for HoopAccessorActor{
    type Result = ();
    fn handle(&mut self, msg: GetHoop, ctx: &mut Self::Context) -> Self::Result {
        
    }
}