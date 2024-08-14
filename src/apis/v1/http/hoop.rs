

use context::AppContext;
use deadpool_redis::redis::AsyncCommands;
use middlewares::check_token::check_token;
use models::{event::{EventQuery, EventType, HoopEventForm}, user::UserData};
use salvo::{http::form::FormData, Error};
use serde::{Deserialize, Serialize};
use crate::*;



/* -ˋˏ✄┈┈┈┈
 ------------------------
|  Hoop CRUD controller
|------------------------
| store new hoop                       => POST   /hoop/
| get all live hoops                   => GET    /hoop/ | story hoops
| get a single hoop                    => GET    /hoop/?id=1
| get all hoops for an owner           => GET    /hoop/?owner=
| get all joined hoops for an owner    => GET    /hoop/joined/?owner=
| get all invited hoops for an owner   => GET    /hoop/invited/?owner=
| delete a single hoop                 => DELETE /hoop/?id=1 
| delete all hoops for an owner        => DELETE /hoop/?owner=
| update a single hoop                 => PUT    /hoop/ | is_finished, is_locked, capacity, duration, entrance_fee
| join a hoop                          => POST   /hoop/user/join
| left a hoop                          => DELETE /hoop/user/leave
|

tables: users_hoop (user events interactions), hoops (events), notifs (platform notifs)

*/

#[endpoint]
pub async fn add_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, // extracting app context and global data structures 
    ctrl: &mut FlowCtrl, // control the execution of next handlers
    // request body infos
    hoop_info: FormBody<HoopEventForm>,
){


    // trying to get the user data in here 
    let user_data = depot.get::<UserData>("user_data").unwrap();

    // extracting necessary structures from the app context
    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let redis_pool = app_ctx.clone().unwrap().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    let sea_orm_pool = app_ctx.clone().unwrap().app_storage.clone().unwrap().get_seaorm_pool().await.unwrap();
    let actors = app_ctx.clone().unwrap().actors.unwrap();
    let hoop_mutator_actor_controller = actors.clone().cqrs_actors.mutators.hoop_mutator_actor;

    let cover = req.file("cover").await.unwrap();
    let decoded = serde_json::from_str::
        <Vec<std::collections::HashMap<String, i64>>>
        (&hoop_info.invitations.clone()).unwrap();

    let etype = match hoop_info.etype.as_str(){
        "social" => EventType::SocialGathering,
        "proposal" => EventType::Proposal,
        "streaming" => EventType::Streaming, // use hooper streamer handlers
        _ => EventType::None
    };

    // store cover on vps then on s3 or digispaces
    // store hoop info in db by sending the message to the hoop_mutator_actor_controller
    
    res.render("developing...")    

}


#[endpoint]
pub async fn delete_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, // extracting app context and global data structures 
    ctrl: &mut FlowCtrl, // control the execution of next handlers
){
    res.render("developing...")    
}


#[endpoint]
pub async fn update_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, // extracting app context and global data structures 
    ctrl: &mut FlowCtrl, // control the execution of next handlers
    hoop_info: FormBody<HoopEventForm>
){
    res.render("developing...")    
}

#[endpoint]
pub async fn get_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, // extracting app context and global data structures 
    ctrl: &mut FlowCtrl, // control the execution of next handlers,
    // https://salvo.rs/book/features/openapi.html#extractors (QueryParam, HeaderParam, CookieParam, PathParam, FormBody, JsonBody)
    query_params: QueryParam<EventQuery, true> // query param is required, showcasing in swagger ui
){

    // extracting necessary structures from the app context
    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let redis_pool = app_ctx.clone().unwrap().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    let sea_orm_pool = app_ctx.clone().unwrap().app_storage.clone().unwrap().get_seaorm_pool().await.unwrap();
    let actors = app_ctx.clone().unwrap().actors.unwrap();
    let hoop_mutator_actor_controller = actors.clone().cqrs_actors.mutators.hoop_mutator_actor;
    let redis_conn = redis_pool.get().await.unwrap();


    // trying to get the user data in here 
    let user_data = depot.get::<UserData>("user_data").unwrap();

    /* --------------------------------------------------------------------------------------------------------
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

    // get live hoops (those ones that are not finished or expired)
    // get all owner hoops
    // get all user joined hoops

    let query_params = req.parse_queries::<EventQuery>().unwrap();

    res.render("developing...")
}

#[endpoint]
pub async fn leave_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, // extracting app context and global data structures 
    ctrl: &mut FlowCtrl, // control the execution of next handlers
){

    // user can leave a hoop before it starts
    // check that the user must be already registered

    res.render("developing...")    
}
 
#[endpoint]
pub async fn join_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, // extracting app context and global data structures 
    ctrl: &mut FlowCtrl, // control the execution of next handlers
){


    // user can join a hoop before it starts
    // check the the user must not be already registered
    res.render("developing...")    
}


// register all apis and push them into a new router tree
pub fn register_controller() -> Router{

    Router::with_path("/v1/hoop/")
        .oapi_tag("Hoop")
        .get(get_hoop)
        .post(add_hoop)
        .delete(delete_hoop)
        .put(update_hoop)
        .hoop(check_token) // execute this middleware before executing any handler
        .append( // append vector of routers into the current router tree
            &mut vec![
                Router::with_path("user/")
                    .delete(leave_hoop) // user can leave a hoop before it starts
                    .post(join_hoop) // user can join a hoop
            ]
        )
        
}