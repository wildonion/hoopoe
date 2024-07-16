


use std::sync::atomic::{AtomicU8, AtomicUsize};
use salvo::websocket::Message;
use tokio::{sync::mpsc, time::Duration};
use crate::*;
pub mod queries;


/* -------------------------
    this is an static mutable vector of lock ids which is safe to be shared
    and sent across threads and scopes, we'll be using this to handle the process
    of mingint a product atomically, always use lazy arc mutex in a static 
    context since having arc mutex as static type requires the value to be 
    static and since arc mutex are not static solution to this must be 
    lazy arc mutex.
    can't have an static atomic cause to change the atomic we need to deref it 
    as mutable and can't have mutable static instead we should use static lazy 
    arc mutex data which is the safest and best way to mutate an static type 
    inside other threads and scopes make sure you're using tokio spawn to execute 
    locking task of the arc mutex data inside the lightweight thread.
*/
pub static PURCHASE_DEMO_LOCK_MUTEX: Lazy<std::sync::Arc<tokio::sync::Mutex<Vec<i32>>>> = 
    Lazy::new(||{
        std::sync::Arc::new(
            tokio::sync::Mutex::new(
                vec![]
            )
        )
    });


/* ------------------------- 
    followings are an in-memory websocket server states for online rooms and users
    this would be much easier by storing the ws server in an actor object cause
    each actor state is isolated which allows us to initialize it once in the whole 
    app and pass it between routers, apis and scopes.
    each peer session would have a unique actor address which is atomic and safe 
    to mutate it, the message passing between each peer and the server would be 
    possible using actor message passing logic which is upon mpsc jobq channel
    and will be automatically handled and scaled by the actor object itself.
    we can initialize the object once and simply pass the instance to the app context 
    so we could use it between each api, this technique is used when we have http 
    server and want to pass an app state which contains the whole setup between 
    routers if there wasn't such service then creating each field of the following 
    struct as an static type would be the best option, for now we should create an
    static type for this since app context derives Clone which is not implemented 
    for the RwLock and UnboundedSender subtypes.
    if there was an actor instead of storing users in RwLock we would have stored
    users in actor state hence mutating it would be safe as long as the actor state
    is isolated cause each actor is a thread safe object by default.
    also there would be no need to store the value of the map inside the online_users 
    field which is an unbounded sender used to send user messages to websocket channel
    cause by having actor this would be done using the actor message passing logic
    which allows us to send message to an actor mailbox using its own sender.

    static type: static Lazy Arc RwLock Data{}
    AppContext : contains whole app setup then share the instance between threads and apis
    RwLock     : more reads and less writes (contains the read and write locks separately)
    Mutex      : more writes and less reads (both read and write in one lock)

    use read for sending message to users 
    use write to insert new users into the map
*/
pub type WsUsers = RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, salvo::Error>>>>; // websocket users: a thread safe mapping between user thread safe id and an unbounded sender for each one  
pub type WsRooms = RwLock<HashMap<String, HashSet<usize>>>; // a mapping between room names and the users
pub static ONLINE_USERS: Lazy<WsUsers> = Lazy::new(||{ WsUsers::default() }); // all online users
pub static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1); // used as a thread safe user id, this would be simply a usize in actor state cause actors are an isolated and thread safe objects 
pub static WS_ROOMS: Lazy<WsRooms> = Lazy::new(||{ WsRooms::default() }); // thread safe map between event room and all user ids in that room 

pub const APP_NAME: &str = "Hoopoe";
pub const APP_DOMAIN: &str = "hoopoe.app";

pub static HOME_HTML: &str = r#"

    <h1>Welcome Home</h1>

"#;


// every code has 2 bytes long since the code value is larger than 
// 255 we've used u16 which is 2 chars in hex
pub static SERVER_IO_ERROR_CODE: &u16 = &0xFFFE;
pub static STORAGE_IO_ERROR_CODE: &u16 = &0xFFFF;
pub static CHRONO_ERROR_CODE: &u16 = &0xFFAE;
pub static CODEC_ERROR_CODE: &u16 = &0xFFFB;
pub static MAILBOX_CHANNEL_ERROR_CODE: &u16 = &0xFFFD;
pub static HTTP_RESPONSE_ERROR_CODE: &u16 = &0xFFFA;
pub static CRYPTER_THEMIS_ERROR_CODE: &u16 = &0xFFAB;
pub static THIRDPARTYAPI_ERROR_CODE: &u16 = &0xFFFC;
pub static FILE_ERROR_CODE: &u16 = &0xFFCE;
pub const PING_INTERVAL: Duration = Duration::from_secs(10);
pub const LOGS_FOLDER_ERROR_KIND: &str = "logs/error-kind";