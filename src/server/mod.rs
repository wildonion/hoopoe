


use crate::*;
use actix::{Actor, Addr, AsyncContext, Context};
use actix_web::middleware::Compress;
use constants::{NEXT_USER_ID, ONLINE_USERS, WS_ROOMS};
use futures::{lock, FutureExt};
use models::event::EventQuery;
use salvo::{cors::Cors, http::method::Method, websocket::{Message, WebSocket}};
use context::AppContext;
use tokio::sync::{mpsc::Receiver, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use wallexerr::misc::Wallet;
use workers::zerlog::ZerLogProducerActor;
use salvo::conn::rustls::{Keycert, RustlsConfig};


pub trait HoopoeService<G: Send + Sync>{ // supports polymorphism
    type Service: Send + Sync; // binding for GAT is supported now we can have async method
    async fn run(&mut self);
    async fn backup(&mut self, instance: G);
}

pub struct HoopoeServer{
    pub service: Option<Service>,
    pub addr: String,
    pub app_ctx: Option<AppContext>, // the whole app context: db, actors and global types
    pub ssl_domain: Option<String>
}

impl HoopoeServer{

    pub fn buildRouter(&self) -> Router{
        let cors = Cors::new()
            .allow_origin("*")
            .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PATCH, Method::PUT])
            .into_handler();
        let app_ctx = self.app_ctx.clone();
        let router = Router::new()
            .push(routers::register_app_controllers())
            .hoop(Compression::new().enable_brotli(CompressionLevel::Fastest))
            .hoop(Logger::new()) // register middlewares using hoop() method
            .hoop(affix::inject(app_ctx))
            .hoop(cors);

        // adding api ui routes
        let doc = OpenApi::new("Hoopoe Api", "0.1.0").merge_router(&router);
        let router = router
            .push(doc.into_router("/api-doc/openapi.json"))
            .unshift(
                Scalar::new("/api-doc/openapi.json")
                    .title("Hoopoe - Scalar")
                    .into_router("scalar"),
            )
            .unshift(
                SwaggerUi::new("/api-doc/openapi.json")
                    .title("Hoopoe - Swagger UI")
                    .into_router("swagger")
            )
            .unshift(
                RapiDoc::new("/api-doc/openapi.json")
                    .title("Hoopoe - RapiDoc")
                    .into_router("rapidoc"),
            )
            .unshift(
                ReDoc::new("/api-doc/openapi.json")
                    .title("Hoopoe - ReDoc")
                    .into_router("redoc"),
            );
        router
    }

    fn internalBuildRouter(app_ctx: Option<AppContext>) -> Router{
        let cors = Cors::new()
            .allow_origin("*")
            .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PATCH, Method::PUT])
            .into_handler();
        let router = Router::new()
            .push(routers::register_app_controllers())
            .hoop(Compression::new().enable_brotli(CompressionLevel::Fastest))
            .hoop(Logger::new()) // register middlewares using hoop() method
            .hoop(affix::inject(app_ctx))
            .hoop(cors);

        // adding swagger ui route
        let doc = OpenApi::new("Hoopoe Api", "0.1.0").merge_router(&router);
        let router = router
            .push(doc.into_router("/api-doc/openapi.json"))
            .unshift(
                Scalar::new("/api-doc/openapi.json")
                    .title("Hoopoe - Scalar")
                    .into_router("scalar"),
            )
            .unshift(
                SwaggerUi::new("/api-doc/openapi.json")
                    .title("Hoopoe - Swagger UI")
                    .into_router("swagger")
            )
            .unshift(
                RapiDoc::new("/api-doc/openapi.json")
                    .title("Hoopoe - RapiDoc")
                    .into_router("rapidoc"),
            )
            .unshift(
                ReDoc::new("/api-doc/openapi.json")
                    .title("Hoopoe - ReDoc")
                    .into_router("redoc"),
            );
        router
    }

    pub async fn new(ssl_domain: Option<String>) -> Self{
        dotenv::dotenv().expect("expected .env file be there!");
        Self{
            addr: {
                format!("{}:{}",
                    std::env::var("HOST").unwrap_or(String::from("0.0.0.0")),
                    std::env::var("HTTP_PORT").unwrap_or(String::from("2344"))
                )
            },
            app_ctx: None,
            service: None,
            ssl_domain
        }
    }

    pub fn initLog(&self){

        env::set_var("RUST_LOG", "trace");
        env_logger::init();
        
        // logging
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::TRACE) // higher than trace like debug, info, warn
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

    }

    pub async fn injectService<T: Send + Sync, N, G: Send + Sync>(&mut self, service: impl HoopoeService<T>)
        where N: HoopoeService<G, Service: Send + Sync>{ // binding Service GAT to traits

            // service is an static dispatch which is an instance 
            // of any type who implements the HoopoeService trait
            // ...
    }

    pub async fn buildAppContext(&mut self){

        /* -ˋˏ✄┈┈┈┈ initializing app contexts: storages and actor workers
            >_ run actor workers, app_state contains the whole app data 
            which will be used globally during the execution of the app
        */
        self.app_ctx = Some(context::AppContext::init().await);

    }

    pub async fn applyMigrations(&self){

        let app_ctx = &self.app_ctx;
        let db_url = &app_ctx.as_ref().unwrap().config.as_ref().unwrap().vars.DATABASE_URL;

        /* -ˋˏ✄┈┈┈┈ migrating on startup
            >_ ORM checks on its own that the db is up to create the pool connection
            it won't start the app if the db is off, makes sure you've started
            the pg db server
        */
        let args = cli::ServerKind::parse();
        let connection = sea_orm::Database::connect(
            db_url
        ).await.unwrap();
        let fresh = args.fresh;
        if fresh{
            Migrator::fresh(&connection).await.unwrap();
            Migrator::refresh(&connection).await.unwrap();
        } else{
            Migrator::up(&connection, None).await.unwrap();
        }
        
        Migrator::status(&connection).await.unwrap();
    }

    // wrapping around the whole apis and make them as a service
    pub fn buildService(&mut self){
        let apis = self.buildRouter();
        let service = Service::new(apis); // dependencry injection
        self.service = Some(service);
    }

    // don't use &self cause we'll get stuck in the hell of borrowing and ownership 
    // rules inside the method! can't move out of types if it's behind a shared refrence 
    // means we can't use * to deref or use methods that take ownership of type we 
    // can either clone it or use self instead of &self in method params, note that we 
    // can't deref ref if it doesn't impl Copy trait.
    // ---------
    // the server must be started in a loop inside a lightweight thread of execution
    // using tokio::spawn() so spawning the loop to halt the program and making it 
    // constantly running is not correct since the halting part would be inside a
    // separate thread other than the current one because we should halt the current 
    // thread to start the server so spawn the an async task of starting server in
    // tokio threads inside the loop like: loop{ tokio::spawn(async move{ start_server() }); }
    pub async fn run(self){ 
        let Self { service, addr, ssl_domain, app_ctx } = self;
        if ssl_domain.is_some() && !ssl_domain.as_ref().unwrap().is_empty(){
            
            let mut apis = Self::internalBuildRouter(app_ctx);
            let serv = service.unwrap();

            // acme with http3 quinn
            let listener =  TcpListener::new(addr)
                .acme() // acme with let's encrypt
                .add_domain(&ssl_domain.unwrap())
                .http01_challege(&mut apis).quinn("0.0.0.0:443"); // secure the 443 port with ssl
            
            let acceptor = listener.join(TcpListener::new("0.0.0.0:80")).bind().await;
            Server::new(acceptor).serve(serv).await;
            
        } else{
            let acceptor = TcpListener::new(addr).bind().await;
            let server = Server::new(acceptor);
            let serv = service.unwrap();
            server.serve(serv).await; // start server in a loop{} in a tokio lightweight thread of execution
        }
    }

    pub async fn runOverHTTP3(self){
        
        let Self { service, addr, app_ctx, ssl_domain } = self;
        let serv = service.unwrap(); // contains apis 

        // include_bytes!{} macro loads a file in form of utf8 bytes array,
        // since it's a macro thus it checks the paths at compile time!
        let key = include_bytes!("../../infra/certs/http3/hoopoecert.pem").to_vec();
        let cert = include_bytes!("../../infra/certs/http3/hoopoekey.pem").to_vec();
        let config = RustlsConfig::new(Keycert::new().cert(cert.as_slice()).key(key.as_slice()));

        let listener = TcpListener::new(addr).rustls(config.clone());
        let acceptor = QuinnListener::new(config, ("127.0.0.1", 5800))
            .join(listener)
            .bind()
            .await;

        let server = Server::new(acceptor);
        server.serve(serv).await;

    }

}


/* -------------------------
    in actix web socket each peer session is an actor which has
    its own id and actor address, message from server actor can 
    be sent easily to each peer session actor using actor message
    pssing feature through the jobq based channel like mpsc, generally 
    in building ws application the message must be sent to each peer
    socket in a room through the mpsc channel from there i'll be sent 
    through the socket back to the client address in that room so 
    the server actor or object must contain all the peer sockets in
    a room to communicate with them easily, typically this can be 
    achieved easily with actors in such a way to build a new actor
    per each session that gets connected to the ws server, the 
    msg sending part would be easy by using actor msg sending logic.
*/
pub struct HoopoeWsServerActor;

impl Actor for HoopoeWsServerActor{
    
    type Context = Context<Self>;
    
    fn started(&mut self, ctx: &mut Self::Context) {
        // we can execute a closure trait inside the run_interval method, which 
        // contains the actor and the context itself 
        ctx.run_interval(constants::PING_INTERVAL, |actor, ctx|{
            log::info!("hoopoe websocket server is alive");
        });
    }
}


impl HoopoeWsServerActor{

    async fn check_healt(user_id: usize){
        let mut interval = tokio::time::interval(constants::PING_INTERVAL);
        tokio::spawn(async move{
            loop{ 
                interval.tick().await; // tick every 5 second in a loop
                log::info!("websocket session for user#{} is alive", user_id);
            }
        });
    }

    pub async fn send_message_to_ws_users_in_room(my_id: usize, msg: Message, this_room: &str) {
        let this_cloned_msg = msg.clone();
        let msg = if let Ok(s) = this_cloned_msg.to_str() {
            s
        } else {
            return;
        };

        for (&uid, tx) in ONLINE_USERS.read().await.iter(){
            if my_id != uid { // send the message to all users execpt the current one
                let new_msg = format!("<User#{}> in <Room#{}>: {}", my_id, this_room, msg);
                let ws_message = Ok(Message::text(new_msg));
                // sending the notif data to the user tx, the message will be received by the receiver
                // and forwarded to the current_user_ws_tx sender of the unbounded mpsc channel, later we can 
                // stream over current_user_ws_rx and receive message.
                if let Err(e) = tx.send(ws_message){ // catch the error if the channel is closed
                    log::error!("can't send notif message to unbounded channel using tx: {}", e.to_string());
                }

            }
        }
        
    }

    pub async fn user_disconnected(user_id: usize, room: &str){
        // remove user from room and online users, then log the dead session
        let mut users_in_this_room = WS_ROOMS.write().await.get(room).unwrap().clone();
        users_in_this_room.remove(&user_id);
        ONLINE_USERS.write().await.remove(&user_id);
        log::error!("websocket session for user#{} is dead", user_id);
    } 

    // mutating an Arced type requires the type to behind a Mutex
    // since Arc is an atomic reference used to move type between 
    // threads hence mutating it requires an atomic syncing tools 
    // like Mutex or RwLock
    pub async fn session_handler(ws: WebSocket, notif_query: EventQuery,
        notif_receiver: Arc<Mutex<Receiver<String>>>,
        zerlog_producer_actor: Addr<ZerLogProducerActor>){

            // check that the user is already connected or not, 
            // avoid having duplicate socket connection for a same user
            let user_id = notif_query.clone().owner.unwrap().parse::<usize>().unwrap();
            let get_users = ONLINE_USERS.read().await;
            if get_users.contains_key(&user_id){
                log::error!("user #{} is already connected!", user_id);
                return;
            }

            Self::check_healt(user_id).await;

            /* -------------------------
                can't move pointers which is not live long enough into 
                the tokio spawn scope since the type must live static 
                or have longer lifetime than the tokio spawn scope or 
                move it completely into the scope.    
                current_user_id is the old value and NEXT_USER_ID is old_value + 1, in actor context 
                this would be done with actor address, fetch_add() adds to the current value and 
                returns the previous value
            */
            let current_user_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

            // splitting the ws into a sender sink of ws messages and streamer of ws receiver 
            let (current_user_ws_tx, mut current_user_ws_rx) = ws.split();

            // we're using an unbounded channel to handle buffering, backpressuring and flushing of messages to the websocket
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel(); // unbounded_channel is not async cause there is no waiting process for an unbounded channel
            
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
            
            /* -------------------------
                the rx stream (which receives messages from the unbounded channel) is forwarded 
                to the current_user_ws_tx websocket sender, the forward method is used to send all items 
                from the stream to the sink (the websocket sender in this case), it returns a 
                future that completes when the stream is exhausted, doing so simply sends all 
                messages coming from the unbounded channel to the websocket sender which enables
                us to stream over current_user_ws_rx to receive messages to unbounded tx by the user.
                all user messages will be sent to the unbounded channel instead of sending them
                directly to the websocket channel doing so handles the flexibility of sending 
                user messages by using a middleware channel which allows us to handle messages 
                properly even if the websocket channel is not available yet!

                user sends ws message ---tx---> unbounded channel 
                unbounded channel <---wrapped_rx--- receives user message
                wrapped_rx ---forward(current_user_ws_tx)---> websocket channel
                websocket channel <---current_user_ws_rx--- receives user message

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
                        backpressure, it ensures that the producer or current_user_ws_tx won't be 
                        blocked by a slow consumer or current_user_ws_rx.

                    Stream and Sink integration: 
                        By converting the receiver into a stream, it can be easily integrated 
                        with the websocket sink using the forward method. This allows for seamless 
                        forwarding of messages from the channel to the websocket.

                this setup provides a robust and flexible way to handle websocket communication 
                in an asynchronous context, ensuring that messages can be buffered and forwarded 
                efficiently from the channel to the websocket connection.

            */ 
            let cloned_zerlog_producer_actor = zerlog_producer_actor.clone();
            // ------=====------=====------=====------=====------=====
            // ------=====------=====------=====------=====------=====
            /* ------------------------- 
                don't use async move in tokio::spawn() cause the wrapped_rx or rx will 
                be moved into the tokio::spawn() and gets dropped hence we can't send 
                notif or any message from server to tx cause the receiver or rx is 
                already dropped and the channel is closed.
            */
            tokio::spawn(
                {
                    wrapped_rx.forward(current_user_ws_tx).map(|res|{ // forward all items received from the rx to the websocket channel sender
                        if let Err(e) = res{
                            log::error!("receiver stream can't forward messages to websocket sender: {:?}", e.to_string());
                            tokio::spawn( // handling forward error in a separate thread
                                {
                                    async move{
                                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                                        let error_content = &e.to_string();
                                        let error_content = error_content.as_bytes().to_vec();
                                        let mut error_instance = HoopoeErrorResponse::new(
                                            *constants::SERVER_IO_ERROR_CODE, // error code
                                            error_content, // error content
                                            ErrorKind::Server(crate::error::ServerError::Salvo(e)), // error kind
                                            "HoopoeWsServer.session_handler.wrapped_rx.forward", // method
                                            Some(&cloned_zerlog_producer_actor)
                                        ).await;
                                    }
                                }
                            );
                        }
                    })
                }
            );

            // ------=====------=====------=====------=====------=====
            // ------=====------=====------=====------=====------=====
            tokio::spawn(async move {
                
                ONLINE_USERS.write().await.insert(current_user_id, tx.clone());
                
                // specifying the room
                let room = if notif_query.room.is_some(){
                    notif_query.room.clone().unwrap()
                } else{
                    String::from("notifs") // the default room
                };

                // don't block the current thread for mutating room, every mutating 
                // process of thread safe data types must be in a separate thread 
                // or async function, since async function gets handled by the runtime
                // scheduler and executed in a selected thread by the scheduler at runtime
                let cloned_room = room.clone();
                tokio::spawn(async move{
                    // insert the current user into the list of users
                    let mut users = HashSet::new();
                    users.insert(current_user_id);
                    let mut rooms = WS_ROOMS.write().await;
                    // mutating the current rooms with new user
                    (*rooms) // to mutating it safely we should deref it
                        .entry(cloned_room.clone()) // check that the key is in there or not
                        .and_modify(|users|{ // if is there then update users to add a new user id
                            users.insert(current_user_id);
                        })
                        .or_insert(users); // if the key is not there insert a new user ids
                });


                while let Some(result) = current_user_ws_rx.next().await {
                    let msg = match result {
                        Ok(msg) => msg,
                        Err(e) => {
                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                            let error_content = &e.to_string();
                            let error_content = error_content.as_bytes().to_vec();
                            let mut error_instance = HoopoeErrorResponse::new(
                                *constants::SERVER_IO_ERROR_CODE, // error code
                                error_content, // error content
                                ErrorKind::Server(crate::error::ServerError::Salvo(e)), // error kind
                                "HoopoeWsServer.session_handler.WsMessage", // method
                                Some(&zerlog_producer_actor)
                            ).await;
                            break;
                        }
                    };
                    Self::send_message_to_ws_users_in_room(current_user_id, msg, &room).await;
                }

                // when we're not receiving anything means we've disconnected
                Self::user_disconnected(current_user_id, &room).await;
            });


            // ------=====------=====------=====------=====------=====
            // ------=====------=====------=====------=====------=====
            let cloned_notif_receiver = notif_receiver.clone();
            tokio::spawn(async move{

                let mut locked_notif_broker_receiver = cloned_notif_receiver.lock().await;
                while let Some(notif) = locked_notif_broker_receiver.recv().await{

                    log::trace!("[*] received notif inside ws server: {}", notif);
                    
                    // decode the notif json string into the NotifData struct
                    let notif_data = serde_json::from_str::<NotifData>(&notif).unwrap();

                    // send notif related to the passed in owner as they're coming from the channel
                    if notif_query.owner.is_some(){
                        
                        let owner = notif_query.owner.as_ref().unwrap();
                        if &notif_data.receiver_info == owner{ // only send notif data for this owner
                            for (user_id, tx) in ONLINE_USERS.read().await.iter(){
                                if *user_id == current_user_id{ // send notif data back to the user
                                    let ws_message = Ok(Message::text(notif.clone()));
                                    // sending the notif data to the user tx, the message will be received by the receiver
                                    // and forwarded to the current_user_ws_tx sender of the unbounded mpsc channel.
                                    if let Err(e) = tx.send(ws_message){ // catch the error if the channel is closed
                                        log::error!("can't send notif message to unbounded channel: {}", e.to_string());
                                    }
                                }
                            }
                        }

                    }
                }
            });


    }

}