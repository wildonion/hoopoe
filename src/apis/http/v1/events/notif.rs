


use crate::{models::event::{DbNotifData, EventQuery, NotifData}, workers::cqrs::accessors::notif::{NotifDataResponse, RequestNotifDataByNotifId}};
use redis::AsyncCommands;
use workers::cqrs::accessors::notif::RequestAllNotifData;
pub use super::*;



/* -ˋˏ✄┈┈┈┈
 ------------------------
|  Notif CRUD controller
|------------------------
| register notif        => POST   /notif/
| get all notifs        => GET    /notif/
| get all owner notifs  => GET    /notif/?owner=
| get notif by Id       => GET    /notif/?id=
|
*/




/* -ˋˏ✄┈┈┈┈
    >_  this route is used mainly to either produce a data to RMQ exchange 
        or start consuming in the background, look at the postman
        collection for more details.
*/
#[post("/notif/")]
pub(self) async fn register_notif(
    req: HttpRequest,
    register_notif: web::Json<models::event::RegisterNotif>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    let zerlog_producer_actor = app_state.as_ref().actors.clone().unwrap().producer_actors.zerlog_actor;
    let app_state = app_state.clone();
    let register_notif_req = register_notif.to_owned();
    let get_producer_info = register_notif_req.clone().producer_info;
    let get_consumer_info = register_notif_req.clone().consumer_info;

    match req.verify_token_time(app_state.clone(), "write").await{
        Ok(token_time) => {

            if get_producer_info.is_some(){
                let producer_info = get_producer_info.unwrap();
                let mut notif = producer_info.info;
                notif.exchange_name = format!("{}.notif:{}", APP_NAME, notif.exchange_name);

                /* -ˋˏ✄┈┈┈┈
                    >_ sending notif to RMQ in the background
                        you may want to produce data in a loop{} constantly 
                        like sending gps data contains lat and long into the
                        exchange so other consumers be able to consume constantly
                        as the data coming at a same time, kindly put the sending
                        message logic to actor inside a loop{}.
                */
                tokio::spawn( // running the producing notif job in the background in a free thread
                    {
                        let cloned_app_state = app_state.clone();
                        let cloned_notif = notif.clone();
                        async move{
                            // producing nofit by sending the ProduceNotif message to
                            // the producer actor,
                            match cloned_app_state.clone().actors.as_ref().unwrap()
                                    .producer_actors.notif_actor.send(cloned_notif).await
                                {
                                    Ok(_) => { () },
                                    Err(e) => {
                                        let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                                        let err_instance = crate::error::HoopoeErrorResponse::new(
                                            *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                                            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                            crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                                            &String::from("register_notif.producer_actors.notif_actor.send"), // current method name
                                            Some(&zerlog_producer_actor)
                                        ).await;
                                        return;
                                    }
                                }
                            }
                    }
                );
        
                resp!{
                    &[u8],
                    &[],
                    {
                        Some(serde_json::to_value(&notif).unwrap())
                    }, // metadata
                    &format!("SUCCESS: notification registered succesfully, producer has produced the notification"),
                    StatusCode::OK,
                    None::<Cookie<'_>>,
                }
        
            } else if get_consumer_info.is_some(){
                let consumer_info = get_consumer_info.unwrap();
                let mut notif = consumer_info.info;
                notif.queue = format!("{}.{}:{}", APP_NAME, notif.tag, notif.queue);

                /* -ˋˏ✄┈┈┈┈
                    >_ start consuming in the background
                        if you know the RMQ info you may want to start consuming in the 
                        background exactly when the server is being started otherwise 
                        you would have to send message to its actor to start it as an 
                        async task in the background.
                */
                tokio::spawn( // running the consuming notif job in the background in a free thread
                    {
                        let cloned_app_state = app_state.clone();
                        let cloned_notif = notif.clone();
                        async move{
                            // consuming notif by sending the ConsumeNotif message to 
                            // the consumer actor,
                            match cloned_app_state.clone().actors.as_ref().unwrap()
                                    .consumer_actors.notif_actor.send(cloned_notif).await
                                {
                                    Ok(_) => { 
                                        
                                        // in here you could access the notif for an owner using 
                                        // a redis key like: notif_owner:3 which retrieve all data
                                        // on the redis for the receiver with id 3   
                                        () 

                                    },
                                    Err(e) => {
                                        let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                                        let err_instance = crate::error::HoopoeErrorResponse::new(
                                            *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                                            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                            crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                                            &String::from("register_notif.consumer_actors.notif_actor.send"), // current method name
                                            Some(&zerlog_producer_actor)
                                        ).await;
                                        return;
                                    }
                                }
            
                        }
                    }
                );
        
                resp!{
                    &[u8],
                    &[],
                    {
                        Some(serde_json::to_value(&notif).unwrap())
                    }, // metadata
                    &format!("SUCCESS: notification registered succesfully, consumer has started consuming"),
                    StatusCode::OK,
                    None::<Cookie<'_>>,
                }
        
            } else{
        
                resp!{
                    &[u8],
                    &[],
                    None, // metadata
                    &format!("ERROR: at least one of the consumer or producer info must be available to register a notif"),
                    StatusCode::NOT_ACCEPTABLE,
                    None::<Cookie<'_>>,
                }
            }

        },
        Err(resp_err) => resp_err
    }

}

/* -ˋˏ✄┈┈┈┈
    >_  this route is used mainly to retrieve notifications in a short polling 
        manner, client must call this api in an interval to fetch notifs for an
        owner or whole data like every 5 seconds to simulate realtiming in client.
*/
#[get("/notif/")]
pub(self) async fn get_notif(
    req: HttpRequest,
    notif_query: web::Query<EventQuery>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    let redis_pool = app_state.clone().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    let actors = app_state.clone().actors.clone().unwrap();
    let notif_accessor_actor = actors.cqrs_actors.accessors.notif_accessor_actor;
    let zerlog_producer_actor = actors.producer_actors.zerlog_actor;
    
    let notif_query = notif_query.to_owned().0;

    // get a single notif by id
    if notif_query.id.is_some(){
        
        let get_notif_data = notif_accessor_actor
            .send(RequestNotifDataByNotifId{ notif_id: notif_query.id.unwrap_or_default() }).await;

        match get_notif_data{
            Ok(notif) => {

                let notif_data = notif.0.await;
                resp!{
                    Option<Option<DbNotifData>>,
                    notif_data,
                    {
                        Some(serde_json::json!({
                            "server_time": chrono::Local::now().to_string() 
                        }))
                    },
                    &format!("SUCCESS: fetched notification"),
                    StatusCode::OK,
                    None::<Cookie<'_>>,
                }

            },
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                    &String::from("get_notif.notif_accessor_actor.RequestNotifDataByNotifId.send"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
                return Err(err_instance);
            }
        }

    } else if notif_query.owner.is_some(){ // get notifs for owner
        match redis_pool.get().await{
            Ok(mut redis_conn) => {
    
                // by default every incoming notif from the producer will be cached in redis 
                // while the consumer consumes them so we first try to get owner notif from redis
                // otherwise we send message to accessor actor to fetch them from db.
                let redis_notif_key = format!("notif_owner_api_resp:{}", &notif_query.owner.as_ref().unwrap_or(&String::from("")));
                let is_key_there: bool = redis_conn.exists(&redis_notif_key).await.unwrap();
                if is_key_there{
                    let notif_data: String = redis_conn.get(&redis_notif_key).await.unwrap();
                    let decoded_data = serde_json::from_str::<NotifDataResponse>(&notif_data).unwrap();
                    resp!{
                        NotifDataResponse,
                        decoded_data,
                        {
                            Some(serde_json::json!({
                                "server_time": chrono::Local::now().to_string() 
                            }))
                        },
                        &format!("SUCCESS: fetched all owner notifications"),
                        StatusCode::OK,
                        None::<Cookie<'_>>,
                    }
                } else{
    
                    // there is no data on redis so send message to the accessor actor
                    // to fetch the latest data from the db
                    match notif_accessor_actor.send(
                        RequestNotifData{
                            owner: notif_query.owner,
                            from: notif_query.from,
                            to: notif_query.to,
                            page_size: notif_query.page_size
                        }
                    ).await
                    {
                        Ok(get_notifs) => {
                            
                            let notifs = get_notifs.0.await; 
    
                            resp!{
                                Option<Option<NotifDataResponse>>,
                                notifs,
                                {
                                    Some(serde_json::json!({
                                        "server_time": chrono::Local::now().to_string() 
                                    }))
                                },
                                &format!("SUCCESS: fetched all owner notifications"),
                                StatusCode::OK,
                                None::<Cookie<'_>>,
                            }
                        },
                        Err(e) => {
                            let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                            let err_instance = crate::error::HoopoeErrorResponse::new(
                                *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                                &String::from("get_notif.notif_accessor_actor.RequestNotifData.send"), // current method name
                                Some(&zerlog_producer_actor)
                            ).await;
                            return Err(err_instance);
                        }
                    }
                    
                }
    
            }, 
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                    &String::from("get_notif.redis_pool"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
                return Err(err_instance);
            }
        }

    } else{ // get all notifs

        match redis_pool.get().await{
            Ok(mut redis_conn) => {
    
                // by default every incoming notif from the producer will be cached in redis 
                // while the consumer consumes them so we first try to get owner notif from redis
                // otherwise we send message to accessor actor to fetch them from db.
                let redis_notif_key = format!("all_notifs_api_resp");
                let is_key_there: bool = redis_conn.exists(&redis_notif_key).await.unwrap();
                if is_key_there{
                    let notif_data: String = redis_conn.get(&redis_notif_key).await.unwrap();
                    let decoded_data = serde_json::from_str::<NotifDataResponse>(&notif_data).unwrap();
                    resp!{
                        NotifDataResponse,
                        decoded_data,
                        {
                            Some(serde_json::json!({
                                "server_time": chrono::Local::now().to_string() 
                            }))
                        },
                        &format!("SUCCESS: fetched all owner notifications"),
                        StatusCode::OK,
                        None::<Cookie<'_>>,
                    }
                } else{
    
                    // there is no data on redis so send message to the accessor actor
                    // to fetch the latest data from the db
                    match notif_accessor_actor.send(
                        RequestAllNotifData{
                            from: notif_query.from,
                            to: notif_query.to,
                            page_size: notif_query.page_size
                        }
                    ).await
                    {
                        Ok(get_notifs) => {
                            
                            let notifs = get_notifs.0.await; 
    
                            resp!{
                                Option<Option<NotifDataResponse>>,
                                notifs,
                                {
                                    Some(serde_json::json!({
                                        "server_time": chrono::Local::now().to_string() 
                                    }))
                                },
                                &format!("SUCCESS: fetched all owner notifications"),
                                StatusCode::OK,
                                None::<Cookie<'_>>,
                            }
                        },
                        Err(e) => {
                            let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                            let err_instance = crate::error::HoopoeErrorResponse::new(
                                *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                                &String::from("get_notif.notif_accessor_actor.RequestAllNotifData.send"), // current method name
                                Some(&zerlog_producer_actor)
                            ).await;
                            return Err(err_instance);
                        }
                    }
                    
                }
    
            }, 
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                    &String::from("get_notif.redis_pool"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
                return Err(err_instance);
            }
        }

    }
    
    
}



pub mod exports{
    pub use super::get_notif;
    pub use super::register_notif;
}