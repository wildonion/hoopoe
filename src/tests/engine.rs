

use std::sync::atomic::AtomicU8;
use tokio::task::JoinHandle;

use crate::*;

/* tight coupled structs: use trait interfaces to extend them
    Buffer<T>, Timestamp, Event, Engine, Channel
*/
struct Channel<T: Send + Sync + 'static>{
    pub sender: tokio::sync::mpsc::Sender<T>,
    pub receiver: tokio::sync::mpsc::Receiver<T>
}


// generic buffer, safe to be shared between threads
struct Buffer<T: Send + Sync + 'static>{
    pub events: std::sync::Arc<tokio::sync::Mutex<Vec<T>>>,
    pub size: usize
}

struct Timestamp(pub std::time::Duration);

// the event data 
struct Event{
    pub time: Timestamp,
    pub data: String,
    pub task: std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + Sync + 'static>>, // pointer of self ref types must be pinned, the pointer of future objects are Box
    pub id: Uuid
}


/* 
    Engine contains the state, event buffer, lock and the background worker fields
        - event buffer is the buffer of all current events 
        - lock is used when the engine is busy which is either the worker is working on buffer or there are unprocessed events inside it
        - background worker thread is used to execute tasks related to events constantly in the background, it can be tokio::spawn instead of storing a separate worker as feild
    typically any engine must contains a background worker thread to execute its task 
    in the background thread and a mutex or lock to lock the engine when its busy 
    working with executing tasks.
*/
struct Engine{
    pub buffer: Buffer<Event>, // buffer of events
    pub state: AtomicU8, // the thread safe state of the current engine
    pub lock: tokio::sync::Mutex<()>, // the engine is busy or got locked
    pub worker: tokio::sync::Mutex<tokio::task::JoinHandle<()>> // the background worker thread which is safe to be mutated in other threads
}

// a global and thread safe (Send + Sync + 'static) 
// Engine which can be shared between threads safely
pub static ENGINE: Lazy<std::sync::Arc<tokio::sync::Mutex<Engine>>> = 
    Lazy::new(||{
        std::sync::Arc::new(
            tokio::sync::Mutex::new(
                Engine{
                    buffer: todo!(),
                    state: todo!(),
                    lock: todo!(),
                    worker: todo!(),
                }
            )
        )
});