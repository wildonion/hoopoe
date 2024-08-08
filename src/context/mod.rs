


/* 
                                app context
    in static based lang we sould initialize all global structures once 
    to avoid costs of runtime overhead by reinitializing every time we
    need them again. there would be great to have a thread safe structure 
    to store all that global structures in there and move that structure 
    between apis and threads, it contains all initialized structures that
    is going to be used in every api by extracting them from the depot
    structure.
*/

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


pub struct Channels{
    pub notif_broker: NotifMpscChannel
}

pub struct NotifMpscChannel{
    pub sender: tokio::sync::mpsc::Sender<String>,
    pub receiver: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<String>>>
}

#[derive(Clone)]
pub struct AppContext{
    pub config: Option<std::sync::Arc<Context<ConfigEnv>>>,
    pub app_storage: Option<std::sync::Arc<Storage>>,
    pub actors: Option<ActorInstaces>,
    pub channels: std::sync::Arc<Channels>,
    pub kvan: KVan

}

impl AppContext{

    // initializing everything once!
    pub async fn init() -> Self{

        let app_storage = Storage::new().await;
        let (notif_broker_tx, notif_broker_rx) = tokio::sync::mpsc::channel::<String>(1024);
        let channels = 
            std::sync::Arc::new(
                Channels{
                    notif_broker: NotifMpscChannel{
                        sender: notif_broker_tx.clone(),
                        // making the receiver safe to be shared multiple times and mutated
                        receiver: std::sync::Arc::new(
                            tokio::sync::Mutex::new(
                                notif_broker_rx
                            )
                        )
                    }
                }
            );

        // build the necessary actors, start once
        let zerlog_producer_actor = ZerLogProducerActor::new(app_storage.clone()).start();
        let notif_mutator_actor = NotifMutatorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let notif_accessor_actor = NotifAccessorActor::new(app_storage.clone(), zerlog_producer_actor.clone()).start();
        let notif_actor = NotifBrokerActor::new(app_storage.clone(), notif_mutator_actor.clone(), zerlog_producer_actor.clone(), notif_broker_tx.clone()).start();
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
            channels,
            app_storage: app_storage.clone(), 
            actors: Some(actor_instances),  
            kvan: std::sync::Arc::new( // an in memory, mutable and safe binary tree map which can be shared across threads
                tokio::sync::Mutex::new(
                    BTreeMap::new()
                )
            )
        }

    }

}