

use std::error::Error;

use crate::*;
use actix_web::{cookie::Cookie, http::StatusCode};
use consts::{HTTP_RESPONSE_ERROR_CODE, MAILBOX_CHANNEL_ERROR_CODE};
use futures::StreamExt;
use crate::types::HoopoeHttpResponse;
use appstate::AppState;
use redis::{AsyncCommands, Commands};


/* ――――――――――――――――――――――――――――――
 ╰┈➤ realtime streaming over ws
    users can start subscribing to this route to stablish
    and maintain a new realtime ws channel and full-duplex 
    communication with a long-lived connection to interact 
    with server and send realtime data like chats and user 
    status during the app execution to the server.
 */
#[get("/")]
pub(self) async fn index(
    req: HttpRequest,
    mut stream: web::Payload, // in ws it is possible to convert a request's Payload to a stream of ws::Message with a web::Payload and then use stream combinators to handle actual messages which is done by using ws actors
    payload: Multipart,
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{ 

    // we can convert payload stream into the ws messages using ws msg handlers
    let mut buffer = vec![];
    while let Some(chunk) = stream.next().await{ // reading future objects from the streamer is a mutable process
        match chunk{
            Ok(b) => {
                buffer.extend_from_slice(&b);
            },
            Err(e) => {
                resp!{
                    &[u8],
                    &[],
                    {
                        Some(serde_json::json!({
                            "server_time": chrono::Local::now().to_string() 
                        }))
                    },
                    &format!("ERROR: can't read payload stream"),
                    StatusCode::NOT_ACCEPTABLE,
                    None::<Cookie<'_>>,
                }
            }
        }
    }

    let actors = app_state.clone().actors.clone().unwrap();
    let storage = app_state.clone().app_storage.clone();
    let db = storage.unwrap().get_seaorm_pool().await.unwrap();
    let zerlog_producer_actor = actors.producer_actors.zerlog_actor;

    // some user inputs validation
    // starting session actor probably!
    // ...

    /* -ˋˏ✄┈┈┈┈
        when you use ws::start to initiate a WebSocket connection, it takes care of setting up 
        the WebSocket connection and managing it independently. Once the WebSocket connection 
        is established, the HTTP connection transitions to a WebSocket connection and remains 
        open as long as the WebSocket session is active, the ws::start function is used to start 
        the WebSocket connection, transitioning the HTTP connection to a WebSocket connection.
    */
    // use this: https://github.com/wildonion/gem/blob/master/core/panel/apis/clp/chat.rs
    // let ws_resp = ws::start(
    //     ws_session_actor, 
    //     &req, 
    //     stream
    // );
    
    todo!()

}


pub mod exports{
    pub use super::index;
}