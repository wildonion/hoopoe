

use models::event::{EventQuery, EventType, HoopEventForm};
use salvo::Error;
use serde::{Deserialize, Serialize};
use crate::*;



// todo 
// check token middleware from gem server
// get event cover in add_hoop api
// store images in aws s3 or digispaces
// user_hoops table => joined_at, left_at, user_id, hoop_id, is_invited




/* -ˋˏ✄┈┈┈┈
 ------------------------
|  Hoop CRUD controller
|------------------------
| store new hoop                       => POST   /hoop/
| get all live hoops                   => GET    /hoop/ | story like hoops
| get a single hoop                    => GET    /hoop/?id=1
| get all hoops for an owner           => GET    /hoop/?owner=
| get all joined hoops for an owner    => GET    /hoop/joined/?owner=
| get all invited hoops for an owner   => GET    /hoop/invited/?owner=
| delete a single hoop                 => DELETE /hoop/?id=1 
| delete all hoops for an owner        => DELETE /hoop/?owner=
| update a single hoop                 => PUT    /hoop/
|


NOTE: authenticating and KYCing process will be done using gem server

*/


#[endpoint]
pub async fn add_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, // extracting app context and global data structures 
    ctrl: &mut FlowCtrl, // control the execution of next handlers
    hoop_info: FormBody<HoopEventForm>
){
    
    let cover = req.file("cover").await;
    let etype = match hoop_info.etype.as_str(){
        "social" => EventType::SocialGathering,
        "proposal" => EventType::Proposal,
        "streaming" => EventType::Streaming,
        _ => EventType::None
    };
    
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

    let query_params = req.parse_queries::<EventQuery>().unwrap();
    
    res.render("developing...")
}


pub fn register_controller() -> Router{

    Router::with_path("/v1/hoop/")
        .oapi_tag("Hoop")
        .get(get_hoop)
        .post(add_hoop)
        .delete(delete_hoop)
        .put(update_hoop)
        
}