


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


#[derive(Clone)]
pub struct HoopAccessorActor{
    pub app_storage: std::option::Option<Arc<Storage>>,
}

impl Actor for HoopAccessorActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 HoopAccessorActor has started, let's read baby!");

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

    pub fn new(app_storage: std::option::Option<Arc<Storage>>) -> Self{
        Self { app_storage }
    }

}