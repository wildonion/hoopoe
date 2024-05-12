
use std::collections::HashMap;
use actix::{Actor, Addr};
use crate::actors::cqrs::accessors::hoop::HoopAccessorActor;
use crate::actors::cqrs::mutators::hoop::HoopMutatorActor;
use crate::actors::cqrs::mutators::notif::NotifMutatorActor;
use crate::actors::cqrs::accessors::notif::NotifAccessorActor;
use crate::config::{Env as ConfigEnv, Context};
use crate::config::EnvExt;
use crate::s3::Storage;
use crate::actors::consumers::notif::NotifConsumerActor;
use crate::actors::producers::notif::NotifProducerActor;
use crate::actors::producers::zerlog::ZerLogProducerActor;
use serde::{Serialize, Deserialize};
use crate::types::*;
use crate::consts::*;
use crate::storage;
use crate::actors::ws::servers::hoop::HoopServer;


#[derive(Clone)]
pub struct WsActors{
    pub hoop_server_actor: Addr<HoopServer>,
}

#[derive(Clone)]
pub struct ConsumerActors{
    pub notif_actor: Addr<NotifConsumerActor>,
}

#[derive(Clone)]
pub struct ProducerActors{
    pub notif_actor: Addr<NotifProducerActor>,
    pub zerlog_actor: Addr<ZerLogProducerActor>,
}

#[derive(Clone)]
pub struct MutatorActors{
    pub notif_mutator_actor: Addr<NotifMutatorActor>,
    pub hoop_mutator_actor: Addr<HoopMutatorActor>,
}

#[derive(Clone)]
pub struct AccessorActors{
    pub notif_accessor_actor: Addr<NotifAccessorActor>,
    pub hoop_accessor_actor: Addr<HoopAccessorActor>
}

#[derive(Clone)]
pub struct CqrsActors{
    pub mutators: MutatorActors,
    pub accessors: AccessorActors
}

#[derive(Clone)]
pub struct ActorInstaces{
    pub consumer_actors: ConsumerActors,
    pub producer_actors: ProducerActors,
    pub ws_actors: WsActors,
    pub cqrs_actors: CqrsActors,
}

#[derive(Clone)]
// NO need to store publisher actors cause they have one method 
// called emit which is used to publish data into redis channel
pub struct AppState{
    pub config: Option<std::sync::Arc<Context<ConfigEnv>>>,
    pub app_storage: Option<std::sync::Arc<Storage>>,
    pub actors: Option<ActorInstaces>, // redis subscriber actors
    pub ramdb: RamDb

}

impl AppState{

    pub async fn init() -> Self{

        let env = ConfigEnv::default();
        let ctx_env = env.get_vars();
        let configs = Some(
            std::sync::Arc::new(ctx_env)
        );

        let app_storage = storage!{ // this publicly has exported inside the misc so we can access it here 
            configs.as_ref().unwrap().vars.clone().DB_NAME,
            configs.as_ref().unwrap().vars.clone().DB_ENGINE,
            configs.as_ref().unwrap().vars.POSTGRES_HOST,
            configs.as_ref().unwrap().vars.POSTGRES_PORT,
            configs.as_ref().unwrap().vars.POSTGRES_USER,
            configs.as_ref().unwrap().vars.POSTGRES_PASSWORD
        }.await;
        
        /* 
            actix web spawn threads separately and move the application factory instance 
            in each thread to handle requests concrrently, actix however uses a shread
            threadpool within the actor system instead of using a threadpool per each 
            actor objects this logic prevents runtime overhead.
        */
        let hoop_ws_server_instance = HoopServer::new(app_storage.clone()).start();
        let zerlog_producer_actor = ZerLogProducerActor::new(app_storage.clone()).start();
        
        let notif_producer_actor = NotifProducerActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let notif_mutator_actor = NotifMutatorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let notif_accessor_actor = NotifAccessorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let notif_consumer_actor = NotifConsumerActor::new(app_storage.clone(), notif_mutator_actor.clone(), zerlog_producer_actor.clone()).start();
        
        let hoop_mutator_actor = HoopMutatorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let hoop_accessor_actor = HoopAccessorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();

        let actor_instances = ActorInstaces{
            consumer_actors: ConsumerActors{
                notif_actor: notif_consumer_actor.clone()
            },
            producer_actors: ProducerActors{
                notif_actor: notif_producer_actor.clone(),
                zerlog_actor: zerlog_producer_actor.clone()
            },
            ws_actors: WsActors{
                hoop_server_actor: hoop_ws_server_instance.clone()
            },
            cqrs_actors: CqrsActors{
                mutators: MutatorActors{
                    notif_mutator_actor: notif_mutator_actor.clone(),
                    hoop_mutator_actor: hoop_mutator_actor.clone()
                },
                accessors: AccessorActors{
                    notif_accessor_actor: notif_accessor_actor.clone(),
                    hoop_accessor_actor: hoop_accessor_actor.clone()
                },
            },
        };
        
        Self { 
            config: configs, 
            app_storage: app_storage.clone(), 
            actors: Some(actor_instances),  
            ramdb: std::sync::Arc::new(
                tokio::sync::Mutex::new(
                    HashMap::new()
                )
            )
        }

    }

}