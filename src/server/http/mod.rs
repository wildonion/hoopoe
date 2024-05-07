


use std::collections::HashMap;
use crate::*;
use types::HoopoeHttpResponse;
use crate::actors::sse::{AddClient, BroadcastEvent};
use actix::{actors, Addr};
use config::{Env as ConfigEnv, Context};
use config::EnvExt;
use s3::Storage;
use consts::*;
use crate::actors::sse::Broadcaster;
use appstate::*;
use appstate::AppState;
use crate::error::*;
use crate::consts::SERVER_IO_ERROR_CODE;


// use this method to add new sse client
pub async fn sse_client(app_state: web::Data<AppState>) -> HoopoeHttpResponse{

    // since unwrap() takes the ownership of the isntance and the app_state hence we should clone the 
    // sse_broadcaster to prevent it from moving, as_ref() and as_mut() is not working here
    // DerefMut is not implemented for AppState
    let actors = app_state.actors.as_ref().unwrap();
    // send add client message to the actor
    let sse_broadcaster = actors.sse_actor.clone();
    sse_broadcaster.send(AddClient{}).await;

    let resp = models::http::Response::<&[u8]>{
        data: Some(&[]),
        message: consts::SEE_EVENT_SENT,
        status: 200,
        is_error: false,
        meta: None
    };
    return Ok(HttpResponse::Ok().json(resp))
}

// use this method to broadcast new event
pub async fn broadcast_event(
    app_state: web::Data<AppState>,
    event_info: web::Path<(String, String)>,
) -> HoopoeHttpResponse{
    
    let topic = event_info.clone().0;
    let event = event_info.clone().1;
    // since unwrap() takes the ownership of the isntance and the app_state hence we should clone the 
    // sse_broadcaster to prevent it from moving, as_ref() and as_mut() is not working here
    // DerefMut is not implemented for AppState
    let actors = app_state.actors.as_ref().unwrap();
    let mut sse_broadcaster = actors.sse_actor.clone();
    // send broadcast message to the actor
    sse_broadcaster.send(BroadcastEvent{
        topic,
        event
    }).await;

    let resp = models::http::Response::<&[u8]>{
        data: Some(&[]),
        message: consts::SEE_CLIENT_ADDED,
        status: 200,
        is_error: false,
        meta: None
    };
    return Ok(HttpResponse::Ok().json(resp))
}

#[macro_export]
macro_rules! bootsteap_http {
    (
        $app_state:expr,
    ) => {
        
        {   

            pub use self::*;

            let tcp_listener = std::net::TcpListener::bind(
                format!("{}:{}", 
                        $app_state.config.as_ref().unwrap().vars.HOST, 
                        $app_state.config.as_ref().unwrap().vars.HTTP_PORT.parse::<u16>().unwrap()
                )).unwrap();
            
            let shared_app_state = web::Data::new($app_state.clone());
            let s = match HttpServer::new(move ||{
                App::new()
                    /* 
                        SHARED STATE DATA
                    */
                    .app_data(web::Data::clone(&shared_app_state.clone())) // the whole app state: s3, actors and configs
                    .wrap(Cors::permissive())
                    .wrap(Logger::default())
                    .wrap(Logger::new("%a %{User-Agent}i %t %P %r %s %b %T %D"))
                    .wrap(middleware::Compress::default())
                    /*
                        INIT WS SUBSCRIBE SERVICE
                    */
                    .service(
                        actix_web::web::scope("/v1/stream")
                            .configure(services::stream::init)
                    )
                    /*
                        INIT HEALTH SERIVE
                    */
                    .service(
                        actix_web::web::scope("/v1/health")
                            .configure(services::health::init)
                    )
                    /*
                        INIT EVENTS SERIVE
                    */
                    .service(
                        actix_web::web::scope("/v1/events")
                            .configure(services::events::init)
                    )
                }) 
                .listen(tcp_listener){ // bind the http server on the passed in tcp listener cause after all http is a tcp based protocol!
                    Ok(server) => {
                        server
                            // spawning 10 workers, once the workers are created, they each receive 
                            // a separate application factory instance to handle requests, each worker thread 
                            // processes its requests sequentially, apis which block the current thread 
                            // will cause the current worker thread to stop processing new requests
                            // async apis get executed concurrently by worker threads and thus don't 
                            // block execution like each worker thread which contains the app instance
                            // handles coming requests to async apis as an async task by spawning them
                            // into tokio task with tokio::spawn()
                            .workers(10) 
                            .run() /* actix web http+ws server runs in the same thread that actix has ran */
                            .await
                    },
                    Err(e) => {

                        /* custom error handler */
                        use crate::error::{ErrorKind, ServerError::{ActixWeb, Ws}, HoopoeErrorResponse};
                        let error_content = &e.to_string();
                        let error_content = error_content.as_bytes().to_vec();
                        let mut error_instance = HoopoeErrorResponse::new(
                            *crate::consts::SERVER_IO_ERROR_CODE, // error code
                            error_content, // error content
                            ErrorKind::Server(ActixWeb(e)), // error kind
                            "HttpServer::new().bind", // method
                            None
                        ).await;
        
                        panic!("panicked when running actix web server at {}", chrono::Local::now());

                    }
                };


            /* 
                this can't be reachable unless we hit the ctrl + c since the http server will be 
                built inside multiple threads in which all server instances will be ran constanly 
                in the background loop, and must be the last thing that can be reachable before 
                sending Ok(()) from the main function, it's like the app will be halted in this
                section of the code cause anything after those threads rquires all the threads to 
                be stopped and joined in order to execute the logic after running the http server, 
                which this can be done by stopping all of the threads using ctrl + c as well as 
                the background loop{}
            */
            // info!("➔ 🎛️ starting hoopoe on address: [{}:{}]", host, port);
            
            s // actix concurrent server runs in 10 worker thread

        }
    };
}