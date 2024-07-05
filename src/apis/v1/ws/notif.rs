


use context::AppContext;
use futures::{FutureExt, SinkExt};
use models::event::EventQuery;
use rand_chacha::rand_core::le;
use salvo::websocket::{Message, WebSocket, WebSocketUpgrade};
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
    addr: localhost:2344/v1/stream/notif/?owner=100
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

    // extracting query params
    let notif_query: EventQuery = req.parse_queries().unwrap();
    
    // it's better to lock on every mutexed data inside a lightweight thread of execution
    // in tokio thread, locking must be in a none blocking manner hence to get any data
    // from the mutex us we must use a channel to send data to it and receive it in ouside
    // of the thread using the receiver.
    let notif_broker_receiver = notif_broker_channel.receiver.clone();
    
    WebSocketUpgrade::new()
        .upgrade(req, res, |ws| async move{
            tokio::spawn(async move{
                ws_handler(ws, notif_broker_receiver, notif_query).await;
            });
        })
        .await


}

// mutating an Arced type requires the type to behind a Mutex
// since Arc is an atomic reference used to move type between 
// threads hence mutating it requires an atomic syncing tools 
// like Mutex or RwLock
async fn ws_handler(ws: WebSocket, notif_receiver: Arc<Mutex<Receiver<String>>>, notif_query: EventQuery){
    
    // can't move pointers which is not live long enough into 
    // the tokio spawn scope since the type must live static 
    // or have longer lifetime than the tokio spawn scope or 
    // move it completely into the scope.
    tokio::spawn(
        {
            async move{ // ws and notif_receiver have been moved into the tokio spawn scope
               
                let mut locked_notif_broker_receiver = notif_receiver.lock().await;
                
                /* -------------------------
                    in actix web socket each peer session is an actor which has
                    its own id and actor address, message from server actor can 
                    be sent easily to each peer session actor using actor message
                    pssing feature through the jobq based channel like mpsc, generally 
                    in building ws application the message must be sent to each peer
                    socket in a room through the mpsc channel from there i'll be sent 
                    through the socket back to the client address in that room so 
                    the server actor or object must contain all the peer socket in
                    a room to communicate with them easily, typically this can be 
                    achieved easily with actors.
                */
                // splitting the ws into a sender sink of ws messages and streamer of ws receiver 
                let (user_ws_tx, mut user_ws_rx) = ws.split();

                // we're using an unbounded channel to handle buffering, backpressuring and flushing of messages to the websocket
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                
                /* -------------------------
                    wrapper around tokio::sync::mpsc::UnboundedReceiver that implements Stream
                    which is a collection of future objects, making an stream of future objects 
                    from the unbounded receiver, the UnboundedReceiverStream::new(rx) converts 
                    the rx receiver into a stream that implements the Stream trait. This allows 
                    it to be used with asynchronous stream operations, making it easier to work 
                    with in the context of asynchronous programming. it returns the wrapped rx
                    means everything that rx receives the wrapped_rx will receive too.
                */
                let wrapped_rx = UnboundedReceiverStream::new(rx); 
                
                // spawning async task of forwarding message to websocket
                tokio::spawn(async move{
                    
                    /* -------------------------
                        the rx stream (which receives messages from the unbounded channel) is forwarded 
                        to the user_ws_tx websocket sender, the forward method is used to send all items 
                        from the stream to the sink (the websocket sender in this case), it returns a 
                        future that completes when the stream is exhausted, doing so simply sends all 
                        messages coming from the unbounded channel to the websocket sender which enables
                        us to stream over user_ws_rx to receive messages to unbounded tx by the user.
                        all user messages will be sent to the unbounded channel instead of sending them
                        directly to the websocket channel doing so handles the flexibility of sending 
                        user messages by using a middleware channel which allows us to handle messages 
                        properly even if the websocket channel is not available yet!

                        user sends ws message ---tx---> unbounded channel 
                        unbounded channel <---wrapped_rx--- receives user message
                        wrapped_rx ---forward(user_ws_tx)---> websocket channel
                        websocket channel <---user_ws_rx--- receives user message

                        Why Pass the websocket Sender into the mpsc receiver streamer?

                            Decoupling of message production and consumption: 
                                The use of an unbounded channel allows user messages to be produced 
                                (sent into the tx transmitter) independently of their consumption using
                                the wrapped_tx (forwarded to the websocket). This decoupling can improve 
                                the responsiveness and flexibility of the system, as message production 
                                does not have to wait for the websocket to be ready to send the messages.
                                cause we're actually sending user message to an unbounded channel even
                                if the websocket is not ready!

                            Buffering and backpressure handling: 
                                The unbounded channel provides a buffering mechanism that can help 
                                handle situations where user messages are produced faster than they can 
                                be sent over the websocket, while it's unbounded and doesn't apply 
                                backpressure, it ensures that the producer or user_ws_tx won't be 
                                blocked by a slow consumer or user_ws_rx.

                            Stream and Sink integration: 
                                By converting the receiver into a stream, it can be easily integrated 
                                with the websocket sink using the forward method. This allows for seamless 
                                forwarding of messages from the channel to the websocket.

                        this setup provides a robust and flexible way to handle websocket communication 
                        in an asynchronous context, ensuring that messages can be buffered and forwarded 
                        efficiently from the channel to the websocket connection.

                    */
                    wrapped_rx.forward(user_ws_tx).map(|res|{ // forward all items received from the rx to the websocket sender
                        if let Err(e) = res{
                            log::error!("websocket send error: {:?}", e.to_string());
                        }
                    });

                });


                while let Some(data) = user_ws_rx.next().await{

                }
                
                // cache the user socket id and rooms on redis 
                // ...
                
                
                // receiving notif data from the notif broker channel as they're getting consumed by the consumer
                while let Some(notif) = locked_notif_broker_receiver.recv().await{
                    
                    // decode the notif json string into the NotifData struct
                    let notif_data = serde_json::from_str::<NotifData>(&notif).unwrap();

                    // send notif related to the passed in owner as they're coming from the channel
                    if notif_query.owner.is_some(){
                        
                        let owner = notif_query.owner.as_ref().unwrap();
                        if &notif_data.receiver_info == owner{

                            // send notif_data for this owner through ws to client 
                            // ...

                        }
                        
                    }
                }
            }
        }
    );

}

pub fn register_controller() -> Router{

    Router::with_path("/v1/stream/notif/")
        .oapi_tag("Notification") // this api must be passed as middleware to update the state of depot before accessing the check_health route api
        .push(
            Router::with_path("consume")
                .goal(consume)
        )

}