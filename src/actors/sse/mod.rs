

use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::consts::PING_INTERVAL;
use crate::types::*;
use crate::s3;


#[derive(Message)]
#[rtype(result = "()")]
pub struct AddClient{
     
}


#[derive(Message)]
#[rtype(result = "()")]
pub struct BroadcastEvent{
    pub topic: String,
    pub event: String
}

// create sse using tokio broadcast channel 
// use sse for sending realtime and live locations to browsers
// https://github.com/chaudharypraveen98/actix-question-bank-stackoverflow/blob/master/src/broadcast.rs


#[derive(Clone, Default)]
pub struct Broadcaster{
    pub app_storage: Option<std::sync::Arc<s3::Storage>>,
}


impl Actor for Broadcaster{
    
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        
        // pings clients every 10 seconds to see if they are alive and 
        // remove them from the broadcast list if not.
        ctx.run_interval(PING_INTERVAL, |actor, ctx|{
            
            let this = actor.clone();

            tokio::spawn(async move{

                // check something constantly, schedule to be executed 
                // at a certain time like if there is a new event, broadcast
                // it to all clients
                // ...
                
            });

        });

    }
}

impl Broadcaster{

    pub fn new(app_storage: std::option::Option<std::sync::Arc<s3::Storage>>) -> Self{
        Self { app_storage }
    }

    pub async fn get_clients(&self){

    }

    pub async fn spawn_ping(){
        
    }

    pub async fn add_client(&mut self){
        todo!()

    }

    // broadcast new event, so client can use html5 sse to 
    // fetch latest event through the openned connection
    pub async fn broadcast(&mut self, topic: &str, event: &str){

        todo!()
    } 


    // event checker subscriber
    pub async fn subscribe_sec(){

        // it sends message to subscriber/sec.rs actor to get data of 
        // all finished events, it then notifies all its peers 
        // with the latest info of all events separately
        // send events info to those ones who are joined the event already
        // ... 
    }

    // system notifs subscriber (see ActionType)
    pub async fn subscribe_notif(){

        // get all the notifs from redis which have been cached
        // inside the subscriber/notif.rs, it then notifies all its peers 
        // with the latest info of all system notifs separately
        // send noti info to all related server peers
        // ... 
        
    }
    
}

// other parts should send message of type AddClient or BroadcastEvent to this actor 
// to update the state of the Broadcaster struct during the app execution
impl Handler<AddClient> for Broadcaster{

    type Result = ();

    fn handle(&mut self, msg: AddClient, ctx: &mut Self::Context) -> Self::Result {
        
        let mut this = self.clone();
        tokio::spawn(async move{
            this.add_client().await;
        });

    }
}

impl Handler<BroadcastEvent> for Broadcaster{

    type Result = ();

    fn handle(&mut self, msg: BroadcastEvent, ctx: &mut Self::Context) -> Self::Result {
        
        let BroadcastEvent{topic, event} = msg;

        let mut this = self.clone();
        tokio::spawn(async move{
            this.broadcast(&topic, &event).await;
        });

    }
}