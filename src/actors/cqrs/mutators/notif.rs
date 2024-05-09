
use chrono::{DateTime, FixedOffset, Local};
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, Statement, TryIntoModel, Value};
use serde::{Serialize, Deserialize};
use actix::prelude::*;
use std::future::IntoFuture;
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context};
use crate::actors::producers::zerlog::ZerLogProducerActor;
use crate::entities::hoops;
use crate::actors::producers::notif::ProduceNotif;
use crate::models::event::{NotifData, ReceiverInfo};
use crate::s3::Storage;
use crate::consts::{self, PING_INTERVAL};
use serde_json::json;



#[derive(Message, Clone, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct StoreNotifEvent{
    pub message: ProduceNotif
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NotifInfo{
    pub notif_receiver: ReceiverInfo,
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

        log::info!("received message to store .... {:?}", message);

        // use crate::entities::notifs::ActiveModel;
        // add a single notif data into db 
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
            } = msg.clone(); // the unpacking pattern is always matched so if let ... is useless
        
        let mut this = self.clone();
        
        tokio::spawn(async move{
            this.store(NotifInfo{
                notif_receiver: message.clone().notif_receiver,
                notif_data: message.clone().notif_data
            }).await;
        });
        
        return;
    }

}