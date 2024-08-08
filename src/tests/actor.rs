


// ------------------------------------------------------------
/* actor worker threadpool implementations from scratch 

    https://ryhl.io/blog/actors-with-tokio/
    https://medium.com/@maturationofthe/leveraging-rusts-tokio-library-for-asynchronous-actor-model-cf6d477afb19
    https://www.reddit.com/r/rust/comments/xec77k/rayon_or_tokio_for_heavy_filesystem_io_workloads/

    runtime takes the async task and put it inside the thread queue 
    then at an appropriate time the scheduler pop the task out of the
    thread queue to execute it and if a thread is free it tries to steal
    the task from other threads, once the task gets executed the runtime 
    waker of future object poll the task result out and other codes can 
    fill the placeholder to use the actual value of the solved task.
    async tasks execution flow in user space level (not os threads):
    >_ code             : developer put a future on the stack
    >_ code             : hey runtime scheduler, developer did an await on the task in a lightweight thread of execution of an actor which has its own stack!
    >_ code             : if you don't mind me asking, can i have the result right now?
    >_ runtime scheduler: allow me to poll the result out of the future stack,... oh no sorry the future is not ready yet! i'll go for other tasks and notify you once it gets ready
    >_ code             : alright sir, i'll keep executing other tasks, perhaps sending the result through an mpsc channel to.... 
    >_ runtime scheduler: hey i've just received the result of the future you wanted earlier from the waker, here is the result.
    >_ code             : as i was saying, ....to outside of this current thread! great other threads and scopes can now use the result as they're receiving the result from the channel that i've sent earlier.           

    the threadpool of tokio is used for light io and rayon is used for cpu heavy io and is global and shared 
    across the program to speedup the execution with jobq mpsc channel like the one in actor object to execute 
    received message or future task/job in its own thread. however in those days that async wasn't there we had 
    been using the queue concepts to queue some tasks then execute them one by one but with the arrival of the 
    async tasks/jobs/processes we can just wait for multiple tasks while we're executing others simultaneously 
    and concurrently, here is the concurrency model for actor worker object (actix): 
        > example of executing async task: 
            - handle each socket in the background asyncly in a tokio spawn like writing bytes to a networking interface like tcp based protocols (ws, http, rmq, redis and rpc)
            - mutating an object in a separate thread by locking on the mutex to acquire the lock to avoid blocking the current thread
            - waiting for some io to get received while other tasks are being executed simultaneously
                ::> in this case the runtime scheduler check for the task to poll out of the stack if 
                    it has completed otherwise the waker fo the future object notify the scheduler as
                    soon as the task gets completed.
        > creating new actor object creates new lightweight thread of execution using tokio spawn to execute asyc tasks inside that 
        > thread management: each actor has its own lighweight thread of execution (an actor object is a lightweight worker thread)
        > async task process execution in the background: use tokio::spawn over tokio runtime scheduler
        > send async task resp across threads: use jobq based channels mailboxes, mpsc or rpc 
        > async task example: atomic syncing for mutating data in threads using arc mutex
        > control over the execution flow: use tokio select to only join that thread which is completed sooner than others
        > distributed clustering talking: use rpc for sending message and calling each other methdos
    conclusion:
        actor is a simple structure that can be used to execute async tasks and jobs in the whole 
        actor system threadpool they can also communicate and send message to each other by using 
        their mailbox, mailbox gives each actor a unique address to send message togehter using 
        channels like mpsc and oneshot in local and rpc and rmq in a remote manner.
        if there are two many variables to be stored on CPU registers they'll be stored on the stack 
        related to the current thread cause each thread gets a seprate stack. 
        tokio runtime will execute all async tasks in its lightweight threads when we put the 
        #[tokio::main] above the main function we can also spawn async task inside a separate 
        lightweight thread manually by using tokio::spawn which contains lightweight thread workers 
        can be used to build actor to execute tasks inside of it hence talk with mpsc channels, we 
        can build multithreaded web servers upon tokio runtime in which each socket will be handled 
        inside tokio spawn threads as well as executing each api 

    ====================================
    HOW 2 USE THESE IMPLEMENTATIONS:
    ====================================
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

use std::{default, thread};
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

#[derive(Clone)]
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
        
        // use custom worker threadpool 
        let this = self.clone();
        let mut pool = workerthreadpool::sync::RayonSyncWorker::new();
        pool.spawn(Box::new(||{
            // don't deref the arced_mutex_mailbox since Clone is not implemented for that
            // and can't be move out of the type since derefing return the owned type it's 
            // kinda like clone the type
            tokio::spawn(async move{
                // PROBLEM: can't move out self because it's behind a mutable pointer
                // some how we should move it to tokio scope without losing ownership
                // passing its ref to tokio scope is not ok since the reference won't 
                // be valid and must be static cause self is only valid inside the method
                // body and by moving the &self.mailbox, the pointer will be in tokio scope 
                // and the `self` itself will be dropped out once the method gets executed
                // so it escapes the method body,
                // SOLUTION: use clone, Box, Arc, (Rc is for single thread)
                // we've used Arc and Mutex to make it sendable, shareable and safe to share
                let arced_mutex_mailbox = this.mailbox.clone();
                let mut mailbox = arced_mutex_mailbox.lock().await;
                while let Some(task) = (*mailbox).recv().await{
                    // ...
                }

            });
        }));

        pool.execute().await; // it will receive the spawned task inside a free thread then it call it

    }

    pub async fn start(&mut self) -> Self {
        // create mailbox and communicator
        // ...
        todo!()
    }
    
}


pub mod workerthreadpool{

    /* 
        --------------- GUIDE TO CREATE A MULTITHREADED WEB SERVER ---------------
        every worker is a thread with an id along with the thread itself, a threadpool is a vector containing the number 
        of spawned workers then we'll send the async job to the sender channel by calling the execute method and while we're 
        streaming over the arced mutexed receiver inside each thread we receives the task in on of those free threads and 
        finally call the async task, the spawning threads in the background part are done inside the spawn() method also every 
        worker threadpool needs a channel and multiple spawned threads in the background so we could send the task to the 
        channel and receives it in one of the free spawned thread so the steps would be:
        1 - create channel and spawn threads in the background waiting to receive from the channel by calling new() method
        2 - pass the task to spawn() or execute() method then send it to channel
        3 - make sure receiver is of type Arc<Mutex<Receiver>>
 
        how threadpool works?
        once the execute method is called the task is sent to the jobq channel 
        spawned threads on the other hand are thrilling to receive the task 
        coming from the channel that's why we should put the receiver inside 
        mutex and make it arc to move it between threads, once a free thread
        acquire the lock on the receiver then the task can be called inside 
        that thread
    */
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

        type AsyncJob<I, F> = Box<dyn FnOnce(I) -> F + Send + Sync + 'static>;
        fn create_async_job<I, O, F>(job: impl FnOnce(I) -> F + Send + Sync + 'static) -> AsyncJob<I, F> // static dispatch for job
            where F: std::future::Future<Output = O>, 
                  O: Send + Sync + 'static
            {
                Box::new(job)
            }
        async fn run(){
            let async_job = create_async_job::<u8, String, _>(|status|async move{String::from("")});
            tokio::spawn(async move{
                async_job(0).await;
            });
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
        
        /* 
            NOTE: the process of heavy cpu io must not be blocking that's why rayon is not 
            going to be used for none blocking operations cause it moves the tasks into the cpu 
            core instead of using threads per cpu it uses one thread per each cpu core, tokio 
            however can be used for io blocking tasks cause it uses lightweight thread of 
            execution and it blocks a light thread.
            lightweight threads in tokio is user thread space and naitive threads in rayon is os 
            threads, the first ones have less overhead than the seconds ones.
            spawning native threads are too slow since thread handling in rust is depends 
            on user base context switching means that based on the load of the IO in the 
            app rust might solve the data load inside another cpu core and use multiprocessing 
            approach, it's like rayon threadpool which are global threads and shared across the app
            which causes the race with other threads and steal tasks   
                • https://www.reddit.com/r/rust/comments/az9ogy/help_am_i_doing_something_wrong_or_do_threads/
                • https://www.reddit.com/r/rust/comments/cz4bt8/is_there_a_simple_way_to_create_lightweight/
        */
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