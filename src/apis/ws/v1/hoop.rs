

use crate::*;
use actix_web::{cookie::Cookie, http::StatusCode};
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
    app_state: web::Data<AppState>,
) -> HoopoeHttpResponse{ 

    // we can convert payload stream into the ws messages using ws msg handlers
    let mut buffer = vec![];
    while let Some(chunk) = stream.next().await{ // reading future objects from the streamer is a mutable process
        let bytes = chunk.unwrap();
        buffer.extend_from_slice(&bytes)
    }    

    todo!()

}


pub mod exports{
    pub use super::index;
}