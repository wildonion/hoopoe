


/* -ˋˏ✄┈┈┈┈
    the producing task has been started by sending the ProduceNotif message 
    to this actor which will execute the publishing process to the exchange 
    in either the notif producer actor context itself or the tokio spawn thread:

    notif producer -----payload-----> Exchange
*/

use crate::*;
use actix::prelude::*;
use actix::{AsyncContext, Context};
use actix_redis::{resp_array, Command, RespValue};
use config::EnvExt;
use consts::PING_INTERVAL;
use deadpool_lapin::lapin::options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions};
use deadpool_lapin::lapin::protocol::exchange;
use deadpool_lapin::lapin::types::FieldTable;
use deadpool_lapin::lapin::BasicProperties;
use interfaces::*;
use interfaces::notif::NotifExt;
use std::sync::Arc;
use crate::models::event::*;
use super::zerlog::ZerLogProducerActor;



#[derive(Message, Clone, Serialize, Deserialize, Debug, Default)]
#[rtype(result = "()")]
pub struct ProduceNotif{
    pub local_spawn: bool, // either spawn in actor context or tokio threadpool
    pub notif_data: NotifData,
    pub exchange_name: String,
    pub exchange_type: String,
    pub routing_key: String,
}

#[derive(Clone)]
pub struct NotifProducerActor{
    pub app_storage: Option<Arc<s3::Storage>>,
    pub zerlog_producer_actor: Addr<ZerLogProducerActor>,
} 

impl Actor for NotifProducerActor{
    
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 NotifProducerActor has started, let's produce baby!");

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

impl NotifProducerActor{

    pub async fn produce(&self, data: &str, exchange: &str, routing_key: &str, exchange_type: &str){

        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
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
                            /* 
                                you should set the auto_delete flag to True when declaring the exchange. This will 
                                automatically delete the exchange when all channels are done with it.
                                Keep in mind that this means that it will stay as long as there is an active binding 
                                to the exchange. If you delete the binding, or queue, the exchange will be deleted.
                                if you need to keep the queue, but not the exchange you can remove the binding once 
                                you are done publishing. This should automatically remove the exchange.
                                so when all bindings (queues and exchanges) get deleted the exchange will be deleted.
                            */
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
                                    Ok(ok) => {ok},
                                    Err(e) => {
                                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                                        let e_string = &e.to_string();
                                        let error_content = e_string.as_bytes().to_vec();
                                        let mut error_instance = HoopoeErrorResponse::new(
                                            *consts::STORAGE_IO_ERROR_CODE, // error code
                                            error_content, // error content
                                            ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                            "NotifProducerActor.exchange_declare", // method
                                            Some(&zerlog_producer_actor)
                                        ).await;
                                        
                                    }

                                };

                            // async task: publish messages to exchange in the background in a free thread
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
                                        BasicProperties::default().with_content_type("application/json".into()),
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
                                                    *consts::STORAGE_IO_ERROR_CODE, // error code
                                                    error_content, // error content
                                                    ErrorKind::Storage(crate::error::StorageError::Rmq(error_content_)), // error kind
                                                    "NotifProducerActor.get_confirmation", // method
                                                    Some(&zerlog_producer_actor)
                                                ).await;

                                                return;
                                            };

                                        },
                                        Err(e) => {
                                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                                            let error_content = &e.to_string();
                                            let error_content = error_content.as_bytes().to_vec();
                                            let mut error_instance = HoopoeErrorResponse::new(
                                                *consts::STORAGE_IO_ERROR_CODE, // error code
                                                error_content, // error content
                                                ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                                "NotifProducerActor.basic_publish", // method
                                                Some(&zerlog_producer_actor)
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
                                *consts::STORAGE_IO_ERROR_CODE, // error code
                                error_content, // error content
                                ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                "NotifProducerActor.create_channel", // method
                                Some(&zerlog_producer_actor)
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
                        *consts::STORAGE_IO_ERROR_CODE, // error code
                        error_content, // error content
                        ErrorKind::Storage(crate::error::StorageError::RmqPool(e)), // error kind
                        "NotifProducerActor.produce_pool", // method
                        Some(&zerlog_producer_actor)
                    ).await;

                    return;
                }
            };
            
        });
        
    
    }

    pub fn new(app_storage: std::option::Option<Arc<s3::Storage>>, zerlog_producer_actor: Addr<ZerLogProducerActor>) -> Self{
        Self { app_storage, zerlog_producer_actor }
    }

}

impl Handler<ProduceNotif> for NotifProducerActor{
    
    type Result = ();
    fn handle(&mut self, msg: ProduceNotif, ctx: &mut Self::Context) -> Self::Result {

        // unpacking the notif data
        let ProduceNotif { 
                exchange_name,
                exchange_type,
                routing_key,
                local_spawn,
                notif_data,
            } = msg.clone();
        
        let stringified_data = serde_json::to_string_pretty(&notif_data).unwrap();
        let this = self.clone();

        // spawn the future in the background into the give actor context
        if local_spawn{
            async move{
                this.produce(&stringified_data, &exchange_name, &routing_key, &exchange_type).await;
            }
            .into_actor(self)
            .spawn(ctx); // spawn the future object into this actor context thread
        } else{ // spawn the future in the background into the tokio lightweight thread
            tokio::spawn(async move{
                this.produce(&stringified_data, &exchange_name, &routing_key, &exchange_type).await;
            });
        }
        
        return;
        
    }

}