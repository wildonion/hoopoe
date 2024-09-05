


use std::error::Error;
use actix::Addr;
use actix::Actor;
use actix::prelude::*;
use constants::MAILBOX_CHANNEL_ERROR_CODE;
use constants::STORAGE_IO_ERROR_CODE;
use context::ActorInstaces;
use context::MutatorActors;
use deadpool_redis::redis::AsyncCommands;
use futures::FutureExt;
use models::event::HoopEventForm;
use models::user::UserData;
use redis_async::resp::FromResp;
use storage::engine::Storage;
use workers::cqrs::mutators::hoop::UpdateIsFinished;
use workers::zerlog::ZerLogProducerActor;
use crate::*;
use actix::prelude::Handler as ActixMessageHandler;


#[derive(Message, Clone, Serialize, Deserialize, Debug, Default)]
#[rtype(result = "()")]
pub struct UpdateState{
    pub new_val: u8
}

#[derive(Clone)]
pub struct CronScheduler{
    pub state: u8,
    pub notif_broker_actor: Addr<NotifBrokerActor>, // send the notif message to the notif broker actor
    pub app_storage: Option<Arc<Storage>>,
    pub mutator_actors: MutatorActors,
    pub ws_sender: tokio::sync::mpsc::Sender<String>, // push the triggered notif to ws channel
    pub zerlog_producer: Addr<ZerLogProducerActor> // send error message to the zerlog actor
}

impl CronScheduler{

    pub fn new(
        notif_broker_actor: Addr<NotifBrokerActor>, 
        app_storage: Option<Arc<Storage>>, 
        mutator_actors: MutatorActors,
        ws_sender: tokio::sync::mpsc::Sender<String>,
        zerlog_producer: Addr<ZerLogProducerActor>) -> Self{

            Self{
                state: 0,
                notif_broker_actor,
                app_storage,
                mutator_actors,
                ws_sender,
                zerlog_producer,
            }
    }

    /* -------------------------------------------------------------
        since futures are object safe trait hence they have all traits 
        features we can pass them to the functions in an static or dynamic 
        dispatch way using Arc or Box or impl Future or event as the return 
        type of a closure trait method:
            std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync + 'static>
            Arc<dyn Fn() -> R + Send + Sync + 'static> where R: std::future::Future<Output = ()> + Send + Sync + 'static
            Box<dyn Fn() -> R + Send + Sync + 'static> where R: std::future::Future<Output = ()> + Send + Sync + 'static
            Arc<Mutex<dyn Fn() -> R + Send + Sync + 'static>> where R: std::future::Future<Output = ()> + Send + Sync + 'static
            F: std::future::Future<Output = ()> + Send + Sync + 'static
            param: impl std::future::Future<Output = ()> + Send + Sync + 'static

        NOTE: mutex requires the type to be Sized and since traits are 
        not sized at compile time we should annotate them with dyn keyword
        and put them behind a pointer with valid lifetime or Box and Arc smart pointers
        so for the mutexed_job we must wrap the whole mutex inside an Arc or annotate it
        with something like &'valid tokio::sync::Mutex<dyn Fn() -> R + Send + Sync + 'static>
        the reason is that Mutex is a guard and not an smart pointer which can hanlde 
        an automatic pointer with lifetime 
    */
    pub async fn startCronScheduler<F, R>(seconds: u64, 
        // make the future cloneable in each iteration and tokio scope 
        // as well as safe to be shared between threads
        task: std::sync::Arc<dyn Fn() -> R + Send + Sync + 'static>) where // make the closure trait shareable and cloneable
            R: std::future::Future<Output = ()> + Send + Sync + 'static{
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(seconds));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // execute the checking io task in the background worker thread  
        // in a none blocking manner, runtime suspend or pause the execution
        // in here so other tasks can get executed meanwhile it's waiting for
        // the result of this task.
        tokio::spawn(async move{ // used to start the background loop
            let cloned_task = task.clone();
            loop{
                interval.tick().await;
                tokio::spawn(cloned_task()); // the closure however returns a future 
            }
        });
    }

    pub async fn subscribeToRedisExpChannel(&self){

        let app_storage = self.app_storage.clone().unwrap();
        let redis_pubsubs_conn = app_storage.clone().get_async_redis_pubsub_conn().await.unwrap();
        let zerlog_producer_actor = self.zerlog_producer.clone();
        let redis_conn = app_storage.clone().get_redis_pool().await.unwrap().get().await;
        let mutator_actors = self.mutator_actors.clone();
        let hoop_mutator_actor = mutator_actors.hoop_mutator_actor;

        /* 
            1) instead of doing an io constantly over the db which is 
            more resource intensive we'll be using the below approach:
            once the key gets expired, we receive the key info 
            by subscribing to __keyevent@0__:expired channel in here, 
            we'll then update the hoop field in db.
            
            2) prepare the notif object of the related hoop to publish 
            it to rmq broker (fetch notif using http api in a short 
            polling manner) and ws channel (fetch notif using ws in 
            realtime manner)

            3) set a new redis exp key for the next 1 mins
            when it expires again subscription process takes place
            and step 1 will be executed again.
        */
        match redis_conn{
            Ok(mut conn) => {

                // use redis async for handling realtime streaming of events
                let get_streamer = redis_pubsubs_conn
                    .subscribe("__keyevent@0__:expired")
                    .await;

                let Ok(mut pubsubstream) = get_streamer else{
                    let e = get_streamer.unwrap_err();
                    let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::Storage(crate::error::StorageError::RedisAsync(e)), // the actual source of the error caused at runtime
                        &String::from("subscribeToRedisExpChannel.get_streamer"), // current method name
                        Some(&zerlog_producer_actor)
                    ).await;
                    return; // terminate the caller
                };
                
                // realtime streaming over redis channel to receive emitted notifs 
                while let Some(message) = pubsubstream.next().await{
                    let resp_val = message.unwrap();
                    let expired_key = String::from_resp(resp_val).unwrap(); // this is the expired key
                    log::info!("expired key has subscribed: {}", expired_key);

                    // let's find those keys start with hoop to extract the title for io call
                    let hoop_title = if expired_key.starts_with("hoop"){
                        let splitted: Vec<&str> = expired_key.split("_").collect();
                        splitted[1].to_string()
                    } else{
                        String::from("")
                    };


                    // if we're here means the key has expired which enables us
                    // to emit the expiration logic of the hoop event in here, 
                    // update the IsFinished field of the hoop event to true
                    // ...

                    // send the updation message to the hoop mutator actor
                    // execute this async io task in a background worker by
                    // not blocking the thread
                    let cloned_hoop_mutator_actor = hoop_mutator_actor.clone();
                    let cloned_zerlog_producer_actor = zerlog_producer_actor.clone();
                    tokio::spawn(async move{
                        match cloned_hoop_mutator_actor.send(UpdateIsFinished{
                            hoop_title,
                        }).await 
                        {
                            Ok(_) => { /* ... */ },
                            Err(e) => {
                                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                                let err_instance = crate::error::HoopoeErrorResponse::new(
                                    *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                    crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                                    &String::from("subscribeToRedisExpChannel.hoop_mutator_actor.send"), // current method name
                                    Some(&cloned_zerlog_producer_actor)
                                ).await;
                                return;
                            }
                        }
                    });
                    
                }

            },
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                    &String::from("subscribeToRedisExpChannel.redis_conn"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
            }
        }

    }

}


impl Actor for CronScheduler{
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {

        let this  = self.clone();
        // runt the subscription task in the background thread worker
        // inside an io light thread 
        tokio::spawn(async move{
            this.subscribeToRedisExpChannel().await;
        });

        // simulating the following can be done using tokio time, spawn and loop 
        // then execute io calls inside the loop{}
        ctx.run_interval(constants::PING_INTERVAL, |actor, ctx|{
            tokio::spawn(async move{
                
                // execute some io calling constantly in the background worker thread
                // like checking something against db to see either that an
                // entity deadline has ended or not
                // ...

            });
        });
    }
}

// message handler to update the cron shceduler actor component state
impl ActixMessageHandler<UpdateState> for CronScheduler{
    type Result = ();
    fn handle(&mut self, msg: UpdateState, ctx: &mut Self::Context) -> Self::Result {
        
        let UpdateState { new_val } = msg;
        self.state = new_val;
    }
}