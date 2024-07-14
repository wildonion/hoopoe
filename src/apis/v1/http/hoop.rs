

use models::event::EventQuery;
use salvo::Error;
use serde::{Deserialize, Serialize};
use crate::*;


/* -ˋˏ✄┈┈┈┈
 ------------------------
|  Hoop CRUD controller
|------------------------
| store new hoop                => POST   /hoop/
| get all hoops                 => GET    /hoop/
| get a single hoop             => GET    /hoop/?id=1
| get all hoops for an owner    => GET    /hoop/?owner=
| delete a single hoop          => DELETE /hoop/?id=1 
| delete all hoops for an owner => DELETE /hoop/?owner=
| update a single hoop          => PUT    /hoop/
|
*/


#[endpoint]
pub async fn add_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, 
    ctrl: &mut FlowCtrl
){

    res.render("developing...")    

}


#[endpoint]
pub async fn delete_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, 
    ctrl: &mut FlowCtrl
){
    res.render("developing...")    
}


#[endpoint]
pub async fn update_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, 
    ctrl: &mut FlowCtrl
){
    res.render("developing...")    
}

#[endpoint]
pub async fn get_hoop(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, 
    ctrl: &mut FlowCtrl,
    query_params: QueryParam<EventQuery, true>
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