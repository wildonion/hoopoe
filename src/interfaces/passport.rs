

use deadpool_redis::redis::AsyncCommands;
use std::error::Error;
use crate::{error::HoopoeErrorResponse, *};
use config::EnvExt;
use constants::STORAGE_IO_ERROR_CODE;
use models::{server::Response, user::UserData};
use types::*;
use self::context::AppContext;
use crate::models::server::Response as HoopoeResponse;


/* -----------------
    https://boats.gitlab.io/blog/post/async-methods-i/
    GATs can now have generic params which allows us to have async 
    method in traits cause the future returned by an async function 
    captures all lifetimes inputed into the function, in order 
    to express that lifetime relationship, the Future type needs 
    to be generic over a lifetime, so that it can capture the 
    lifetime from the self argument on the method.
    generic and lifetime wasn't supported in GAT till Rust 1.79
    by the result we can have async methods in traits without 
    using third party crates.
    the fact that async method wasn't supported in traits was
    due to the unspported feature of generic and lifetime in GAT
    which wouldn't allow to return a future object from the trait
    method cause future obejcts capture lifetimes forces us to pass
    the GAT with lifetime as the return type of async trait method
    hence using the GAT as the return type of async trait method 
    wasn't supported therefore having future objects in trait method 
    return type was invalid.
    it's notable that traits with async methods can't be object safe 
    and Boxed with Box<dyn we can't use the builtin async method 
    instead we should either use the async_trait crate or remove 
    the async keywords.
*/
pub trait Passport{

    type Request;

    async fn get_user(&self) -> Result<UserData, salvo::Response>;
    async fn check_secret(&self, app_state: Option<AppContext>) -> Result<String, salvo::Response>;
    async fn verify_token_time(&self, app_state: Option<AppContext>, scope: &str) -> Result<String, salvo::Response>;
}

// only traits defined in the current crate can be implemented for types defined outside of the crate
// Passport trait is defined here and must be implemented in this crate for HttpRequest which is a type
// outside of this crate, we can't add the following codes in server or any other crates.
impl Passport for salvo::Request{

    type Request = Self;

    async fn get_user(&self) -> Result<UserData, salvo::Response>{

        let req = self; // self is a mutable reference to the HttpRequest object

        let jwt = "";
        let api = requests::Fetch::builder(
            "/health/check-token", 
            jwt,
            &String::from("")
        );


        Ok(
            UserData{}
        )

    }

    async fn check_secret(&self, app_state: Option<AppContext>) -> Result<String, salvo::Response>{

        let mut resp = salvo::Response::new(); // create a new response object
        let req = self;
        let config = app_state.as_ref().unwrap().config.as_ref().unwrap();
        let secret_key = &config.vars.SECRET_KEY;

        let headers = req.headers();
        let get_secret_key = headers.get("secret");
        let Some(header_secret_key) = get_secret_key else{
            
            let server_time = format!("{}", chrono::Local::now().to_string());
            resp.status_code = Some(StatusCode::NOT_ACCEPTABLE);
            resp.render(Json(
                HoopoeResponse::<&[u8]>{ 
                    data: &[], 
                    message: "ERROR: empty header secret key", 
                    is_err: true, 
                    status: StatusCode::NOT_ACCEPTABLE.as_u16(),
                    meta: Some(
                        serde_json::json!({
                            "server_time": server_time
                        })
                    )
                }
            ));
            return Err(resp);
        };

        if header_secret_key != secret_key{

            let server_time = format!("{}", chrono::Local::now().to_string());
            resp.status_code = Some(StatusCode::FORBIDDEN);
            resp.render(Json(
                HoopoeResponse::<&[u8]>{ 
                    data: &[], 
                    message: "ERROR: wrong secret key", 
                    is_err: true, 
                    status: StatusCode::FORBIDDEN.as_u16(),
                    meta: Some(
                        serde_json::json!({
                            "server_time": server_time
                        })
                    )
                }
            ));
            return Err(resp);

        }

        Ok(header_secret_key.to_str().unwrap_or_default().to_string())

    }

    async fn verify_token_time(&self, app_state: Option<AppContext>, scope: &str) -> Result<String, salvo::Response>{

        let req = self;
        let mut resp = salvo::Response::new(); // create a new response object
        let redis_pool = app_state.as_ref().unwrap().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
        let zerlog_producer_actor = app_state.as_ref().unwrap().actors.clone().unwrap().broker_actors.zerlog_actor;

        let headers = req.headers();
        let get_secret_key = headers.get("token_time");
        let Some(header_secret_key) = get_secret_key else{
            
            let server_time = format!("{}", chrono::Local::now().to_string());
            resp.status_code = Some(StatusCode::NOT_ACCEPTABLE);
            resp.render(Json(
                HoopoeResponse::<&[u8]>{ 
                    data: &[], 
                    message: "ERROR: empty header token time", 
                    is_err: true,
                    status: StatusCode::NOT_ACCEPTABLE.as_u16(), 
                    meta: Some(
                        serde_json::json!({
                            "server_time": server_time
                        })
                    )
                }
            ));
            return Err(resp);

        };

        match redis_pool.get().await{
            Ok(mut redis_conn) => {
    
                let redis_key = format!("access_token_for_{}", scope);
                let is_key_there: bool = redis_conn.exists(&redis_key).await.unwrap();
                if is_key_there{

                    let token_time: String = redis_conn.get(&redis_key).await.unwrap();
                    if header_secret_key != &token_time{

                        let server_time = format!("{}", chrono::Local::now().to_string());
                        resp.status_code = Some(StatusCode::FORBIDDEN);
                        resp.render(Json(
                            HoopoeResponse::<&[u8]>{ 
                                data: &[], 
                                message: "ERROR: wrong token time", 
                                is_err: true,
                                status: StatusCode::FORBIDDEN.as_u16(), 
                                meta: Some(
                                    serde_json::json!({
                                        "server_time": server_time
                                    })
                                )
                            }
                        ));
                        return Err(resp);
            
                    } else{

                        /*  -ˋˏ✄┈┈┈┈
                            this is not necessary to be checked cause we're using redis set_ex key
                            to set an expirable token time and if we couldn't get the token time from 
                            redis in the first place it means it has been expired.
                        */
                        let now = chrono::Local::now().timestamp();
                        let parsed_token_time = token_time.parse::<i64>().unwrap_or_default();
                        if parsed_token_time > now{
                            let server_time = format!("{}", chrono::Local::now().to_string());
                            resp.status_code = Some(StatusCode::FORBIDDEN);
                            resp.render(Json(
                                HoopoeResponse::<&[u8]>{ 
                                    data: &[], 
                                    message: "ERROR: expired token time", 
                                    is_err: true,
                                    status: StatusCode::FORBIDDEN.as_u16(), 
                                    meta: Some(
                                        serde_json::json!({
                                            "server_time": server_time
                                        })
                                    )
                                }
                            ));
                            return Err(resp);
                        }
                        // -ˋˏ✄┈┈┈┈

                        return Ok(token_time);
                    }

                } else{

                    let server_time = format!("{}", chrono::Local::now().to_string());
                    resp.status_code = Some(StatusCode::FORBIDDEN);
                    resp.render(Json(
                        HoopoeResponse::<&[u8]>{ 
                            data: &[], 
                            message: "ERROR: expired token time or wrong scope", 
                            is_err: true,
                            status: StatusCode::FORBIDDEN.as_u16(), 
                            meta: Some(
                                serde_json::json!({
                                    "server_time": server_time
                                })
                            )
                        }
                    ));
                    return Err(resp);
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
                
                let server_time = format!("{}", chrono::Local::now().to_string());
                resp.status_code = Some(StatusCode::INTERNAL_SERVER_ERROR);
                resp.render(Json(
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
                return Err(resp);

            }
        }

    }

}