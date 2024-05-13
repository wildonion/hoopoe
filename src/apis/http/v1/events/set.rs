


pub use super::*;



/* -ˋˏ✄┈┈┈┈
    >_  this route is used to either produce a data to RMQ exchange 
        or start consuming in the background, look at the postman
        collection for more details.
*/
#[post("/notif/register")]
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


    match req.check_token_time(app_state.clone(), "write").await{
        Ok(token_time) => {

            if get_producer_info.is_some(){
                let producer_info = get_producer_info.unwrap();
                let notif = producer_info.info;
                
                /* -ˋˏ✄┈┈┈┈
                    >_ sending notif to RMQ in the background
                        you may want to produce data in a loop{} constantly 
                        like sending gps data contains lat and long into the
                        exchange so other consumers be able to consume constantly
                        as the data coming at a same time, kindly put the sending
                        message logic to actor inside a loop{}
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
                                    Ok(_) => {},
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
                let notif = consumer_info.info;
                
                /* -ˋˏ✄┈┈┈┈
                    >_ start consuming in the background
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
                                    Ok(_) => {},
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


pub mod exports{
    pub use super::register_notif;
}