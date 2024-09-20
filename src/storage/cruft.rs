


// https://medium.com/@disserman/working-with-data-storages-in-rust-a1428fd9ba2c

use std::{sync::atomic::AtomicU8, time::{SystemTime, UNIX_EPOCH}};
use tokio::task::JoinHandle;
use actix::prelude::*;
use uuid::Uuid;
use crate::*;


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Timestamp(std::time::Duration);

impl Timestamp{
    pub fn now() -> Self{
        Self(SystemTime::now().duration_since(UNIX_EPOCH).unwrap())
    }

    pub fn before24h(self) -> Self{
        Self(Self::now().0 - std::time::Duration::from_secs(86400))
    }

    pub fn as_micros(self) -> u128{
        self.0.as_micros()
    }

    pub fn as_nanos(self) -> u128{
        self.0.as_nanos()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Event{
    pub id: Uuid,
    pub data: String, // notif data event
    pub time: Timestamp
}

impl Event{
    
    pub fn new<S: std::fmt::Display>(data: S) -> Self{
        Self { id: Uuid::new_v4(), data: data.to_string(), time: Timestamp::now() }
    }
}

// thread safe generic buffer, wrapped in arc and async mutex
// use none async mutex when you're not in async methods cause
// async mutex is slow!
struct Buffer<T: Clone>{
    pub atomic_data: std::sync::Arc<std::sync::Mutex<Vec<T>>>, // sendable and syncable data to be mutated between threads
    pub e_size: usize // limit the buffer of events
}

impl<T: Clone> Buffer<T>{
    pub fn new(buf_size: usize) -> Self{
        Self { atomic_data: std::sync::Arc::new(
            std::sync::Mutex::new(
                // <_>::default()
                // Vec impls Default trait use the default type instead of 
                // Vec<T>, we can't fill the Vec<T> cause we don't know what type is T
                Default::default() 
            )
        ), e_size: buf_size }
    }
    pub fn push(&self, value: T) -> Result<Self, &str>{ // it's ok to return a pointer to str since we're using the lifetime of self which is valid as long as the instance is valid
        let get_buf = self.atomic_data.clone();
        let mut buf = get_buf.lock().unwrap();
        if buf.len() > self.e_size{
            Err("maximum buffer size is reached!")
        } else{
            (*buf).push(value); // mutating the buffer by deref it cause it's behind a mutex reference, same address new content
            Ok(Self { atomic_data: self.atomic_data.clone(), e_size: self.e_size })
        }
    }

    pub fn is_empty(&self) -> bool{
        let buf = self.atomic_data.lock().unwrap();
        if buf.is_empty(){
            true
        } else{
            false
        }
    }

    pub fn take(&self) -> Vec<T>{
        let get_buf = self.atomic_data.lock().unwrap();
        get_buf.to_vec()
    }

}


/* 
    traits with async methods can't be object safe trait and Boxed with Box<dyn
    we can't use the builtin async method instead we should either use the async_trait 
    or remove the async keywords. 
    async_trait simply convert the return type of the method into pinned boxed future
    so the std::io::Result<()> would be:
    pin::Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + Sync + 'static>>
*/
#[async_trait]
pub trait StorageEngine: Send + Sync + 'static{ // thread safe trait cause it's bounded to Send Sync
    async fn init(&self) -> std::io::Result<()>;
    async fn load_events(&self, _from: Timestamp, _to: Timestamp) -> std::io::Result<Vec<Event>>;
    async fn save_events(&self, events: Vec<Event>) -> std::io::Result<()>;
}

#[async_trait]
impl StorageEngine for (){
    
    async fn init(&self) -> std::io::Result<()>{
        todo!()
    }

    async fn load_events(&self, _from: Timestamp, _to: Timestamp) -> std::io::Result<Vec<Event>>{
        todo!()
    }

    async fn save_events(&self, events: Vec<Event>) -> std::io::Result<()>{
        todo!()
    }

}

#[async_trait]
impl StorageEngine for Database{
    
    async fn init(&self) -> std::io::Result<()>{
        todo!()
    }

    async fn load_events(&self, _from: Timestamp, _to: Timestamp) -> std::io::Result<Vec<Event>>{
        todo!()
    }

    async fn save_events(&self, events: Vec<Event>) -> std::io::Result<()>{
        todo!()
    }
}

/* 
    making such types like engine field with no generics solves lots of problems, 
    e.g. a type instance can be put into a OnceCell global variable and used in 
    all methods of a micro-service, you don’t need to deal with generics when put 
    such instances into web server framework context and so on. When using generics, 
    sooner or later you are forced to deal with a kind of dynamic dispatching
     
    Database contains the state, event buffer, lock and the background worker fields
        - the engine itself
        - event buffer is the buffer of all current events 
        - lock is used when the engine is busy which is either the worker is working on buffer or there are unprocessed events inside it
        - background worker thread is used to execute tasks related to events constantly in the background, it can be tokio::spawn instead of storing a separate worker as feild
    typically any engine must contains a background worker thread to execute its task 
    in the background thread and a mutex or lock to lock the engine when its busy 
    working with executing tasks.
*/
pub struct Database{
    pub id: Uuid,
    pub state: AtomicU8,
    pub engine: Box<dyn StorageEngine>, // Arc or Box, traits as objects must be boxed or arced for dynamic dispatching
    pub event_buf: Buffer<Event>, // thread safe buffer of events
    pub sender: tokio::sync::mpsc::Sender<String>,
    pub background_worker_thread: std::sync::Mutex<tokio::task::JoinHandle<()>>, // an empty joinhandle which is a background worker thread to execute computation in the background
    pub lock: tokio::sync::Mutex<()> // an async lock to tell the user that the Storage1 is busy and he should stop accepting new events
}

/* tight coupled structs: use trait interfaces to extend them
    Buffer<T>, Timestamp, Event, Engine, Channel
*/
struct Channel<T: Send + Sync + 'static>{
    pub sender: tokio::sync::mpsc::Sender<T>,
    pub receiver: tokio::sync::mpsc::Receiver<T>
}

impl Database{

    pub fn new(tx: tokio::sync::mpsc::Sender<String>) -> Self{

        // since the engine must be a Database instance we're breacking the cycle 
        // of it using Box cause we can't store self ref types in struct 
        Self { 
            id: Uuid::new_v4(), 
            state: AtomicU8::new(0), 
            engine: Box::new(()), 
            event_buf: Buffer::new(1024),
            sender: tx.clone(), 
            background_worker_thread: std::sync::Mutex::new(tokio::spawn(async move{})), 
            lock: <_>::default()
        }
    }

    pub async fn check_health(self){
        
        // interval must be mutate to update its next interval time during the tick process
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

        /* 
            we're spawning the ticking process in the background loop, enables the app 
            to run the code inside the loop every 5 seconds.
            since self is not behind a 
            moving the pointer of state into the tokio scope is not possible 
            since the spawn scope is in the background and lives as long as 
            there is no terminate condition but the pointer to the state 
            is valid as long as the check_health() method is valid and is 
            being executed, in here since self is not behind a reference so 
            moving it into the spawn scope is ok
        */
        tokio::spawn(async move{
            let get_state = self.state.load(Ordering::Relaxed);
            loop{
                let tick = interval.tick().await;
                if get_state == 0{
                    log::info!("engine is alive!");
                } else{
                    log::error!("engine is dead!");
                    break;
                }
            }
        });

    }

    pub async fn execute_me<O, F: std::future::Future<Output = O>>(&self, j: F)
        where F: Send + Sync + 'static, O: Send + Sync + 'static{   
            
        /* 
            we're defining the behavior of an Interval when it misses a tick.
            sometimes an interval may misses a tick during executing of another
            task like if a task takes 3 ms to gets executed and the interval 
            ticks every 2 ms the interval misses a tick in between.
            generally, a tick is missed if too much time is spent without 
            calling Interval::tick.
            the following locking process and saving events might take more 
            than 1 seconds of the interval tick so based on the strategy 
            we're using this gets skipped.
        */
        let mut int = tokio::time::interval(tokio::time::Duration::from_secs(1));
        int.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // no need to run the loop inside a tokio::spawn since the whole function
        // will be executed in there later, in here we're handling 
        loop{
            int.tick().await;
            let acquire_lock = self.lock.lock().await;
            let get_events = &self.event_buf;
            let events = get_events.take();
            self.engine.save_events(events);
            // if don't want to miss a tick we can tick here
            // ...
        }

    }

    pub async fn try_connect(self, timeout: std::time::Duration) -> Option<Self>{

        let state = self.state.load(Ordering::Relaxed);
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(1204);
        
        let now = std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let timeout_later = now + timeout;

        while now < timeout_later{ // try for timeout_later seconds until we get a new instance of the Database
            // if the engine is not halted at time of iterating then return a new instance
            // otherwise iterate one more time until the state become online to return 
            // a new one
            if state == 0{
                return Some(Self::new(tx.clone()));
            }
        }

        // if we couldn't get a new instance then we should drop the self
        // drop the instance since the state didn't get updated during 
        // the timout duration, it means the engine is in a halted mode
        // even after timeout tries!
        drop(self);
        return None;

    }

    // this function may gets called during the engine lifetime 
    // to update its status 
    pub fn halt_engine(&mut self){
        let get_state = self.state.get_mut();
        *get_state = 1;
    }

    pub fn run(&'static self){ // self must live long as static in order it can be moved into the tokio spawn scope
        let mut worker = self.background_worker_thread.lock().unwrap();
        let fut = async move{};
        (*worker) = tokio::spawn(
            {
                self.execute_me(fut)
            }
        );
    }

}

// a global and thread safe (Send + Sync + 'static) 
// Database which can be shared between threads safely
pub static ENGINE: Lazy<std::sync::Arc<tokio::sync::Mutex<Database>>> = 
    Lazy::new(||{
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(1024);
        std::sync::Arc::new(
            tokio::sync::Mutex::new(
                Database{ 
                    id: Uuid::new_v4(), 
                    state: AtomicU8::new(0), 
                    engine: Box::new(()), 
                    event_buf: Buffer::new(1024),
                    sender: tx.clone(), 
                    background_worker_thread: std::sync::Mutex::new(tokio::spawn(async move{})), 
                    lock: <_>::default()
                }
            )
        )
});

/* 
    once the instance of Database gets dropped, the background worker
    thread must also gets dropped and cancel the current future inside of it
    it's notable that awaiting a cancelled task might complete as usual if the task was already completed at the time 
    it was cancelled, but most likely it will fail with a cancelled JoinError.
    be aware that tasks spawned using spawn_blocking cannot be aborted because they are not async. 
    if you call abort on a spawn_blocking task, then this will not have any effect, and the task will 
    continue running normally, the exception is if the task has not started running yet; in that case, 
    calling abort may prevent the task from starting.
*/
impl Drop for Database{
    fn drop(&mut self) {
        // use std::sync::Mutex instead of tokio::sync::Mutex which is async 
        // since the drop() method is not async in here!
        if let Ok(fut) = self.background_worker_thread.lock(){
            fut.abort(); // abort the future, if it's already completed at the time of aborting then it doesn't work!
        }
    }
}

impl Actor for Database{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("the database actor has started");
    }

}