


use consts::STORAGE_IO_ERROR_CODE;
use interfaces::passport::Passport;
use wallexerr::misc::{SecureCellConfig, Wallet};
use crate::consts::CRYPTER_THEMIS_ERROR_CODE;
use base58::{ToBase58, FromBase58};
use self::models::event::GenerateTokenTimeQuery;

pub use super::*;



#[post("/generate-access-token/")]
pub(self) async fn generate_access_token(
    req: HttpRequest,
    exp_query: web::Query<GenerateTokenTimeQuery>,
    app_state: web::Data<AppState>
) -> HoopoeHttpResponse{
    
    let redis_pool = app_state.clone().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    let actors = app_state.clone().actors.clone().unwrap();
    let zerlog_producer_actor = actors.producer_actors.zerlog_actor;
    let generate_token_query = exp_query.to_owned().0;

    match redis_pool.get().await{
        Ok(mut redis_conn) => {

            match req.get_secret(app_state.clone()).await{
                Ok(sec) => {
        
                    let now = chrono::Local::now();
                    let token_time_with_exp = now + chrono::Duration::seconds(generate_token_query.exp_time.unwrap_or(300) as i64);
                    let token_time = token_time_with_exp.timestamp();

                    let mut secure_cell_config = SecureCellConfig{ 
                        secret_key: sec.clone(), 
                        passphrase: sec.clone(), 
                        data: format!("{}", token_time).as_bytes().to_vec()
                    };
                    
                    match Wallet::secure_cell_encrypt(&mut secure_cell_config){
                        Ok(token) => {
                            
                            let base58 = token.to_base58();
                            let redis_key = format!("access_token_for_{}", generate_token_query.scope.unwrap_or_default());
                            let _: () = redis_conn.set_ex(redis_key, &base58, generate_token_query.exp_time.unwrap_or(300)).await.unwrap();
        
                            resp!{
                                String,
                                base58,
                                {
                                    Some(serde_json::json!({
                                        "server_time": chrono::Local::now().to_string() 
                                    }))
                                },
                                &format!("SUCCESS: token time generated"),
                                StatusCode::OK,
                                None::<Cookie<'_>>,
                            }
        
                        },
                        Err(e) => {
                            let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                            let err_instance = crate::error::HoopoeErrorResponse::new(
                                *CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                                &String::from("register_notif.producer_actors.notif_actor.send"), // current method name
                                Some(&zerlog_producer_actor)
                            ).await;
                            return Ok(err_instance.error_response());
                        }
                    }
        
                },
                Err(resp_err) => resp_err
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
            return Ok(err_instance.error_response());
        }
    }

}

pub mod exports{
    pub use super::generate_access_token;
}