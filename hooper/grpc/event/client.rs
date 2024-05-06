


use crate::*;


// use this to send request to other grpc server nodes

pub async fn send_event_request(addr: &str) -> Result<(), Box<dyn std::error::Error>>{

    /* creating a new client service actor to connect to node addr */
    let mut client = EventPubsubServiceClient
        ::connect(addr.to_string())
            .await
            .unwrap();
    
    
    Ok(())
    
}