



use actix::prelude::*;
use crate::{consts::PING_INTERVAL, *};


#[derive(Clone)]
pub struct UserActor{

}

impl Actor for UserActor{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        
        log::info!("🎬 NotifMutatorActor has started, let's mutate baby!");

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

impl UserActor{

    pub async fn consume(&self){

        // send message to notif consumer actor to start consuming 
        // from a queue, consume in a fanout way from a queue that 
        // friends of this user have done some action
    }
}