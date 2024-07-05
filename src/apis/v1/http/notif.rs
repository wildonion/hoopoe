



use std::error::Error;
use constants::{MAILBOX_CHANNEL_ERROR_CODE, STORAGE_IO_ERROR_CODE};
use context::AppContext;
use models::event::{EventQuery, RegisterNotif};
use workers::cqrs::accessors::notif::{NotifDataResponse, RequestAllNotifData, RequestNotifData, RequestNotifDataByNotifId};
use crate::*;
use crate::models::server::Response as HoopoeResponse;
use crate::middlewares::passport::check_passport;


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
#[endpoint]
pub async fn register_notif(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
    /* -----------
        ctrl controls the execution flow of the handlers like calling 
        the next handler in this one from the matched router tree, in 
        nodejs:
        app.get('/users', function(req, res, next) {
        
            next()

        });
    */
    ctrl: &mut FlowCtrl
){


    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let zerlog_producer_actor = app_ctx.as_ref().unwrap().actors.clone().unwrap().broker_actors.zerlog_actor;
    let app_ctx = app_ctx.clone();

    let register_notif_req = req.extract::<RegisterNotif>().await.unwrap();
    let get_producer_info = register_notif_req.clone().producer_info;
    let get_consumer_info = register_notif_req.clone().consumer_info;

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
                let cloned_app_ctx = app_ctx.clone();
                let cloned_notif = notif.clone();
                async move{
                    // producing nofit by sending the ProduceNotif message to
                    // the producer actor,
                    match cloned_app_ctx.clone().unwrap().actors.as_ref().unwrap()
                            .broker_actors.notif_actor.send(cloned_notif).await
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

        // fill the response object, salvo returns it by itself to the caller
        res.status_code = Some(StatusCode::OK);
        res.render(Json(
            HoopoeResponse::<&[u8]>{ 
                data: &[], 
                message: "SUCCESS: notification registered succesfully, producer has produced the notification", 
                is_err: false, 
                status: StatusCode::OK.as_u16(),
                meta: Some(
                    serde_json::to_value(&notif).unwrap()
                )
            }
        ));

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
                let cloned_app_ctx = app_ctx.clone();
                let cloned_notif = notif.clone();
                async move{
                    // consuming notif by sending the ConsumeNotif message to 
                    // the consumer actor,
                    match cloned_app_ctx.clone().unwrap().actors.as_ref().unwrap()
                            .broker_actors.notif_actor.send(cloned_notif).await
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

        // fill the response object, salvo returns it by itself to the caller
        res.status_code = Some(StatusCode::OK);
        res.render(Json(
            HoopoeResponse::<&[u8]>{ 
                data: &[], 
                message: "SUCCESS: notification registered succesfully, consumer has started consuming", 
                is_err: false, 
                status: StatusCode::OK.as_u16(),
                meta: Some(
                    serde_json::to_value(&notif).unwrap()
                )
            }
        ));

    } else{

        // fill the response object, salvo returns it by itself to the caller
        let server_time = format!("{}", chrono::Local::now().to_string());
        res.status_code = Some(StatusCode::NOT_ACCEPTABLE);
        res.render(Json(
            HoopoeResponse::<&[u8]>{ 
                data: &[], 
                message: "ERROR: at least one of the consumer or producer info must be available to register a notif", 
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

}

/* -ˋˏ✄┈┈┈┈
    >_  this route is used mainly to retrieve notifications in a short polling 
        manner, client must call this api in an interval to fetch notifs for an
        owner or whole data like every 5 seconds to simulate realtiming in client.
*/
#[endpoint]
pub async fn get_notif(
    req: &mut Request, 
    res: &mut Response,
    depot: &mut Depot,
    ctrl: &mut FlowCtrl
){

    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let zerlog_producer_actor = app_ctx.as_ref().unwrap().actors.clone().unwrap().broker_actors.zerlog_actor;
    let app_ctx = app_ctx.clone();
    let redis_pool = app_ctx.clone().unwrap().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    let actors = app_ctx.clone().unwrap().actors.clone().unwrap();
    let notif_accessor_actor = actors.cqrs_actors.accessors.notif_accessor_actor;
    
    // extracting query params
    let notif_query: EventQuery = req.parse_queries().unwrap();

    // get a single notif by id
    if notif_query.id.is_some(){
        
        // send notif info with this id to the accessor actor to fetch notif info
        let get_notif_data = notif_accessor_actor
            .send(RequestNotifDataByNotifId{ notif_id: notif_query.id.unwrap_or_default() }).await;

        match get_notif_data{
            Ok(notif) => { // result received from the actor thread

                let notif_data = notif.0.await;
                
                // fill the response object, salvo returns it by itself to the caller
                let server_time = format!("{}", chrono::Local::now().to_string());
                res.status_code = Some(StatusCode::OK);
                res.render(Json(
                    HoopoeResponse{ 
                        data: notif_data, 
                        message: "SUCCESS: fetched notification", 
                        is_err: false, 
                        status: StatusCode::OK.as_u16(),
                        meta: Some(
                            serde_json::json!({
                                "server_time": server_time
                            })
                        )
                    }
                ));

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
                
                // fill the response object, salvo returns it by itself to the caller
                let server_time = format!("{}", chrono::Local::now().to_string());
                res.status_code = Some(StatusCode::INTERNAL_SERVER_ERROR);
                res.render(Json(
                    HoopoeResponse::<&[u8]>{ 
                        data: &[], 
                        message: source, 
                        is_err: true, 
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                        meta: Some(
                            serde_json::json!({
                                "server_time": server_time
                            })
                        )
                    }
                ));
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

                    // fill the response object, salvo returns it by itself to the caller
                    let server_time = format!("{}", chrono::Local::now().to_string());
                    res.status_code = Some(StatusCode::OK);
                    res.render(Json(
                        HoopoeResponse{ 
                            data: decoded_data, 
                            message: "SUCCESS: fetched all owner notifications", 
                            is_err: false, 
                            status: StatusCode::OK.as_u16(),
                            meta: Some(
                                serde_json::json!({
                                    "server_time": server_time
                                })
                            )
                        }
                    ));


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

                            // fill the response object, salvo returns it by itself to the caller
                            let server_time = format!("{}", chrono::Local::now().to_string());
                            res.status_code = Some(StatusCode::OK);
                            res.render(Json(
                                HoopoeResponse{ 
                                    data: notifs, 
                                    message: "SUCCESS: fetched all owner notifications", 
                                    is_err: false, 
                                    status: StatusCode::OK.as_u16(),
                                    meta: Some(
                                        serde_json::json!({
                                            "server_time": server_time
                                        })
                                    )
                                }
                            ));
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

                            // fill the response object, salvo returns it by itself to the caller
                            let server_time = format!("{}", chrono::Local::now().to_string());
                            res.status_code = Some(StatusCode::INTERNAL_SERVER_ERROR);
                            res.render(Json(
                                HoopoeResponse::<&[u8]>{ 
                                    data: &[], 
                                    message: &source, 
                                    is_err: true, 
                                    status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                                    meta: Some(
                                        serde_json::json!({
                                            "server_time": server_time
                                        })
                                    )
                                }
                            ));

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
                
                // fill the response object, salvo returns it by itself to the caller
                let server_time = format!("{}", chrono::Local::now().to_string());
                res.status_code = Some(StatusCode::INTERNAL_SERVER_ERROR);
                res.render(Json(
                    HoopoeResponse::<&[u8]>{ 
                        data: &[], 
                        message: &source, 
                        is_err: true, 
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                        meta: Some(
                            serde_json::json!({
                                "server_time": server_time
                            })
                        )
                    }
                ));

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
                    
                    // fill the response object, salvo returns it by itself to the caller
                    let server_time = format!("{}", chrono::Local::now().to_string());
                    res.status_code = Some(StatusCode::OK);
                    res.render(Json(
                        HoopoeResponse{ 
                            data: decoded_data, 
                            message: "SUCCESS: fetched all owner notifications", 
                            is_err: false, 
                            status: StatusCode::OK.as_u16(),
                            meta: Some(
                                serde_json::json!({
                                    "server_time": server_time
                                })
                            )
                        }
                    ));

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
    
                            // fill the response object, salvo returns it by itself to the caller
                            let server_time = format!("{}", chrono::Local::now().to_string());
                            res.status_code = Some(StatusCode::OK);
                            res.render(Json(
                                HoopoeResponse{ 
                                    data: notifs, 
                                    message: "SUCCESS: fetched all owner notifications", 
                                    is_err: false, 
                                    status: StatusCode::OK.as_u16(),
                                    meta: Some(
                                        serde_json::json!({
                                            "server_time": server_time
                                        })
                                    )
                                }
                            ));

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
                            
                            // fill the response object, salvo returns it by itself to the caller
                            let server_time = format!("{}", chrono::Local::now().to_string());
                            res.status_code = Some(StatusCode::INTERNAL_SERVER_ERROR);
                            res.render(Json(
                                HoopoeResponse::<&[u8]>{ 
                                    data: &[], 
                                    message: &source, 
                                    is_err: true, 
                                    status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                                    meta: Some(
                                        serde_json::json!({
                                            "server_time": server_time
                                        })
                                    )
                                }
                            ));
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
                
                // fill the response object, salvo returns it by itself to the caller
                let server_time = format!("{}", chrono::Local::now().to_string());
                res.status_code = Some(StatusCode::INTERNAL_SERVER_ERROR);
                res.render(Json(
                    HoopoeResponse::<&[u8]>{ 
                        data: &[], 
                        message: &source, 
                        is_err: true, 
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                        meta: Some(
                            serde_json::json!({
                                "server_time": server_time
                            })
                        )
                    }
                ));
            }
        }

    }
    
}



pub fn register_controller() -> Router{
    Router::with_path("/v1/events/")
        .hoop(check_passport)
        .oapi_tag("Events") // the passport verifier middleware
        .post(register_notif)
        .get(get_notif)
}