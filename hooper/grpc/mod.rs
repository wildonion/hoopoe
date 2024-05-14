



pub mod event;


#[macro_export]
macro_rules! bootstrap_grpc {
    (
        // ...
    ) => {
        {
            let addr = format!("{}:{}", 
                std::env::var("HOST").unwrap(), 
                std::env::var("GRPC_PORT").unwrap().parse::<u16>().unwrap()
            ).parse::<SocketAddr>().unwrap();
            
            EventServer::start(addr).await;

            Ok(())
        }        
    };
}