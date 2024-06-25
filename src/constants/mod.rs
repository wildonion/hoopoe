


use std::sync::atomic::AtomicU8;
use tokio::time::Duration;
use crate::*;
pub mod queries;

pub const APP_NAME: &str = "Hoopoe";
pub const APP_DOMAIN: &str = "hoopoe.app";


/* --------------------
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