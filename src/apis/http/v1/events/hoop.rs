


use crate::{models::event::{EventQuery, NewHoopRequest, NotifData, UpdateHoopRequest}, 
    workers::cqrs::accessors::notif::NotifDataResponse};
use actix_web::{delete, put};
use redis::AsyncCommands;
use sea_orm::ConnectionTrait;
pub use super::*;



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


#[delete("/hoop/")]
pub(self) async fn delete_hoop(
    req: HttpRequest,
    hoop_id: web::Query<EventQuery>,
    app_state: web::Data<AppState>
) -> HoopoeHttpResponse{

    let app_storage = app_state.app_storage.clone().unwrap();
    let db = app_storage.get_seaorm_pool().await.unwrap();
    let zerlog_producer_actor = app_state.actors.clone().unwrap().producer_actors.zerlog_actor;

    // execute raw query or call accessor actor to do so
    db.execute_unprepared("").await.unwrap();

    todo!()
}

#[delete("/hoop/")]
pub(self) async fn delete_hoop_by_owner(
    req: HttpRequest,
    hoop_owner: web::Query<EventQuery>,
    app_state: web::Data<AppState>
) -> HoopoeHttpResponse{

    let app_storage = app_state.app_storage.clone().unwrap();
    let db = app_storage.get_seaorm_pool().await.unwrap();
    let zerlog_producer_actor = app_state.actors.clone().unwrap().producer_actors.zerlog_actor;

    // execute raw query or call accessor actor to do so
    db.execute_unprepared("").await.unwrap();

    todo!()
}

#[put("/hoop/")]
pub(self) async fn update_hoop(
    req: HttpRequest,
    update_hoop_request: web::Json<UpdateHoopRequest>,
    app_state: web::Data<AppState>
) -> HoopoeHttpResponse{

    let app_storage = app_state.app_storage.clone().unwrap();
    let db = app_storage.get_seaorm_pool().await.unwrap();
    let zerlog_producer_actor = app_state.actors.clone().unwrap().producer_actors.zerlog_actor;

    // execute raw query or call accessor actor to do so
    db.execute_unprepared("").await.unwrap();

    todo!()
}


#[post("/hoop/")]
pub(self) async fn add_hoop(
    req: HttpRequest,
    new_hoop_request: web::Json<NewHoopRequest>,
    app_state: web::Data<AppState>
) -> HoopoeHttpResponse{

    let app_storage = app_state.app_storage.clone().unwrap();
    let db = app_storage.get_seaorm_pool().await.unwrap();
    let zerlog_producer_actor = app_state.actors.clone().unwrap().producer_actors.zerlog_actor;

    // execute raw query or call accessor actor to do so
    db.execute_unprepared("").await.unwrap();

    todo!()
}

// /hoop/
// /hoop/?hoop_id=
// /hoop/?owner=
#[get("/hoop/")]
pub(self) async fn get_hoop(
    req: HttpRequest,
    hoop_query: web::Query<EventQuery>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    let redis_pool = app_state.clone().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    let actors = app_state.clone().actors.clone().unwrap();
    let hoop_accessor_actor = actors.cqrs_actors.accessors.hoop_accessor_actor;
    let zerlog_producer_actor = actors.producer_actors.zerlog_actor;

    // if both of them is none (hoop_id and hoop owner) then all hoops are retunred
    // ...
    
    match redis_pool.get().await{
        Ok(redis_conn) => {


            // try to get from redis cache
            // otherwise send message to its accessor actor 
            // ...

            todo!()

        },
        Err(e) => {
            let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
            let err_instance = crate::error::HoopoeErrorResponse::new(
                *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                &String::from("get_hoop.redis_pool"), // current method name
                Some(&zerlog_producer_actor)
            ).await;
            return Err(err_instance);
        }
    }

}

pub mod exports{
    pub use super::get_hoop;
    pub use super::add_hoop;
    pub use super::update_hoop;
    pub use super::delete_hoop;
    pub use super::delete_hoop_by_owner;
}