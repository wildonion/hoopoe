


use crate::{actors::cqrs::accessors::notif::NotifDataResponse, models::event::{NotifData, NotifQuery}};
use redis::AsyncCommands;
pub use super::*;


#[get("/hoop/get/")]
pub(self) async fn get_hoop(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    todo!()

}

#[get("/notif/get/")]
pub(self) async fn get_notif(
    req: HttpRequest,
    notif_query: web::Query<NotifQuery>,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{

    let redis_pool = app_state.clone().app_storage.clone().unwrap().get_redis_pool().await.unwrap();
    let actors = app_state.clone().actors.clone().unwrap();
    let notif_accessor_actor = actors.cqrs_actors.accessors.notif_accessor_actor;
    let zerlog_producer_actor = actors.producer_actors.zerlog_actor;
    
    let notif_query = notif_query.to_owned().0;
    
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
                            &String::from("register_notif.producer_actors.notif_actor.send"), // current method name
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

pub mod exports{
    pub use super::get_hoop;
    pub use super::get_notif;
}