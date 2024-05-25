


use std::error::Error;

use crate::{error::HoopoeErrorResponse, models::http::Response, *};
use config::EnvExt;
use consts::STORAGE_IO_ERROR_CODE;
use models::user::UserData;
use types::*;

use self::appstate::AppState;


// async methods and functions need to be Send in order to share them
// between threads for future solvation
#[trait_variant::make(PassportSend: Send)]
pub trait Passport{

    type Request;

    async fn get_user(&self) -> Result<UserData, HoopoeHttpResponse>;
    async fn get_secret(&self, app_state: web::Data<AppState>) -> Result<String, HoopoeHttpResponse>;
    async fn verify_token_time(&self, app_state: web::Data<AppState>, scope: &str) -> Result<String, HoopoeHttpResponse>;
}

// only traits defined in the current crate can be implemented for types defined outside of the crate
// Passport trait is defined here and must be implemented in this crate for HttpRequest which is a type
// outside of this crate, we can't add the following codes in server or any other crates.
impl Passport for actix_web::HttpRequest{

    type Request = Self;

    async fn get_user(&self) -> Result<UserData, HoopoeHttpResponse>{

        let req = self; // self is a mutable reference to the HttpRequest object

        let jwt = "";
        let api = requests::Request::builder(
            "/health/check-token", 
            jwt,
            &String::from("")
        );

        todo!()
    }

    async fn get_secret(&self, app_state: web::Data<AppState>) -> Result<String, HoopoeHttpResponse>{

        let req = self;
        let config = app_state.config.as_ref().unwrap();
        let secret_key = &config.vars.SECRET_KEY;

        let headers = req.headers();
        let get_secret_key = headers.get("x-api-key");
        let Some(header_secret_key) = get_secret_key else{
            
            let res = HttpResponse::build(StatusCode::NOT_ACCEPTABLE).json(
                Response::<'_, &[u8]>{
                    data: Some(&[]),
                    message: &format!("ERROR: empty header secret key"),
                    status: StatusCode::NOT_ACCEPTABLE.as_u16(),
                    is_error: true,
                    meta: None
                }
            );
            return Err(
                Ok(res)
            );
        };

        if header_secret_key != secret_key{

            let res = HttpResponse::build(StatusCode::FORBIDDEN).json(
                Response::<'_, &[u8]>{
                    data: Some(&[]),
                    message: &format!("ERROR: wrong secret key"),
                    status: StatusCode::FORBIDDEN.as_u16(),
                    is_error: true,
                    meta: None
                }
            );
            return Err(
                Ok(res)
            );

        }

        Ok(header_secret_key.to_str().unwrap_or_default().to_string())

    }

    async fn verify_token_time(&self, app_state: web::Data<AppState>, scope: &str) -> Result<String, HoopoeHttpResponse>{

        let req = self;
        let redis_pool = app_state.clone().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
        let zerlog_producer_actor = app_state.as_ref().actors.clone().unwrap().producer_actors.zerlog_actor;

        let headers = req.headers();
        let get_secret_key = headers.get("x-api-key");
        let Some(header_secret_key) = get_secret_key else{
            
            let res = HttpResponse::build(StatusCode::NOT_ACCEPTABLE).json(
                Response::<'_, &[u8]>{
                    data: Some(&[]),
                    message: &format!("ERROR: empty header token time"),
                    status: StatusCode::NOT_ACCEPTABLE.as_u16(),
                    is_error: true,
                    meta: None
                }
            );
            return Err(
                Ok(res)
            );
        };

        match redis_pool.get().await{
            Ok(mut redis_conn) => {
    
                let redis_key = format!("access_token_for_{}", scope);
                let is_key_there: bool = redis_conn.exists(&redis_key).await.unwrap();
                if is_key_there{

                    let token_time: String = redis_conn.get(&redis_key).await.unwrap();
                    if header_secret_key != &token_time{

                        let res = HttpResponse::build(StatusCode::FORBIDDEN).json(
                            Response::<'_, &[u8]>{
                                data: Some(&[]),
                                message: &format!("ERROR: wrong token time "),
                                status: StatusCode::FORBIDDEN.as_u16(),
                                is_error: true,
                                meta: None
                            }
                        );
                        return Err(
                            Ok(res)
                        );
            
                    } else{

                        /*  -ˋˏ✄┈┈┈┈
                            this is not necessary to be checked cause we're using redis set_ex key
                            to set an expirable token time and if we couldn't get the token time from 
                            redis means it has been expired
                        */
                        let now = chrono::Local::now().timestamp();
                        let parsed_token_time = token_time.parse::<i64>().unwrap_or_default();
                        if now > parsed_token_time{
                            let res = HttpResponse::build(StatusCode::FORBIDDEN).json(
                                Response::<'_, &[u8]>{
                                    data: Some(&[]),
                                    message: &format!("ERROR: ahead of time for token time"),
                                    status: StatusCode::FORBIDDEN.as_u16(),
                                    is_error: true,
                                    meta: None
                                }
                            );
                            return Err(
                                Ok(res)
                            );
                        }
                        // -ˋˏ✄┈┈┈┈

                        return Ok(token_time);
                    }

                } else{
                    let res = HttpResponse::build(StatusCode::FORBIDDEN).json(
                        Response::<'_, &[u8]>{
                            data: Some(&[]),
                            message: &format!("ERROR: expired token time or wrong scope"),
                            status: StatusCode::FORBIDDEN.as_u16(),
                            is_error: true,
                            meta: None
                        }
                    );
                    return Err(
                        Ok(res)
                    );
                }
    
            },
            Err(e) => {
                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                let err_instance = crate::error::HoopoeErrorResponse::new(
                    *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                    crate::error::ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // the actual source of the error caused at runtime
                    &String::from("generate_access_token.redis_pool"), // current method name
                    Some(&zerlog_producer_actor)
                ).await;
                return Err(Ok(err_instance.error_response()));
            }
        }

    }

}