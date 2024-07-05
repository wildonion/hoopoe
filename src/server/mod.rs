


use crate::*;
use models::event::EventQuery;
use salvo::{cors::Cors, http::method::Method, websocket::WebSocket};
use context::AppContext;
use tokio::sync::{mpsc::Receiver, Mutex};


pub trait HoopoeService<G: Send + Sync>{
    type Service: Send + Sync; // binding for GAT is supported now we can have async method
    async fn run(&mut self);
}

pub struct HoopoeServer{
    pub service: Option<Service>,
    pub addr: String,
    pub app_ctx: Option<AppContext>,
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
            .hoop(Logger::new()) // register middlewares using hoop() method
            .hoop(affix::inject(app_ctx))
            .hoop(cors);

        // adding swagger ui route
        let doc = OpenApi::new("Hoopoe Api", "0.1.0").merge_router(&router);
        let router = router
            .push(doc.into_router("/api-doc/openapi.json"))
            .push(SwaggerUi::new("/api-doc/openapi.json").into_router("swagger"));
        router
    }

    fn internalBuildRouter(app_ctx: Option<AppContext>) -> Router{
        let cors = Cors::new()
            .allow_origin("*")
            .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PATCH, Method::PUT])
            .into_handler();
        let router = Router::new()
            .push(routers::register_app_controllers())
            .hoop(Logger::new()) // register middlewares using hoop() method
            .hoop(affix::inject(app_ctx))
            .hoop(cors);


        // adding swagger ui route
        let doc = OpenApi::new("Hoopoe Api", "0.1.0").merge_router(&router);
        let router = router
            .push(doc.into_router("/api-doc/openapi.json"))
            .push(SwaggerUi::new("/api-doc/openapi.json").into_router("swagger"));
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

}