



use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use actix::{Actor, AsyncContext, Context};
use crate::s3::Storage;
use crate::consts::PING_INTERVAL;


pub struct HoopSession{
    pub app_storage: std::option::Option<Arc<Storage>>, 
}

impl Actor for HoopSession{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        ctx.run_interval(PING_INTERVAL, |actor, ctx|{
            
            // ctx scheduler, check something in the background constantly 
            // if it has been happened then publish event data either self.remote_emit() or self.local_emit()
            // ...

        });
        
    }
}

impl HoopSession{

    pub fn new(app_storage: std::option::Option<Arc<Storage>>) -> Self{
        Self { app_storage }
    }

}