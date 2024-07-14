



use std::error::Error;
use constants::STORAGE_IO_ERROR_CODE;
use context::AppContext;
use interfaces::product::ProductExt;
use lockers::llm::Product;
use serde::{Deserialize, Serialize};
use crate::*;
use crate::models::server::{Server, ServerStatus};
use crate::models::server::Response as HoopoeResponse;
use crate::middlewares::state::set_data;


#[endpoint]
pub async fn check_health(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, 
    ctrl: &mut FlowCtrl
){

    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let app_storage = app_ctx.clone().unwrap().app_storage.clone().unwrap();
    let redis_conn = app_storage.get_redis_pool().await.unwrap().get().await;
    let rmq_conn = app_storage.get_lapin_pool().await.unwrap().get().await;
    let searom_pool = app_storage.get_seaorm_pool().await.unwrap();
    let actors = app_ctx.clone().unwrap().actors.clone().unwrap();
    let db_ping = searom_pool.ping().await;
    let zerlog_producer_actor = actors.broker_actors.zerlog_actor;

    // check db health
    if let Err(e) = db_ping{
        let source = format!("SeaOrm: {}", &e.source().unwrap().to_string()); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
        let err_instance = crate::error::HoopoeErrorResponse::new(
            *STORAGE_IO_ERROR_CODE, // error hex (u16) code
            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
            crate::error::ErrorKind::Storage(crate::error::StorageError::SeaOrm(e)), // the actual source of the error caused at runtime
            &String::from("check.db_ping"), // current method name
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

    // check redis health
    if let Err(e) = redis_conn{
        let source = format!("Redis: {}", &e.source().unwrap().to_string()); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
        let err_instance = crate::error::HoopoeErrorResponse::new(
            *STORAGE_IO_ERROR_CODE, // error hex (u16) code
            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
            crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
            &String::from("check.redis_pool"), // current method name
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

    // check rmq health 
    if let Err(e) = rmq_conn{
        let source = format!("Rmq: {}", &e.source().unwrap().to_string()); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
        let err_instance = crate::error::HoopoeErrorResponse::new(
            *STORAGE_IO_ERROR_CODE, // error hex (u16) code
            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
            crate::error::ErrorKind::Storage(crate::error::StorageError::RmqPool(e)), // the actual source of the error caused at runtime
            &String::from("check.rmq_pool"), // current method name
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

    // fill the response object, salvo returns it by itself to the caller
    let server_time = format!("{}", chrono::Local::now().to_string());
    res.status_code = Some(StatusCode::OK);
    res.render(Json(
        HoopoeResponse{ 
            data: Server::<String>{status: ServerStatus::Alive, data: "".to_string()}, 
            message: &format!("{:?}", ServerStatus::Alive), 
            is_err: false, 
            status: StatusCode::OK.as_u16(),
            meta: Some(
                serde_json::json!({
                    "server_time": server_time
                })
            )
        }
    ));
    
}

#[endpoint]
pub async fn mint(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
    ctrl: &mut FlowCtrl,
    prod: JsonBody<Product>, // used to extract the request body as well as showcasing in swagger ui
){

    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context

    /* ----------- 
        1) get the product info 
        2) check that if it's already minted in db or not 
        3) get its atomic_purchase_status during the app execution 
        4) if it's locked then reject the client request 
        5) otherwise respond the client with: notify you once the minting gets done 
        6) client can get its notification data by calling the api in a short polling manner
    */

    let mut prod = req.extract::<Product>().await.unwrap();
    let (is_being_minted, mut product_receiver) = prod.atomic_purchase_status().await;

    match is_being_minted{
        true => {
            let server_time = format!("{}", chrono::Local::now().to_string());
            res.status_code = Some(StatusCode::LOCKED);
            res.render(Json(
                HoopoeResponse{ 
                    data: prod.clone(), 
                    message: "pid is locked", 
                    is_err: true, 
                    status: StatusCode::LOCKED.as_u16(),
                    meta: Some(
                        serde_json::json!({
                            "server_time": server_time
                        })
                    )
                }
            )); // reject the request
        },
        false => {
            // if you want to use while let Some the prod must be cloned in every iteration
            // hence using if let Some is the best option in here to avoid using clone.
            if let Some(product) = product_receiver.recv().await{
                log::info!("received updated product info: {:?}", product);
                
                let server_time = format!("{}", chrono::Local::now().to_string());
                res.status_code = Some(StatusCode::OK);
                res.render(Json(
                    HoopoeResponse{ 
                        data: prod.clone(), 
                        message: "notify you once the minting gets done", 
                        is_err: false, 
                        status: StatusCode::OK.as_u16(),
                        meta: Some(
                            serde_json::json!({
                                "server_time": server_time
                            })
                        )
                    }
                )); // the minting was successfully started
            }
        }
    }
}

pub fn register_controller() -> Router{

    Router::with_path("/v1/health/")
        .oapi_tag("Health")
        .hoop(set_data) // this api must be passed as middleware to update the state of depot before accessing the check_health route api
        .push(
            Router::with_path("check")
                .get(check_health)
        )
        .push(
            Router::with_path("atomic-mint")
                .post(mint)
        )
    
}