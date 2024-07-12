


use actix::Actor;
use constants::NEXT_USER_ID;
use context::AppContext;
use futures::{FutureExt, SinkExt};
use models::event::EventQuery;
use rand_chacha::rand_core::le;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
use server::HoopoeWsServerActor;
use tokio::sync::{Mutex, mpsc::Receiver};
use tokio_stream::wrappers::UnboundedReceiverStream;
use std::sync::Arc;
use crate::*;


/* -----------
    client app gets connected to this route and started consuming 
    the all notifications coming from the RMQ exchange in realtime
    the notif broker however stores data on redis and db allows 
    the client to fetch notifs for an owner in a short polling manner
    this way is used to fetch all notifs for an owern in realtime as
    they're receiving by the RMQ consumer.
    addr: localhost:2344/v1/stream/notif/consume/?owner=100&room=notif_room
    owner is the notification owner which must be equal to the `receiver_info`
    field inside the notif_data instance received by the consumer. 
*/
#[handler]
async fn consume(
    req: &mut Request,
    res: &mut Response, 
    depot: &mut Depot,
    ctrl: &mut FlowCtrl
) -> Result<(), StatusError>{

    let app_ctx = depot.obtain::<Option<AppContext>>().unwrap(); // extracting shared app context
    let actors = app_ctx.as_ref().unwrap().actors.clone();
    let channels = app_ctx.as_ref().unwrap().channels.clone();
    let notif_broker_channel = &channels.notif_broker;
    let zerlog_producer_actor = actors.clone().unwrap().broker_actors.zerlog_actor;

    // extracting query params
    let notif_query: EventQuery = req.parse_queries().unwrap();
    
    // it's better to lock on every mutexed data inside a lightweight thread of execution
    // in tokio thread, locking must be in a none blocking manner hence to get any data
    // from the mutex us we must use a channel to send data to it and receive it in ouside
    // of the thread using the receiver.
    let notif_broker_receiver = notif_broker_channel.receiver.clone();
    
    WebSocketUpgrade::new()
        .upgrade(req, res, |ws| async move{
            // spawn the async task of handling websocket session inside a lightweight thread
            tokio::spawn(async move{
                HoopoeWsServerActor::session_handler(ws, notif_query, notif_broker_receiver, zerlog_producer_actor).await;
            });
        })
        .await


}



pub fn register_controller() -> Router{

    Router::with_path("/v1/stream/notif/")
        .oapi_tag("Notification") // this api must be passed as middleware to update the state of depot before accessing the check_health route api
        .push(
            Router::with_path("consume")
                .goal(consume)
        )

}