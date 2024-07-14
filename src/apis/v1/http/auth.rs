


use constants::{CRYPTER_THEMIS_ERROR_CODE, STORAGE_IO_ERROR_CODE};
use context::AppContext;
use log::kv::source;
use wallexerr::misc::{SecureCellConfig, Wallet};
use crate::*;
use base58::ToBase58;
use std::error::Error;
use crate::models::server::Response as HoopoeResponse;
use interfaces::passport::Passport;



#[derive(Deserialize, ToSchema)]
struct GenTokenQueries {
    exp: Option<i64>,
    scope: Option<String>
}

#[endpoint]
pub async fn generate_access_token(
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
    ctrl: &mut FlowCtrl,
    params: QueryParam<GenTokenQueries, true> // query param is required, showcasing in swagger ui
){


    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let redis_pool = app_ctx.clone().unwrap().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    
    let actors = app_ctx.clone().unwrap().actors;
    let zerlog_producer_actor = actors.unwrap().broker_actors.zerlog_actor;
    
    let params: GenTokenQueries = req.parse_queries().unwrap();
    let exp_time = params.exp.unwrap_or_default();
    let scope = params.scope.unwrap_or_default();

    match redis_pool.get().await{
        Ok(mut redis_conn) => {

            match req.check_secret(app_ctx.clone()).await{
                Ok(sec) => {
        
                    let now = chrono::Local::now();
                    let token_time_with_exp = now + chrono::Duration::seconds(exp_time); // 5 mins or longer expiration time
                    let token_time = token_time_with_exp.timestamp();

                    let mut secure_cell_config = SecureCellConfig{ 
                        secret_key: sec.clone(), 
                        passphrase: sec.clone(), 
                        data: format!("{}", token_time).as_bytes().to_vec()
                    };
                    
                    match Wallet::secure_cell_encrypt(&mut secure_cell_config){
                        Ok(token) => {
                            
                            let base58 = token.to_base58();
                            let redis_key = format!("access_token_for_{}", scope);
                            let _: () = redis_conn.set_ex(redis_key, &base58, exp_time as u64).await.unwrap();

                            // fill the response object, salvo returns it by itself to the caller
                            let server_time = format!("{}", chrono::Local::now().to_string());
                            res.status_code = Some(StatusCode::OK);
                            res.render(Json(
                                HoopoeResponse{ 
                                    data: base58, 
                                    message: "SUCCESS: token time generated", 
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
                                *CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                                &String::from("generate_access_token.get_secret.secure_cell_encrypt"), // current method name
                                Some(&zerlog_producer_actor)
                            ).await;
                            
                            // fill the response object, salvo returns it by itself to the caller
                            let server_time = format!("{}", chrono::Local::now().to_string());
                            res.status_code = Some(StatusCode::OK);
                            res.render(Json(
                                HoopoeResponse::<&[u8]>{ 
                                    data: &[], 
                                    message: &source, 
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
                    }
        
                },
                Err(resp_err) => {
                    /* -------------------
                        in order to mutate the content of a mutable pointer we should
                        dereferencing it first then update the address which is pointing 
                        to the old content with a new content, so in here we're:
                        updating the state of the response object with the new one coming 
                        from the error part, hence we're dereferencing it to override its 
                        content with the new one, it would be same address but new value 
                        or no binding.
                    */
                    *res = resp_err;

                    /* 
                        // new binding; new address and new value
                        // not that the res object must be defined 
                        // a a mutable object inside the handler
                        // res = &mut res_err;

                        // example of new valud vs new binding:
                        let mut resp = String::from("");
                        let mut p = &mut resp;
                        println!("addres of resp: {:p}", p);
                        
                        // mutating with dereferencing
                        *p = String::from("wildonion"); // value
                        println!("addres of resp: {:p}", p); // same address
                            println!("value of resp: {}", resp);
                            
                        // mutating with new binding
                        let new_binding = &mut String::from("changed");
                        p = new_binding;
                        println!("addres of resp: {:p}", p); // new address
                    */
                },
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
            
            // fill the response object, salvo returns it by itself to the caller
            let server_time = format!("{}", chrono::Local::now().to_string());
            res.status_code = Some(StatusCode::OK);
            res.render(Json(
                HoopoeResponse::<&[u8]>{ 
                    data: &[], 
                    message: &source, 
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
    }

}

pub fn register_controller() -> Router{
        
    Router::with_path("/v1/auth/")
        .oapi_tag("Auth")
        .push(
            Router::with_path("generate-access-token")
                .post(generate_access_token)
        )

}