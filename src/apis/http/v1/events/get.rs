


use crate::models::event::NotifQuery;

pub use super::*;


#[get("/hoop/get/")]
pub(self) async fn get_hoop(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    // first try to fetch data from the redis cache if the key is not expired yet
    // otherwise try to get hoop by sending message to accessor which directly fetch from db
    // eventually cache the response into redis with an expirable key
    // ...
    
    // the order of async tasks exeuction inside tokio spawn is asyncly
    // and completely are indepndent from each other.
    tokio::spawn(async move{
        // spawn async tasks to avoid blocking
        // ...
    });
    
    todo!()

}

#[get("/notif/get/owner/")]
pub(self) async fn get_notif(
    req: HttpRequest,
    notif_query: web::Query<NotifQuery>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    // first try to fetch data from the redis cache if the key is not expired yet
    // otherwise try to get notif by sending message to notif accessor which directly fetch from db
    // eventually cache the response into redis with an expirable key
    // tricky: try to talk to the accessor actor directly and get a response from it intead of reading from redis
    // ...


    let actors = app_state.clone().actors.clone().unwrap();
    let notif_accessor_actor = actors.cqrs_actors.accessors.notif_accessor_actor;
    let zerlog_producer_actor = actors.producer_actors.zerlog_actor;
    let notif_owner = notif_query.to_owned().0;

    match notif_accessor_actor.send(
        RequestNotifData{
            owner: notif_owner.owner
        }
    ).await
    {
        Ok(get_notifs) => {
            
            let notifs = get_notifs.0.await;
            
            todo!()

        },
        Err(e) => {
            let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
            let err_instance = crate::error::HoopoeErrorResponse::new(
                *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                &String::from("register_notif.producer_actors.notif_actor.send"), // current method name
                Some(&zerlog_producer_actor)
            ).await;
            return Ok(err_instance.error_response());
        }
    }
    
}

#[get("/notif/get/mint-status")]
pub(self) async fn get_mint_status_notif(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    // first try to fetch data from the redis cache if the key is not expired yet
    // otherwise try to get notif by sending message to notif accessor which directly fetch from db
    // eventually cache the response into redis with an expirable key
    // ...

    // the order of async tasks exeuction inside tokio spawn is asyncly
    // and completely are indepndent from each other.
    tokio::spawn(async move{
        // spawn async tasks to avoid blocking
        // ...
    });
    
    todo!()

}

pub mod exports{
    pub use super::get_hoop;
    pub use super::get_notif;
}