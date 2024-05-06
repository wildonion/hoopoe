


use std::net::SocketAddr;
use crate::{*, event::event_pubsub_service_server::EventPubsubService};
use tonic::transport::Server as TonicServer;
use wallexerr::misc::Wallet;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use log::*;

// future objects must be pinned to use them as object safe trait along with dynamic dispatching
// response stream contains an item of type Result<EventResponse, Status>, the stream response 
// is a pinned future trait object which sendable, think of response stream as an iterator object 
// contains multiple future objects of pack of u8 bytes that must get streamed over each by calling
// next() method so this is called streaming over future objects.
// if Future<Output = T> is an asynchronous version of T, then Stream<Item = T> is an asynchronous 
// version of Iterator<Item = T>. A stream represents a sequence of value-producing events that occur 
// asynchronously to the caller.
type ResponseStream = std::pin::Pin<Box<dyn Stream<Item = Result<EventResponse, Status>> + Send >>;
type EventResult<T> = Result<TonicResponse<T>, Status>;

// https://github.com/hyperium/tonic/blob/master/examples/src/streaming/server.rs
// https://github.com/hyperium/tonic/tree/master/examples

#[derive(Clone, Debug, Default)]
pub struct EventServer{}

impl EventServer{

    pub async fn start(addr: SocketAddr) 
        -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>{ // handling error dynamically at runtime using dynamic dispatching

        info!("🚀 {} gRPC server has launched in the background from [{}] at {}", 
            APP_NAME, addr, chrono::Local::now().naive_local());

        let hooper = EventServer::default();
        
        TonicServer::builder()
            /* creating a new server service actor from the EchoServer structure which is our rpc server */
            .add_service(EventPubsubServiceServer::new(hooper))
            .serve(addr)
            .await
            .unwrap();
        
        Ok(())
        
    }

    fn restart() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>{

        Ok(())

    }

}

#[tonic::async_trait]
impl EventPubsubService for EventServer{

    type SubscribeStream = ResponseStream;

    
    // client call this to subscribe to a topic sent by server
    async fn subscribe(&self, request: TonicRequest<tonic::Streaming<EventRequest>>) // request streaming
        -> EventResult<Self::SubscribeStream> {

        info!("got an gRPC request at time {:?} | {:?}", 
            chrono::Local::now().naive_local(), request);
        
        let req = request.into_inner();
        
        // get a list of all grpc clients from redis cache
        // then send the received message with its title (channel) to 
        // those ones are subscribing to the topic 
        // ...

        // let request_parts = request.into_parts();
        // let node_rpc_request_body = request_parts.2;
        // let metadata = request_parts.0;
        // let get_headeres = metadata.get("authorization");

        // match get_headeres{

        //     Some(metadata_value) => {

        //         let jwt = format!("Bearer {}", metadata_value.to_str().unwrap());
        //         let node_resp = NodeResponse::default();

        //         let data = node_rpc_request_body.message.clone();
        //         let mut wallet = Wallet::new_ed25519();
        //         let signature = models::node::tcp::Node::ed25519_with_aes_signing(&data, wallet);
        //         println!("base58 ed25519 signature >> {:?}", signature);

        //         Ok(TonicResponse::new(node_resp))


        //     },
        //     None => {

        //         error!("found no jwt in metadata {:?}", chrono::Local::now().naive_local());
        //         let node_resp = NodeResponse::default();
        //         Err(Status::unauthenticated("invalid token"))

        //     }
        // }

        todo!()
        

    }

}