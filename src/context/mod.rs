


// app context

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use crate::apis::v1::http::notif;
use crate::workers::cqrs::accessors::hoop::HoopAccessorActor;
use crate::workers::cqrs::accessors::notif::NotifAccessorActor;
use crate::workers::cqrs::mutators::hoop::HoopMutatorActor;
use crate::workers::cqrs::mutators::notif::NotifMutatorActor;
use crate::workers::notif::{NotifBrokerActor};
use crate::config::{Env as ConfigEnv, Context};
use crate::config::EnvExt;
use crate::storage::engine::Storage;
use actix::{Actor, Addr};
use indexmap::IndexMap;
use ractor::concurrency::JoinHandle;
use serde::{Serialize, Deserialize};
use crate::types::*;
use crate::constants::*;
use crate::storage;
use crate::workers::zerlog::ZerLogProducerActor;


#[derive(Clone)]
pub struct BrokerActor{
    pub notif_actor: Addr<NotifBrokerActor>,
    pub zerlog_actor: Addr<ZerLogProducerActor>
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
    pub broker_actors: BrokerActor,
    pub cqrs_actors: CqrsActors,
}


#[derive(Clone)]
pub struct AppContext{
    pub config: Option<std::sync::Arc<Context<ConfigEnv>>>,
    pub app_storage: Option<std::sync::Arc<Storage>>,
    pub actors: Option<ActorInstaces>,
    pub ramdb: RamDb

}

impl AppContext{

    pub async fn init() -> Self{

        let app_storage = Storage::new().await;

        // build the necessary actors, start once
        let zerlog_producer_actor = ZerLogProducerActor::new(app_storage.clone()).start();
        let notif_mutator_actor = NotifMutatorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let notif_accessor_actor = NotifAccessorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let notif_actor = NotifBrokerActor::new(app_storage.clone(), notif_mutator_actor.clone(), zerlog_producer_actor.clone()).start();
        let hoop_mutator_actor = HoopMutatorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let hoop_accessor_actor = HoopAccessorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        
        let actor_instances = ActorInstaces{
            broker_actors: BrokerActor{
                notif_actor: notif_actor,
                zerlog_actor: zerlog_producer_actor,
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
            config: {
                let env = ConfigEnv::default();
                let ctx_env = env.get_vars();
                let configs = Some(
                    std::sync::Arc::new(ctx_env)
                );
                configs
            }, 
            app_storage: app_storage.clone(), 
            actors: Some(actor_instances),  
            ramdb: std::sync::Arc::new( // an in memory, mutable and safe map which can be shared across threads
                tokio::sync::Mutex::new(
                    HashMap::new()
                )
            )
        }

    }

}