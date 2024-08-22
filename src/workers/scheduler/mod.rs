


/* 
    scheduler actor worker to check something contantly in the background using cron scheduler
    as well as redis subscriber to __keyevent@0__:expired channel
    realtime notification streaming setups:
        → rpc to get some authenticated data from auth server
        → cron scheduler actor to check something constantly in the background io light thread
        → redis to set the time of checking using set_exp and subscribing to exp channel
        → ws mpsc channel for realtime streaming and send trigerred notif object through ws to its owner; receive the notif in ws server setup
        → rmq notifborker actor to publish notif to rmq broker
        → http api endpoint to fetch owner notifs using a short polling manner

    event scheduler (check the endTime of the event constantly to close the event) 
    resource-intensive with regular checking in a loop{}:
        1 - an actor task or cron scheduler to check the end time of the hoop constantly to update the is_finished field
        2 - loop tokio spawn interval tick then include!{} hoop_scheduler.rs to call the method 

    the optimal and none intensive solution would be using of key space notifications 
    which allows allow clients to subscribe to Pub/Sub channels in order to receive 
    events affecting the Redis data set in some wayin Redis, however the followings 
    are the steps must be taken to complete the logic. We're consuming that we have a 
    user_id as the key and some value with an exportable key for 10mins later 
    after login time. 

        let login_time = chrono::Local::now();
        let ten_mins_later = login_time + chrono::Duration::minutes(10);
        redis_conn.set_exp(user_id, ten_mins_later);

        1 - configuring Redis to enable key space notifications 
        2 - when the key expires Redis publish its event to a prebuilt expiration channel 
        2 - we then subscribe to the __keyevent@0__:expired channel
        3 - we'll receive the event from the channel 
        4 - trigger the notification for the related user id (expired key) 
        5 - publish triggered notif to rmq producer using notif_borker_actor
        6 - consume notif from rmq broker to cache on redis and store in db for future short pollings 
        7 - send received notif to mpsc sender of the ws server 
        8 - receive the notif from the mpsc channel inside ws server setup

    at this time to send notif to client we can either 
    cache the notif on Redis or store it on db, allows clients use short polling approach to 
    fetch the notif through an interval process.
    or another approach which is more resource intensive for push notification strategies 
    is by using a channel (MPSC) to send the notif to a websocket server actor configuration 
    thread from there send to the ws peer actor in realtime.
    the ws setup could be an actor based setup which is more simpler to send messages to 
    peer sessions from ws server through actor concepts like: 
    atomic syncing with mutex rwlock and channels, os/async io threads or tasks, 
    select, mailbox mpsc channels and task scheduler interval.
*/

use std::error::Error;

use actix::Addr;
use actix::Actor;
use actix::prelude::*;
use constants::STORAGE_IO_ERROR_CODE;
use deadpool_redis::redis::AsyncCommands;
use models::event::HoopEventForm;
use models::user::UserData;
use storage::engine::Storage;
use workers::zerlog::ZerLogProducerActor;
use crate::*;



#[derive(Clone)]
pub struct Scheduler{
    pub notif_broker_actor: Addr<NotifBrokerActor>,
    pub app_storage: Option<Arc<Storage>>,
    pub ws_sender: tokio::sync::mpsc::Sender<String>,
    pub zerlog_producer: Addr<ZerLogProducerActor>
}

impl Scheduler{

    pub fn new(
        notif_broker_actor: Addr<NotifBrokerActor>, 
        app_storage: Option<Arc<Storage>>, 
        ws_sender: tokio::sync::mpsc::Sender<String>,
        zerlog_producer: Addr<ZerLogProducerActor>) -> Self{

            Self{
                notif_broker_actor,
                app_storage,
                ws_sender,
                zerlog_producer,
            }
    }

    pub async fn subscribe_to_redis_exp_channel(&self, hoop_info: HoopEventForm, user_data: UserData){

        let app_storage = self.app_storage.clone().unwrap();
        let redis_pool = app_storage.clone().get_redis_pool().await.unwrap();
        let mut redis_conn = redis_pool.get().await;
        let zerlog_producer_actor = self.zerlog_producer.clone();

        match redis_conn{
            Ok(mut conn) => {

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

                    3) set a new redis exp key for the next 10 mins
                */

                let now = chrono::Local::now();
                let ten_mins_later = chrono::Local::now() + chrono::Duration::minutes(10);
                let duration_in_seconds = ten_mins_later.timestamp() - now.timestamp();
                let key = format!("hoop_{}_at_{}", hoop_info.title, hoop_info.started_at);
                let _: () = conn.set_ex(key, &user_data.username, duration_in_seconds as u64).await.unwrap();

            },
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                    &String::from("subscribe_to_redis_exp_channel.redis_conn"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
            }
        }

    }

}


impl Actor for Scheduler{
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {

        ctx.run_interval(constants::PING_INTERVAL, |actor, ctx|{

            tokio::spawn(async move{
                
                // execute some io calling constantly in the background thread
                // like checking something against db to see either that an
                // entity has ended or not
                // ...

            });

        });
    }
}