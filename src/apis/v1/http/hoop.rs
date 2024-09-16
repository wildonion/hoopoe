

use context::AppContext;
use deadpool_redis::redis::AsyncCommands;
use middlewares::check_token::check_token;
use models::{event::{EventQuery, EventType, HoopEventForm}, user::UserData};
use salvo::{http::form::FormData, Error};
use serde::{Deserialize, Serialize};
use models::server::Response as HoopoeResponse;
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

    // extracting the required user_data from the depot, this gets filled 
    // inside the middleware setup, if we're here means the middleware 
    // has injected the right data into the depot.
    let user_data = depot.get::<UserData>("user_data").unwrap();
    

    // extracting necessary structures from the app context
    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let redis_pool = app_ctx.clone().unwrap().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    let sea_orm_pool = app_ctx.clone().unwrap().app_storage.clone().unwrap().get_seaorm_pool().await.unwrap();
    let actors = app_ctx.clone().unwrap().actors.unwrap();
    let hoop_mutator_actor_controller = actors.clone().cqrs_actors.mutators.hoop_mutator_actor;
    let mut redis_conn = redis_pool.get().await.unwrap();
    let scheduler_actor = actors.clone().scheduler_actor;


    let cover = req.file("cover").await.unwrap();
    let inv_info = serde_json::from_str::
        <Vec<std::collections::HashMap<String, i64>>>
        (&hoop_info.invitations.clone()).unwrap();

    // setting up the exp time of the event inside the redis as an expirable key
    // later on the scheduler actor can subscribe to redis expire channel 
    let hoop_end_time = hoop_info.end_at.parse::<i64>().unwrap();
    let hoop_start_time = hoop_info.started_at.parse::<i64>().unwrap();
    if hoop_end_time < hoop_start_time{
        // reject the request
        let server_time = format!("{}", chrono::Local::now().to_string());
        res.status_code = Some(StatusCode::NOT_ACCEPTABLE);
        res.render(Json(
            HoopoeResponse::<&[u8]>{ 
                data: &[], 
                message: "invalid end time", 
                is_err: true, 
                status: StatusCode::NOT_ACCEPTABLE.as_u16(),
                meta: Some(
                    serde_json::json!({
                        "server_time": server_time
                    })
                )
            }
        ));
    }

    let end_date_time = chrono::DateTime::from_timestamp(hoop_end_time, 0).unwrap().date_naive();
    let now = chrono::Local::now().date_naive();
    let duration_in_seconds = (end_date_time - now).num_seconds() as u64;
    let key = format!("hoop_{}_at_{}", hoop_info.title, hoop_info.started_at);
    let _: () = redis_conn.set_ex(key, "", duration_in_seconds).await.unwrap();

    let etype = match hoop_info.etype.as_str(){
        "social" => EventType::SocialGathering,
        "proposal" => EventType::Proposal,
        "streaming" => EventType::Streaming,
        _ => EventType::None
    };

    // store cover on vps then on s3 or digispaces
    // store hoop info in db by sending the message to the hoop_mutator_actor_controller
 

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