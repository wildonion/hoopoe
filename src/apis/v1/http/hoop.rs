

use context::AppContext;
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


NOTE: authorizing and KYCing process will be done using gem server

users_hoop, hoops, notifs

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


    // event actor scheduler (check the endTime of the event constantly to close the event): 
    //      an actor cron scheduler to check the end time of the hoop constantly to update the is_finished field
    //      loop tokio spawn interval tick
    //      include!{} hoop_scheduler.rs
    //      redis exp key 
    // get live hoops
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