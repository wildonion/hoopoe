



use std::error::Error;
use constants::STORAGE_IO_ERROR_CODE;
use context::AppContext;
use interfaces::crypter::Crypter; // use it here so we can call the encrypt and decrypt methods on the &[u8]
use interfaces::product::ProductExt;
use lockers::llm::Product;
use models::event::UserSecureCellConfig;
use serde::{Deserialize, Serialize};
use crate::*;
use crate::models::server::{Server, ServerStatus};
use crate::models::server::Response as HoopoeResponse;
use crate::middlewares::state::set_data;
use base58::{ToBase58, FromBase58}; 




#[endpoint]
pub async fn check_health(
    req: &mut Request, 
    res: &mut Response, 
    depot: &mut Depot, 
    ctrl: &mut FlowCtrl // with this we can control the flow of each route like executing the next handler
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
pub async fn home(
    req: &mut Request, 
    res: &mut Response,
    depot: &mut Depot,
    ctrl: &mut FlowCtrl // with this we can control the flow of each route like executing the next handler
){

    res.render(Text::Html(constants::HOME_HTML));

} 

#[endpoint]
pub async fn mint(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
    ctrl: &mut FlowCtrl, // with this we can control the flow of each route like executing the next handler
    // https://salvo.rs/book/features/openapi.html#extractors (QueryParam, HeaderParam, CookieParam, PathParam, FormBody, JsonBody)
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
                                "server_time": server_time,
                                "minted_product": product
                            })
                        )
                    }
                )); // the minting was successfully started
            }
        }
    }

}


#[endpoint]
pub async fn picer(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
    ctrl: &mut FlowCtrl, // with this we can control the flow of each route like executing the next handler
    // https://salvo.rs/book/features/openapi.html#extractors (QueryParam, HeaderParam, CookieParam, PathParam, FormBody, JsonBody)
    secure_cell_config: FormBody<UserSecureCellConfig>, // used to extract the request body as well as showcasing in swagger ui
){

    // getting the multipart file from the request
    let pic = req.file("pic").await;
    match pic{
        Some(file) => {
            let default_file_name = format!("img-{}", chrono::Local::now().timestamp());
            let dest = format!("{}/{}", constants::ASSETS_IMG_DIR, file.name().unwrap_or(&default_file_name));
            
            // use Path to load a path as u8 slice buffer
            let path_buff_dest = std::path::Path::new(&dest);
            
            // copy the u8 buffer of the image to the passed in pathbuf of dest 
            // the method also copies the permission bits to the dest path
            match tokio::fs::copy(file.path(), path_buff_dest).await{
                Ok(size) => {

                    // open the file and load it into buffer for encryption
                    let mut img = tokio::fs::File::open(&dest).await.unwrap();
                    let mut img_buffer = vec![];
                    let img_bytes = img.read_to_end(&mut img_buffer).await;


                    // consider uploading the image to aws or digispaces
                    // ...

                    // encrypt the content of the file
                    let mut secure_cell_config = wallexerr::misc::SecureCellConfig{ 
                        secret_key: secure_cell_config.clone().secret_key, 
                        passphrase: secure_cell_config.clone().passphrase, 
                        data: img_buffer.clone()
                    };
                    img_buffer.encrypt(&mut secure_cell_config);

                    // store the encrypted content in another file
                    let encrypted_data = secure_cell_config.data;
                    let enc_path = format!("{}.enc", dest);
                    let mut f = tokio::fs::File::create(enc_path).await.unwrap();
                    f.write_all(&encrypted_data).await;
                    
                    // let the encrypted file only be there 
                    tokio::fs::remove_file(dest).await;

                    // fill the response object
                    let server_time = format!("{}", chrono::Local::now().to_string());
                    res.status_code = Some(StatusCode::OK);
                    res.render(Json(
                        HoopoeResponse::<&[u8]>{ 
                            data: &[], 
                            message: &format!("uploaded successfully, wrote {} bytes", size), 
                            is_err: false, 
                            status: StatusCode::OK.as_u16(),
                            meta: Some(
                                serde_json::json!({
                                    "server_time": server_time
                                })
                            )
                        }
                    )); // the minting was successfully started
                },
                Err(e) => { // can't copy the file into dest path 
                    
                    // fill the response objcet 
                    let server_time = format!("{}", chrono::Local::now().to_string());
                    res.status_code = Some(StatusCode::NOT_ACCEPTABLE);
                    res.render(Json(
                        HoopoeResponse::<&[u8]>{ 
                            data: &[], 
                            message: &e.to_string(), 
                            is_err: false, 
                            status: StatusCode::NOT_ACCEPTABLE.as_u16(),
                            meta: Some(
                                serde_json::json!({
                                    "server_time": server_time
                                })
                            )
                        }
                    )); // the minting was successfully started

                    // handle the zerlog error
                    let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *constants::FILE_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::File(crate::error::FileEror::ReadWrite(e)), // the actual source of the error caused at runtime
                        &String::from("picer.tokio::fs::copy"), // current method name
                        None
                    ).await;

                }
            }
            
        },
        None => {
            let server_time = format!("{}", chrono::Local::now().to_string());
            res.status_code = Some(StatusCode::BAD_REQUEST);
            res.render(Json(
                HoopoeResponse::<&[u8]>{ 
                    data: &[], 
                    message: "found no file in request", 
                    is_err: true, 
                    status: StatusCode::BAD_REQUEST.as_u16(),
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


pub fn register_controller() -> Router{

    Router::with_path("/v1/health/")
        .oapi_tag("Health")
        .hoop(set_data) // this api must be passed as middleware to update the state of depot before accessing the check_health route api
        .push(
            Router::with_path("check")
                .get(check_health)
        )
        .push(
            Router::with_path("upload")
                .post(picer)
        )
        .push(
            Router::with_path("atomic-mint")
                .post(mint)
        )
        .push(
            Router::with_path("home")
                .get(home)
        )
}