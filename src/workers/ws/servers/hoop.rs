



use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use actix::{Actor, Addr, AsyncContext, Context};
use crate::workers::ws::sessions::hoop::HoopSession;
use crate::s3::Storage;
use crate::consts::PING_INTERVAL;


pub struct HoopServer{
    pub app_storage: std::option::Option<Arc<Storage>>, 
    pub rooms: HashMap<String, HashSet<Addr<HoopSession>>>,
    pub sessions: HashSet<Addr<HoopSession>>
}

impl Actor for HoopServer{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        ctx.run_interval(PING_INTERVAL, |actor, ctx|{
            
            // ctx scheduler, check something in the background constantly 
            // if it has been happened then publish event data either self.remote_emit() or self.local_emit()
            // ...

        });
        
    }
}

impl HoopServer{

    pub fn new(app_storage: std::option::Option<Arc<Storage>>) -> Self{
        Self { app_storage, rooms: Default::default(), sessions: Default::default() }
    }

    // sending to all sessions
    pub async fn broadcast_msg(&self){

    }

    // sending to all clients in the passed room
    pub async fn send_msg_to_room(&self, room: &str){

    }

    // sending to an specific actor 
    pub async fn send_msg_to_actor(&self, addr: Addr<HoopSession>){

    }

}