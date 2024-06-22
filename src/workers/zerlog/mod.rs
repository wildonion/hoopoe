


use crate::*;
use actix::prelude::*;
use actix::Handler as ActixHandler;
use actix::{AsyncContext, Context};
use config::EnvExt;
use constants::PING_INTERVAL;
use deadpool_lapin::lapin::options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions};
use deadpool_lapin::lapin::protocol::exchange;
use deadpool_lapin::lapin::types::FieldTable;
use deadpool_lapin::lapin::BasicProperties;
use interfaces::*;
use std::sync::Arc;
use crate::models::event::*;



#[derive(Message, Clone, Serialize, Deserialize, Debug, Default)]
#[rtype(result = "()")]
pub struct ProduceNotif{
    pub notif_data: NotifData,
    pub exchange_name: String,
    pub exchange_type: String,
    pub routing_key: String,
}

#[derive(Clone, Default)]
pub struct ZerLogProducerActor{
    pub app_storage: Option<Arc<storage::engine::Storage>>,
}

impl Actor for ZerLogProducerActor{
    
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 ZerLogProducerActor has started");

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

impl ZerLogProducerActor{

    pub async fn produce(&self, data: &str, exchange: &str, routing_key: &str, exchange_type: &str){

        let this = self.clone();

        // these are must be converted into String first to make longer lifetime 
        // cause &str can't get moved into tokio spawn as its lifetime it's not 
        // static the tokio spawn lives longer than the &str and the &str gets 
        // dropped out of the ram once the function is finished with executing
        let exchange = exchange.to_string();
        let routing_key = routing_key.to_string();
        let exchange_type = exchange_type.to_string();
        let data = data.to_string();

        tokio::spawn(async move{

            let storage = this.clone().app_storage.clone();
            let rmq_pool = storage.unwrap().get_lapin_pool().await.unwrap();
            
            // trying to ge a connection from the pool
            match rmq_pool.get().await{
                Ok(pool) => {

                    // -ˋˏ✄┈┈┈┈ creating a channel in this thread
                    match pool.create_channel().await{
                        Ok(chan) => {

                            let mut ex_options = ExchangeDeclareOptions::default();
                            ex_options.auto_delete = true; // the exchange can only be deleted automatically if all bindings are deleted
                            ex_options.durable = true; // durability is the ability to restore data on node shutdown

                            // -ˋˏ✄┈┈┈┈ creating exchange
                            match chan
                                .exchange_declare(&exchange, {
                                    match exchange_type.as_str(){
                                        "fanout" => deadpool_lapin::lapin::ExchangeKind::Fanout,
                                        "direct" => deadpool_lapin::lapin::ExchangeKind::Direct,
                                        "headers" => deadpool_lapin::lapin::ExchangeKind::Headers,
                                        _ => deadpool_lapin::lapin::ExchangeKind::Topic,
                                    }
                                }, 
                                    ex_options, FieldTable::default()
                                )
                                .await
                                {
                                    Ok(ex) => ex,
                                    Err(e) => {
                                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                                        let e_string = &e.to_string();
                                        let error_content = e_string.as_bytes().to_vec();
                                        let mut error_instance = HoopoeErrorResponse::new(
                                            *constants::STORAGE_IO_ERROR_CODE, // error code
                                            error_content, // error content
                                            ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                            "ZerLogProducerActor.exchange_declare", // method
                                            None
                                        ).await;

                                        return;   
                                    }

                                };

                            tokio::spawn(async move{

                                // -ˋˏ✄┈┈┈┈ publishing to exchange from this channel,
                                // later consumer bind its queue to this exchange and its
                                // routing key so messages go inside its queue, later they 
                                // can be consumed from the queue by the consumer
                                use deadpool_lapin::lapin::options::BasicPublishOptions;
                                let payload = data.as_bytes();
                                match chan
                                    .basic_publish(
                                        &exchange, // the way of sending messages
                                        &routing_key, // the way that message gets routed to the queue based on a unique routing key
                                        BasicPublishOptions::default(),
                                        payload, // this is the ProduceNotif data,
                                        BasicProperties::default(),
                                    )
                                    .await
                                    {
                                        Ok(pc) => {
                                            let get_confirmation = pc.await;
                                            let Ok(confirmation) = get_confirmation else{
                                                use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                let error_content_ = get_confirmation.unwrap_err();
                                                let e_string = &error_content_.to_string();
                                                let error_content = e_string.as_bytes().to_vec();
                                                let mut error_instance = HoopoeErrorResponse::new(
                                                    *constants::STORAGE_IO_ERROR_CODE, // error code
                                                    error_content, // error content
                                                    ErrorKind::Storage(crate::error::StorageError::Rmq(error_content_)), // error kind
                                                    "ZerLogProducerActor.get_confirmation", // method
                                                    None
                                                ).await;

                                                return;
                                            };

                                        },
                                        Err(e) => {
                                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                                            let error_content = &e.to_string();
                                            let error_content = error_content.as_bytes().to_vec();
                                            let mut error_instance = HoopoeErrorResponse::new(
                                                *constants::STORAGE_IO_ERROR_CODE, // error code
                                                error_content, // error content
                                                ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                                "ZerLogProducerActor.basic_publish", // method
                                                None
                                            ).await;

                                            return;
                                        }
                                    }
                            });

                        },
                        Err(e) => {
                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                            let error_content = &e.to_string();
                            let error_content = error_content.as_bytes().to_vec();
                            let mut error_instance = HoopoeErrorResponse::new(
                                *constants::STORAGE_IO_ERROR_CODE, // error code
                                error_content, // error content
                                ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                "ZerLogProducerActor.create_channel", // method
                                None
                            ).await;

                            return;
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
                        ErrorKind::Storage(crate::error::StorageError::RmqPool(e)), // error kind
                        "ZerLogProducerActor.produce_pool", // method
                        None
                    ).await;

                    return;
                }
            };
            
        });
        
    
    }

    pub fn new(app_storage: std::option::Option<Arc<storage::engine::Storage>>) -> Self{
        Self { app_storage }
    }

}

impl ActixHandler<ProduceNotif> for ZerLogProducerActor{
    
    type Result = ();
    fn handle(&mut self, msg: ProduceNotif, ctx: &mut Self::Context) -> Self::Result {

        // unpacking the notif data
        let ProduceNotif { 
                exchange_name,
                exchange_type,
                routing_key,
                .. // notif_receiver and notif_data
            } = msg.clone();
        
        let stringified_data = serde_json::to_string_pretty(&msg).unwrap();
        
        let this = self.clone();
        tokio::spawn(async move{
            this.produce(&stringified_data, &exchange_name, &routing_key, &exchange_type).await;
        });
        
        return;
    }

}