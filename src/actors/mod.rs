



// we have producer and consumer actors per each data streamer 
// we have mutator and accessor actors per each db model
pub mod cqrs;
pub mod consumers;
pub mod producers;
pub mod sse;
pub mod ws;


/* 

⚈ --------- ⚈ --------- ⚈ --------- ⚈ --------- ⚈ --------- ⚈ --------- ⚈ 
        actor worker threadpool implementations from scratch  
⚈ --------- ⚈ --------- ⚈ --------- ⚈ --------- ⚈ --------- ⚈ --------- ⚈ 

    for worker in 0..10{ //// spawning tokio green threads for 10 workers
        tokio::spawn(async move{ //// spawning tokio worker green threadpool to solve async task
            
            //// any heavy logic here that must be shared using tokio channels inside a threadpool  
            //// ...
            
        });
    }

    let mut tokio_async_worker = AsyncWorker::new();
    let mut native_sync_worker = NativeSyncWorker::spawn(n_workers);
    let mut rayon_sync_worker  = RayonSyncWorker::new();
    let (sender, receiver) = std_mpsc::channel::<u8>();
    let cloned_sender = sender.clone();
    
    native_sync_worker.execute(move ||{
        let async_heavy_method = || async move{
            // mining();
            let big_end_bytes = number.to_be_bytes();
            let index = 0;
            let new_chunk = cloned_ops(big_end_bytes[index]);
            cloned_sender.send(new_chunk).unwrap();
        }
        block_on(async_heavy_method());
    });
    
    rayon_sync_worker.spawn(move ||{
        block_on(async_heavy_method()); 
    });

    tokio_async_worker.spawn(async move{
        async_heavy_method().await;
        Ok(())
    })
    tokio_async_worker.execute().await // wait for all the workers of this worker to complete if there were any

    
    let bytes: Vec<u8> = receiver.iter().take(n_workers).collect() // collecting data from all workers
        
*/

use std::thread;
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;
use futures_util::Future;
use is_type::Is;
use uuid::Uuid;
use std::sync::{Arc, Mutex};
use log::info;
use crate::*;


struct ActerMessage{
    pub to: String,
    pub from: String,
    pub body: String,
}

struct Acter{
    // std::sync::Mutex is not Send so we can't move it into tokio spawn
    // we must use tokio Mutex
    pub mailbox: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<ActerMessage>>>,
    pub communicator: tokio::sync::mpsc::Sender<ActerMessage>,
}

impl Acter{

    pub async fn send<R>(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output=R>>>{
        todo!()
    }

    // execute tasks and messages in its threadpool (mainly tokio spawn)
    pub async fn execute(&mut self){
        // PROBLEM: can't move out self because it's behind a mutable pointer
        // some how we should move it to tokio scope without losing ownership
        // passing its ref to tokio scope is not ok since the reference won't 
        // be valid and must be static cause self is only valid inside the method
        // body and by moving the &self.mailbox, the pointer will be in tokio scope 
        // and the `self` itself will be dropped out once the method gets executed
        // so it escapes the method body,
        // SOLUTION: use clone, Box, Arc, (Rc is for single thread)
        // we've used Arc and Mutex to make it sendable, shareable and safe to share
        let arced_mutex_mailbox = self.mailbox.clone();
        
        // don't deref the arced_mutex_mailbox since Clone is not implemented for that
        // and can't be move out of the type since derefing return the owned type it's 
        // kinda like clone the type
        tokio::spawn(async move{
            let mut mailbox = arced_mutex_mailbox.lock().await;
            while let Some(task) = mailbox.recv().await{
                
            }

        });
    }

    pub async fn start(&mut self) -> Self {
        // create mailbox and communicator
        // ...
        todo!()
    }
    
}

struct Scheduler{
    pub queue: Queue,
}

type ErrType = Box<dyn std::error::Error + Send + Sync + 'static>;
pub struct Agent<J, T> where J: FnMut(fn() -> T) -> Result<(), ErrType>{ //// generic `J` is a closure type that accept a function as its argument 
    job: J, //// the job itself
    res: T, //// job response
    task: Task
}
pub type Task = Job; //// the type of the Task is of type Job structure
pub struct Job{ // the job that must be received by the receiver
    pub id: Uuid,
    pub task: Box<dyn FnOnce() + Send + Sync + 'static>, //// the task that can be shared between worker threadpool for solving
} 
pub struct Queue{ // a queue which contains all the incoming jobs from the sender 
    pub tasks: Vec<Task>,   
}
pub struct JobHandler; // a threadpool structure to handle the poped-out job from the queue



pub mod workerthreadpool{

    // --------------- GUIDE TO CREATE A MULTITHREADED WEB SERVER ---------------
    // every worker is a thread with an id along with the thread itself, a threadpool is a vector containing the number 
    // of spawned workers then we'll send the async job to the sender channel by calling the execute method and while we're 
    // streaming over the arced mutexed receiver inside each thread we receives the task in on of those free threads and 
    // finally call the async task, the spawning threads in the background part are done inside the spawn() method also every 
    // worker threadpool needs a channel and multiple spawned threads in the background so we could send the task to the 
    // channel and receives it in one of the free spawned thread so the steps would be:
    // 1 - create channel and spawn threads in the background waiting to receive from the channel by calling new() method
    // 2 - pass the task to spawn() or execute() method then send it to channel
    // 3 - make sure receiver is of type Arc<Mutex<Receiver>>


    pub use super::*;

    // ------------------------------------------------------------
    // ------------ aync worker threadpool using tokio ------------
    // ------------------------------------------------------------
    // async worker pool scheduler using tokio based on mpsc jobq channel protocol
    // this scheduler is used for asynchronous IO by not blocking the thread using tokio green threads
    pub mod _async{

        use super::*;

        pub struct AsyncWorker<E>{
            count: usize, // number of workers
            sender: mpsc::UnboundedSender<Result<(), E>>, // sender async side with no byte limitation
            receiver: mpsc::UnboundedReceiver<Result<(), E>>, // receiver async side with no byte limitation
        }


        impl<E: Send + 'static> AsyncWorker<E>{ // E can be shared between threads

            pub fn new() -> Self{
                let (sender, 
                    receiver) = mpsc::unbounded_channel(); // async mpsc jobq channel channel with no byte limitation to avoid deadlocks and race conditions
                AsyncWorker{
                    count: 0, // will be equaled to the number of workers by solving all the jobs which are comming to the downside of the mpsc jobq channel
                    sender,
                    receiver
                }
            }

            pub fn spawn<T>(&mut self, task: T)
                where 
                    T: Future<Output=Result<(), E>> + Send + 'static, // T can be shared between threads
                    T::Output: Is<Type = Result<(), E>>, // T is a future and now we can access the Output type to make sure that is of type Result<(), E> - T::Output is the GAT of the Future trait
                    {
                        let sender = self.sender.clone();
                        tokio::spawn(async move{ // spawn the task inside tokio green threads
                            let res = task.await;
                            match sender.send(res.into_val()){
                                Ok(()) => (),
                                Err(_) => panic!("Impossible Panic for Sender"),
                            }
                        });
                        self.count += 1;
                    }


            pub async fn execute(mut self) -> Result<(), E>{

                std::mem::drop(self.sender); // make sure that the sender is dead since we want to receive all the messages and avoid deadlocks and race condition
                let mut index = 0;

                loop{ // we can use while let Some() syntax
                    match self.receiver.recv().await{
                        Some(Ok(())) => {
                            assert!(index < self.count);
                        }
                        Some(Err(e)) => {
                            assert!(index < self.count);
                            return Err(e);
                        }
                        None => {
                            assert_eq!(index, self.count);
                            break Ok(()); // return this to the main
                        }
                    }
                    index+=1;
                }

            }

        }


    }


    // ----------------------------------------------------------------------------------
    // ------------ none async worker threadpool using rayon and std::thread ------------
    // ----------------------------------------------------------------------------------
    // a sync task scheduler (worker pool) with mpsc as the jobq channel protocol
    // this scheduler is used for synchronous IO by blocking the thread using rust native std thread - alternative to this is rayon
    pub mod sync{


        use super::*;

        type Job = Box<dyn FnOnce() + Send + 'static>; // a job is of type closure which must be Send and static across all threads inside a Box on the heap


        //// there is no guaranteed order of execution for spawns, given that other threads 
        //// may steal tasks at any time, however, they are generally prioritized in a LIFO order 
        //// on the thread from which they were spawned, other threads always steal from the 
        //// other end of the deque, like FIFO order, the idea is that recent tasks are most 
        //// likely to be fresh in the local CPU's cache, while other threads can steal older stale tasks.
        pub struct RayonSyncWorker{
            count: usize, // number of workers
            sender: mpsc::UnboundedSender<Job>, // sender async side with no byte limitation
            receiver: mpsc::UnboundedReceiver<Job>, // receiver async side with no byte limitation
        }


        impl RayonSyncWorker{

            pub fn new() -> Self{
                let (sender, 
                    receiver) = mpsc::unbounded_channel(); // async mpsc jobq channel channel with no byte limitation to avoid deadlocks and race conditions
                RayonSyncWorker{
                    count: 0, // will be equaled to the number of workers by solving all the jobs which are comming to the downside of the mpsc jobq channel
                    sender,
                    receiver
                }
            }

            pub fn spawn(&mut self, task: Job)
                where 
                    {
                        let sender = self.sender.clone();
                        rayon::spawn(move || { // firing off a task into the rayon threadpool in the 'static or global scope
                            match sender.send(task){
                                Ok(()) => (),
                                Err(_) => panic!("Impossible Panic for Sender"),
                            }
                        });
                        self.count += 1;
                    }


            pub async fn execute(mut self) -> Result<(), Box<dyn std::error::Error + Send +'static>>{

                std::mem::drop(self.sender); // make sure that the sender is dead since we want to receive all the messages and avoid deadlocks and race condition
                let mut index = 0;

                loop{ // we can use while let Some() syntax
                    match self.receiver.recv().await{
                        Some(job) => {
                            job();
                            assert!(index < self.count);
                        },
                        None => {
                            assert_eq!(index, self.count);
                            break Ok(()); // return this to the main
                        }
                    }
                    index+=1;
                }

            }

        }


        //// spawning native threads are too slow since thread handling in rust is depends 
        //// on user base context switching means that based on the load of the IO in the 
        //// app rust might solve the data load inside another cpu core and use multiprocessing approach:
        ////     • https://www.reddit.com/r/rust/comments/az9ogy/help_am_i_doing_something_wrong_or_do_threads/
        ////     • https://www.reddit.com/r/rust/comments/cz4bt8/is_there_a_simple_way_to_create_lightweight/
        struct Worker{
            id: Uuid,
            thread: Option<thread::JoinHandle<()>>, //// thread is of type JoinHandld struct which return nothing or ()
        }

        pub struct NativeSyncWorker {
            workers: Vec<Worker>,
            sender: std_mpsc::Sender<Message>, // all sends will be asynchronous and they never block
        }

        enum Message {
            NewJob(Job),
            Terminate,
        }

        impl NativeSyncWorker{
            
            pub fn spawn(size: usize) -> NativeSyncWorker {
                assert!(size > 0);
                let (sender, receiver) = std_mpsc::channel();
                let receiver = Arc::new(Mutex::new(receiver)); // reading and writing from an IO must be mutable thus the receiver must be inside a Mutex cause data inside Arc can't be borrows as mutable since the receiver read operation is a mutable process
                let mut workers = Vec::with_capacity(size); // capacity is not always equals to the length and the capacity of this vector is same as the maximum size based on the system arch, on 32 bits arch usize is 4 bytes and on 64 bits arch usize is 8 bytes
                for _ in 0..size { // since the receiver is not bounded to trait Clone we must clone it using Arc in each iteration cause we want to share it between multiple threads to get what the sender has sent 
                    workers.push(Worker::new(Uuid::new_v4(), Arc::clone(&receiver)));
                }
                NativeSyncWorker{workers, sender}
            }

            pub fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static { // calling this method means send the incoming task from the process through the mpsc sender to down side of the channel in order to block a free thread using the receiver on locking the mutex
                let job = Box::new(f);
                self.sender.send(Message::NewJob(job)).unwrap(); // by executing the task handler sender will send a job asynchronously and only one receiver at a time can get that job and solve it by locking on the mutex to block the choosen thread since thread safe programming is all about this pattern!
            }
        }

        impl Drop for NativeSyncWorker{ // shutting down all threads on ctrl + c by dropping all of them
            fn drop(&mut self) { // destructor for NativeSyncWorker struct 
                info!("Sending terminate message to all workers.");
                for _ in &self.workers {
                    self.sender.send(Message::Terminate).unwrap();
                }
                info!("Shutting down all workers.");
                for worker in &mut self.workers {
                    info!("Shutting down worker {}", worker.id);
                    if let Some(thread) = worker.thread.take(){ // take() takes the value out of the option, leaving a None in its place
                        thread.join().unwrap(); // joining on thread will block the current thread to get the computation result and stop the thread from being processed in the background
                    }
                }
            }
        }

        impl Worker{
            fn new(id: Uuid, receiver: Arc<Mutex<std_mpsc::Receiver<Message>>>) -> Worker {
                let thread = thread::spawn(move || loop { // spawning a thread inside the new() method and waiting for the receiver until a job becomes available to down side of the channel
                    while let Ok(message) = receiver.lock().unwrap().recv(){ // iterate through the receiver to get all incoming messages - since other thread shouldn't mutate this message while this thread is waiting for the job we must do a locking on the message received from the sender to acquire the mutex by blocking the current thread to avoid being in dead lock, shared state and race condition situation
                        match message {
                            Message::NewJob(job) => {
                                info!("Worker {} got a job; executing.", id);
                                job(); // this might be an async task or job spawned by the tokio spawner in the background
                            }
                            Message::Terminate => {
                                info!("Worker {} was told to terminate.", id);
                                break; // break the loop of this worker so we are not interested in solving task any more 
                            }
                        }
                    }
                });
                Worker {
                    id,
                    thread: Some(thread),
                }
            }
        }


    }
}