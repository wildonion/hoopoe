


// ------------------------------------------------------------
/* actor worker threadpool implementations from scratch 

    https://ryhl.io/blog/actors-with-tokio/
    https://medium.com/@maturationofthe/leveraging-rusts-tokio-library-for-asynchronous-actor-model-cf6d477afb19
    https://www.reddit.com/r/rust/comments/xec77k/rayon_or_tokio_for_heavy_filesystem_io_workloads/

    
    What is an Actor? 
    Actor is a threadpool or a single threaded structure which has its own mailbox and cron scheduler 
    to receive and execute tasks inside its thread of execution it can use tokio or os threads to execute 
    async io or cpu tasks, they talk through message sending patterns like mpsc in local and grpc remotely
    the message or task execution can be happened by receiving the task from the actor eventloop which is 
    the receiver of the mailbox jobq mpsc channel in local or the grpc remotely then execute in a free 
    thread if it has threadpool or its own single thread of execution, the thread can be either a light
    or os thread for executing async io or intensive cpu tasks.

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
        A threadpool has its own internal eventloop queue for popping out tasks.
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

use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::{default, fmt, thread};
use std::sync::mpsc as std_mpsc;
use futures::future::select;
use sha2::digest::generic_array::functional;
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
                    // following is called the eventloop handler, handles and execute
                    // all coming tasks from the channel in a loop using while let some 
                    // streaming and execute them in a separate thread as they're getting 
                    // received.
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

// see actor.rs for a real world example of writing a threadpool actor object
// executor uses an event loop to execute tasks in their own thread.
pub async fn ExecutorEventLoop(){

    // actor worker streaming with while let some, ws, atomic addr, mailbox, rpc, redis, rmq)
    // actor worker task execution with io vs os threads (lazy static &mut rc arc mutex rwlock select spawn channels scheudler interval)
    // thread queue and runtime scheduler to pop the task out of the queue, send sync static separate io threads vs os threads thread joining
    
    
    // concurrency      : (&mut <-> Mutex RwLock)
    // threadpool       : light or os threads, eventloop (threadpool channel queue to handle events in a loop see actor.rs with threadpool)
    // atomic syncing   : channels mutex rwlock arc select
    // future objects   : async io task, thread joining on main (caller) thread
    // purchase locking : lock the object when someone is minting it using select mutex spawn
    
    /*
        simple executor without threadpool but with eventloop:
        task is a unit of work thread or groutine that must be executed 
        by runtime executor by poping it out of the queue, where each task 
        runs to completion before the next task starts.
        it's similar to how tokio event loop manages async io future object 
        using an eventloop in its lightweight thread of execution
    */

    use tokio::sync::{mpsc::channel, Mutex};
    enum EventData{
        Task(Task),
        Quit
    }
    // traits as separate objects must be behind dyn keyword 
    // and boxed into the heap cause they're unsized
    type Function = fn();
    struct Fucntions<F: FnOnce() + Send + Sync + 'static, 
        A: std::future::Future<Output = String> + Send + Sync + 'static>{
        pub func: Function,
        pub cls: F,
        pub boxed_cls: Box<dyn FnOnce() + Send + Sync + 'static>,
        pub fut: A,
        pub fut1: std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + Sync + 'static>>
    }
    struct Task{
        name: String,
        func: Box<dyn FnOnce() + Send + Sync + 'static>, // closures are traits, traits are not sized thus we use dyn and need to behind pointer hence Box<dyn
        fut: Option<std::pin::Pin<Box<dyn std::future::Future<Output=String> + Send + Sync + 'static>>>,
    }
    impl Task{
        fn new<F: FnOnce() + Send + Sync + 'static>(name: String, func: F) -> Self{
            Task { name, func: Box::new(func), fut: None }
        }
        fn run(self){ // don't use &self in here cause we can't move out of it since func is not cloned
            (self.func)() // calling the the function
        }
    }
    struct Executor{
        pub tx: mpsc::Sender<EventData>,
        pub rx: mpsc::Receiver<EventData>
    }
    impl Executor{
        fn new(buffer_size: usize) -> Self{
            let (tx, rx) = tokio::sync::mpsc::channel::<EventData>(buffer_size);
            Executor{
                tx,
                rx
            }
        }
        fn spawn<F: FnOnce() + Send + Sync + 'static>(&mut self, name: String, func: F){
            self.tx.send(EventData::Task(Task::new(name, func)));
        }
        // the event lopp
        async fn run(&mut self){
            // await puases the execution and run the future until it completes
            // futures don't block the thread of execution
            while let Some(event_data) = self.rx.recv().await{
                match event_data{
                    EventData::Task(task) => {
                        log::info!("executing the task with name: {}", task.name);
                        std::thread::spawn(move || task.run());
                    },
                    EventData::Quit => {
                        break;
                    },
                    _ => {
                        panic!("invalid event data, event loop is panicked");
                    }
                }
            }
        }
    }

}

pub fn StockPriceEvent(){

    /* --------------------------------------------------------------------
        condvar is used to check that either a condition var is met
        inside a mutex or not, this will block the mutex thread by 
        waiting on it until this cond var receives a notification in 
        somewhere else.
        CondVar: block the thread such that it consumes no CPU time while
        waiting for an event to occur, other threads can do their jobs
        functions in this module will block the current thread of 
        execution, condvars are typically associated with a boolean 
        predicate (a condition) and a mutex, the predicate is always 
        verified inside of the mutex before determining that a 
        thread must block.
        condvar blocks the mutex thread by waiting until new changes 
        is received which has notified in other threads.
        what we're doing here is basically we're monitoring a stock price in 
        a safe manner in such a way that we're creating 10 threads, each of 
        them wants to mutate the price of the stock instance but the actual 
        instance is wrapped through a mutex and has a condvar, the updating 
        process is happened like we start by locking on the stock instance 
        then call update price method after that notify the condvar that the 
        value of the price of the stock instance has changed so the notification 
        process is happening inside each thread. 
        then in the function we're calling the wait for release which lock the 
        stock again and checks its price against a limit causes to block the 
        main (caller) thread until the price of the stock is smaller than the limit, it 
        depends on the update price every time the update price function update 
        the price of the stock a notif gets triggered which will be checked 
        by the wait for release method to check the price agains the limit this 
        process continues constantly the main (caller) thread is blocked until the price 
        reaches a higher amount than the limit.
    */


    use std::sync::{Arc, Mutex, Condvar};

    struct Buffer<T>{
        pub data: Arc<Mutex<Vec<T>>>,
        pub size: usize
    }

    #[derive(Debug, Clone)]
    struct Stock{
        name: String,
        price: f64
    }
    impl Stock{
        fn new(name: &str, price: f64) -> Self{
            Self { name: name.to_string(), price }
        }
        fn getPrice(&self) -> f64{
            self.price
        }
        fn getName(&self) -> &str{ // ret pointer, use the lifetime of the self
            &self.name
        }
        fn updatePrice(&mut self, new_price: f64){
            self.price = new_price;
        }
    }

    // worker, locker
    struct Monitor{
        pub event: std::sync::Mutex<Stock>,
        pub events: Option<Buffer<Stock>>,
        pub event_signal: Condvar,
        pub std_worker: thread::JoinHandle<()>,
        pub tokio_worker: tokio::task::JoinHandle<()>,
        pub locker: std::sync::Mutex<()>
    }

    impl Monitor{
        fn new(init_event: Stock) -> Self{
            Self {
                events: None,
                event: std::sync::Mutex::new(init_event), 
                event_signal: Condvar::new(), 
                std_worker: thread::spawn(move ||{}), 
                tokio_worker: tokio::spawn(async move{}),
                locker: std::sync::Mutex::new(())
            }
        }

        fn update_price(&self, new_price: f64){
            let mut get_stock = self.event.lock().unwrap();
            (*get_stock).updatePrice(new_price);

            // wakes up one blocked mutex thread on this condvar
            // we notify the condvar that the stock price is changed
            // the update_price method locks the Stock instance, updates 
            // its price, and then calls notify_one() on the Condvar. 
            // this notifies any thread waiting or blocking on the Condvar 
            // that the stock price has changed.
            self.event_signal.notify_one() // notify the blocked thread that the value has changed, wake it up from the wait status  
        }

        /* 
            once the price of the locked stock reaches the limit we wait, wait blocks the current thread 
            until this condition variable receives a notification which will be triggered inside the 
            update_price method means that the price of the stock has changed and we need to block the 
            mutex thread again until we reaches the limit again for the stock price.
            in the wait_for_release() method, we lock the Stock object. it then enters a loop where 
            it continually checks if the price of the Stock is less than a certain limit. if the price 
            is less than the limit, the method calls the self.event_signal.wait(get_stock) 
            method. this block the current (main) thread of the mutex, until another thread calls notify_one() 
            or notify_all() on the same Condvar
            the consequence of this, is that if the price of the Stock is initially less than the limit, 
            this method will block the current (main) thread until the price increases to the limit or above. 
            this will allow other threads to update the price of the Stock while the current (main) thread is 
            blocked. once the prices reaches the limit, the wait() method will return. the method will 
            exit the loop and continue executing.
            using a Condvar in this way, we can effectively manage access to the Stock. By using the 
            wait_for_release() method, the main (caller) thread waits for the price of the Stock to reach a certain 
            limit before proceeding. this is useful in scenarios where the order of operations matters, 
            for example when one operation depends on the result of another. example scenarios would be 
            things like managing stocks, purchasing a product, or a warehouse ledger system.
        */
        fn wait_for_release(&self){
            let limit = 115.0;
            let mut get_stock = self.event.lock().unwrap();
            while get_stock.getPrice() < limit{ // we block and wait as long as the the price is smaller than the limit
                get_stock = self.event_signal.wait(get_stock).unwrap();
            }

        }

    }


    /* 
        testing:
        basically in here we're updating the price 
        in 10 threads and block the main (caller) thread if 
        the price is smaller than the limit until 
        we notify the blocked thread by the condvar 
        that the price value is changed, then there 
        would be no need to wait for the notif until
        another thread tries to update the price.
        we spawn the update_price() method inside 10
        threads then block the main (caller) thread if the price
        is not met the limit finally we iterate through 
        all the threads to join them on the main (caller) thread
        and wait for them to finish.
        waiting in os threads means blocking the thread 
        until we get the result.
    */
    
    // arc monitor to move it between threads
    let monitor = Arc::new(Monitor::new(Stock::new("DOGTOKEN", 100.0)));
    let threads = (0..10)
        .map(|counter|{
            let cloned_monitor = monitor.clone();
            // we'll update the price of the monitor instance in a separate 10 of threads
            thread::spawn(move ||{
                cloned_monitor.update_price(110.0 + 2.0*(counter as f64));
            })
        })
        .collect::<Vec<_>>(); // if you don't know the type use _

    // we'll check the price of the stock against the limit 
    // if it was less than the limit then we'll block the main 
    // thread until the notifier notify the condvar in another 
    // thread with a new value of the price, then we'll wait and 
    // block the thread until the price reaches higher than the limit again.
    // ------- this blocks the main (caller) thread -------
    monitor.wait_for_release(); 

    // join on all threads in main (caller) thread to execute the stock price task
    for thread in threads{
        thread.join().unwrap();
    }

    // finally get the final value of the stock event after all mutations
    let final_value = monitor.event.lock().unwrap();
    println!("final value of the stock is {:?}", final_value);


    // wait_for_release() method blocks the main (caller) thread until we reach
    // the limit, or receives a notification from the condvar which might
    // happens in another thread by updating the price of the stock.  

    /* 
        product minting: 
           - condvar with tokio spawn threads: use a condvar with a mutex and lock the product id in a tokio
                      io thread then check while the product is still locked or its state is not minted yet, 
                      we'll block the current thread until the notifier notifies the condvar once the 
                      product gets minted successfully and its state changed to minted then we'll remove it 
                      from the lock_ids.
           - channels with tokio spawn thrads: use a mutex and lock the product id in a tokio io thread
                      then send a true flag to a channel if the product id is being locked then start minting 
                      product in another tokio io thread finally use select to control the flow of execution 
                      of each joinhandle task.
    */
    

}

pub fn MutexCondvarPlayground(){
    
    /* 
        in monolithic  : use mutex, rwlock, arc, channels, spawn, select for atomic syncing and avoiding deadlocks
        in microservice: use distributed lock tools like redlock, zookeeper, k8s
        eventloop streaming with channels as publisher and subscriber of events like condvar
        atomic syncing with mutex, condvar and channels in async and none async block 

        none async version: std threadpool executor eventloop, channels, mutex will block the current thread and wait for the task to gets executed
        async version     : tokio threadpool executor eventloop, channel, mutex won't block the thread and execute tasks in the background thread 
    
        NOTE: don't use Condvar in async environment cause it blocks the thread
        NOTE: Mutex in both std and tokio will block the thread to make sure that only one thread can access data for writing
    
        use channel to share data between threads instead of Mutex or RwLock
        tokio::spawn(), tokio::select! {}, channels, tokio::sync::Mutex
        execute async io future obejcts in an io thread in a none blocking manner
        there is no need to block threads cause we want to execute other tasks 
        without blocking any code flow or section in a concurrent manner.
    
        use mutex to ensure that only one thread can access the protected data.
        mutex always block the current thread to ensure that only one thread can mutate the data.
        do the locking in a separate thread if you care about executing rest of the codes in a none blocking manner.
        use condvar to block the thread or wait for a notification on some value changes.
        joining on thread block the thread till we get the result from that
        in none async env we should use os threads which might get block during the execution when we need the result inside the thread.
    
        tokio runtime scheduler and executor can execute async io tasks in a none blocking manner 
        by awaiting on async tasks the executor pause the execution in there but don't block the thread 
        it goes to run and execute other tasks in other threads concurrently in a none blocking manner
        it then notify the caller and fill the placeholder once the task completes.    
    */
    

    /* ---------------- BUCKET ----------------
        a bucket that contains a queue which is a pool of events 
        all the events are safe to be shared and mutated between threads.
        it has a locker, worker, condvar and channels, eventloop streamer 
        to receive the task using receiver, use condvar with mutex 
        to lock the thread and wait for the notifier then if we want 
        to fill the bucket we should use either channels, or condvar mutex
    */
    pub struct Bucket<S, T: Clone + Send + Sync, F: std::future::Future<Output = String> + Send + Sync>{
        pub signal: std::sync::Condvar,
        pub broadcaster: std::sync::mpsc::Sender<S>,
        pub receiver: std::sync::mpsc::Receiver<S>,
        pub fut: F,
        pub dep_injection_fut: std::pin::Pin<Box<dyn std::future::Future<Output = String>>>,
        pub worker_handler: std::thread::JoinHandle<()>,
        pub queue: BufferEvent<T> // a thread safe queue to mutate it - while !self.queue.is_empty() { pop the card out }
    }
    pub struct BufferEvent<T>{
        pub data: std::sync::Arc<std::sync::Mutex<Vec<T>>>, // Mutex always block the thread for mutating avoid accessing the data by other threads at the same time
        pub size: usize
    }
    impl<T> Drop for BufferEvent<T>{
        fn drop(&mut self) {
            self.data.lock().unwrap().clear(); // clear the whole events
        }
    }
    /* ---------------------------------------- */


    // example: safe atomic data syncing between threads using mutex and condvar 
    #[derive(Debug, Clone)]
    pub enum Color{
        Yellow,
        Red,
        Blue
    }
    #[derive(Clone, Debug)]
    pub struct ExpensiveCar{
        pub brand: String,
        pub color: Color
    }

    impl fmt::Display for ExpensiveCar{
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let color_string = format!("{:#?}", self.color);
            write!(f, "brand: {}, color: {}", self.brand, color_string) // write to the current buffer formatter
        }
    }
    
    impl ExpensiveCar {
        fn new(brand: &str, color: Color) -> ExpensiveCar {
            ExpensiveCar {
                brand: brand.to_string(),
                color,
            }
        }
    }
    
    #[derive(Debug)]
    pub struct Garage{
        queue: Buffer<ExpensiveCar>,
        guard: std::sync::Condvar, // the condvar notifier notifies the blocked or waiting thread that there might be data to be consumed.
    }

    #[derive(Debug)]
    pub struct Buffer<T: Clone>{
        // Arc allows us to share between multiple threads by cloning the data (each thread gets its own ref to it)
        // Mutex ensures only one thread at a time are mutating the data
        pub data: std::sync::Arc<std::sync::Mutex<Vec<T>>>,
    }
    impl<T: Clone> Buffer<T>{
        pub fn new() -> Self{
            Self{data: std::sync::Arc::new(std::sync::Mutex::new(vec![]))}
        }

        pub fn size(&self) -> usize{
            let get_data = self.data.lock().unwrap();
            get_data.len()
        }
    }

    #[derive(Debug, Clone)]
    pub struct PooledObject<T: Clone>{
        pub item: T, // used to push back the last item into the data buffer when the pooled object is being dropped
        pub pool: std::sync::Arc<std::sync::Mutex<Vec<T>>>,
    }

    // if you want to park means the garage must get blocked 
    // by the mutex and tells other threads that i'm busy with 
    // parking a car wait (block) until i release the lock and 
    // the car gets parked in there.
    impl Garage{
        
        pub fn new() -> Self{
            Self { queue: Buffer::new(), guard: std::sync::Condvar::new() }
        }

        pub fn pool_size(&self) -> usize{
            self.queue.size()
        }

        pub fn acquire(&self) -> Option<PooledObject<ExpensiveCar>>{
            let mut get_cars = self.queue.data.lock().unwrap();
            let pool_of_expensive_cars = (*get_cars).pop()
                .map(|car| PooledObject{ // mapping the poped out element into a PooledObject
                    item: car,
                    pool: self.queue.data.clone(),
                });
                pool_of_expensive_cars
        }

        pub fn park(&self, car: ExpensiveCar){ // try to park an expensive car inside the garage
            let mut get_queue = self.queue.data.lock().unwrap();
            (*get_queue).push(car);

            // notify the blocked thread that we have pushed a new car into the garage
            // since the thread will be blocked if there would be no data in queue 
            // hence we should notify the thread as soon as a car comes into the garage
            self.guard.notify_one();
        }   
        pub fn get(&self) -> ExpensiveCar{
            let mut get_queue = self.queue.data.lock().unwrap();
            while get_queue.is_empty(){
                get_queue = self.guard.wait(get_queue).unwrap();
            }

            // if the blocked thread gets woken up in here we remove the 
            // first item from the queue and return it since the get method 
            // method is about returning an expensive car
            (*get_queue).remove(0)
        }

    }


    /* ----------------------------------------------------------------------
        impl ownership and borrowing traits for the PooledObject
        deref mutable/immutable pointer will be called by the compiler at runtime
        we can also use * to deref a type and get the value out of the pointer
        deref any pointer (immutable or mutable) to the item of type T.
        change the value of a pointer or &mut by dereferencing it like *v = 1;

        the drop, deref and derefmut will be called automatically at runtime 
        when the type wants to gets dropped out of the ram and dereferenced
        by * or its inner value gets accessed by during the execution.

        note: pointers contain inner value of a type so accessing the inner 
        value requires to deref the pointer so when we use * it allows us to 
        access the inner value for mutating or reading, the Deref/DerefMut 
        traits however can be implemented for smart pointers like Mutex, Arc, Box
        making them behave like regular references.
        reimplementing the Deref/DerefMut trait methods for a type allows us 
        to call their methods on the object when we try to access the inner value
        of the type by * or 

        when we use * operator the deref or derefmut trait methods will be called 
        by the compiler as well as for smart pointers cause smart pointers implement 
        the traits any type that implements the deref or derefmut traits, trait methods 
        will be invoked once it’s dereferenced by * 
    */
    impl<T: Clone> Drop for PooledObject<T>{
        fn drop(&mut self) {
            // we must push back the item into the pool cause we've poped it out 
            // in acquire method
            println!("dropping pooled object, pushing back the item into the pool");
            let mut get_items = self.pool.lock().unwrap();
            (*get_items).push(self.item.clone());
        }
    }
    impl<T: Clone> Deref for PooledObject<T>{ // dereference the immutable pointer of type T which is the type of the pooled object item
        type Target = T;
        // compiler calls the deref() method once the type gets dereferenced by * immutably
        fn deref(&self) -> &Self::Target {
            println!("dereferencing pooled object");
            &self.item
        }
    }
    impl<T: Clone> DerefMut for PooledObject<T>{ // dereference the mutable pointer of type T which is the type of the pooled object item 
        // this is useful when we have a mutable pointer to the last item like `ref mut`
        // compiler calls the deref() method once the type gets dereferenced by * mutably
        fn deref_mut(&mut self) -> &mut Self::Target {
            println!("dereferencing pooled object mutably");
            &mut self.item
        }
    }

    /* 
        actor workers have os or ligh thread of executions (see actor.rs)
        they can communicate with other parts through message passing techniques
        using jobq channels, they're basically an smart threadpool object
        with an isolated state from each other and are safe to mutate data
        in their states. there must also be a task executor or some runtime
        scheduler to handle task execution process in each thread.
        tools: threadpool, mutex, spawn, arc, channels, executor and eventloop  
    */
    // worker threads
    pub struct Handlers{
        pub producer: std::thread::JoinHandle<()>,
        pub consumer: std::thread::JoinHandle<()>
    }

    impl Handlers{
        pub fn new() -> Self{
            Self { producer: std::thread::spawn(move||{}), consumer: std::thread::spawn(move ||{}) }
        }
        pub fn produce(&mut self, garage: std::sync::Arc<Garage>){
            // create 10 new expensive cars in a thread
            // the producer threads, produces 10 expensive cars to park them
            let cloned_garage = garage.clone();
            self.producer = thread::spawn(move || {
                for i in 0..10{
                    let new_expensive_car = ExpensiveCar::new("BMW", Color::Red);
                    cloned_garage.park(new_expensive_car);
                }
            });
        }
        pub fn consume(&mut self, garage: std::sync::Arc<Garage>){
            // get all the 10 cars from the garage one by one
            // the consumer thread, consumes or get 10 expensive car from the garage
            // since the garage queue is a lock hence we should wait and block the 
            // thread per each consumed data
            self.consumer = thread::spawn(move ||{
                for i in 0..10{
                    let get_expensive_car_from_queue = garage.get();
                    println!("got a car: {}", get_expensive_car_from_queue);
                }
            });
        } 

    }

    // with mutex and condvar we can control the producer and consumer 
    // threads at the same time, the first one is used to produce the 
    // data and the second one is used to consume the data by locking 
    // the data one at a time.

    let garage = std::sync::Arc::new(Garage::new());
   
   
    // ----------------------------------------------------------
    // poping all the cars out of the garage using consumer thread
    // ----------------------------------------------------------
    let mut handler = Handlers::new();
    handler.produce(garage.clone()); // fill the garage 
    // handler.consume(garage.clone()); // pop out the cars one by one

    // join the both producer and consumer threads at the same time 
    // the first one is responsible for creating then parking cars
    // inside the garage queue and second one is responsible for 
    // getting cars one by one from the garage, since the garage 
    // queue is a locker we should consider this that at the time of
    // parking a new car we should notify the blocked thread since 
    // the thread or the consumer is blocked until a new card comes 
    // into the parking.
    let Handlers { producer, consumer } = handler;
    producer.join().unwrap();
    // consumer.join().unwrap();

    // ----------------------------------------------------------
    // poping all the cars out of the garage using pooled object
    // ----------------------------------------------------------

    // testing pooledobject with mutable and immutable pointers to the 
    // the last item of type T
    let pool = garage.clone();
   
    // scope1
    { // define a new scope for fetching cars
        let mut get_poped_out_item_with_pool = pool.acquire(); // acquireing the pooledobject 
        match get_poped_out_item_with_pool{
            // poped_out_item_with_pool is a mutable poitner to the PooledObject<ExpensiveCar>
            // when dropping the object it calls the deref mut trait
            Some(ref mut poped_out_item_with_pool) => {
                println!("scope1 pooledobject : {:#?}", poped_out_item_with_pool.pool);
                poped_out_item_with_pool.color = Color::Yellow;
            },
            None => println!("there is no item!"),
        }
    
    // after the scope has ended this object is dropped and automatically the last item returned to the pool
    // since we're dereferencing mutably to mutate the color of the car the DerefMut trait will be called during 
    // this process and the log message will be printed out to the console.
    } 

    println!("-------------------------------------------");
    println!("pool size is : {:#?}", pool.pool_size());

    // scope2
    let get_car = pool.acquire();
    match get_car{
        Some(ref car) => println!("scope2 pooledobject: {:?}", car),
        None => println!("there is no item!"),
    };

    println!("-------------------------------------------");
    println!("pool size is : {:#?}", pool.pool_size());

    // scope3
    let last_car = pool.acquire();
    match last_car{
        Some(ref car) => println!("scope3 pooledobject: {:?}", car),
        None => println!("there is no item!"),
    };

    println!("-------------------------------------------");
    println!("pool size is : {:#?}", pool.pool_size());


}

pub async fn jobQChannelFromScratch(){

    // use trait to pass different types to a function through a single interface
    // use Any to try to cast any type that impls Any trait into an specific type
    // pass different functions to a method using Fn closure
    // dependency injection for observer field inside the Mutex (it can be other smart pointers like Arc and Box also)

    trait GetVal{
        fn getVal(&self) -> Self;
    }
    impl<T: Clone> GetVal for T{
        fn getVal(&self) -> Self {
            self.clone()
        }
    }

    struct Person<V>{
        info: V,
        // dependency injection with Mutex like Box smart pointers
        subscribers: Vec<std::sync::Arc<std::sync::Mutex<dyn Fn(V) + Send + Sync>>>,
    }

    impl<V: Any + GetVal + Send + Sync + Clone> Person<V>{
        pub fn new(info: V) -> Self{
            Self { info, subscribers: Vec::new() }
        }

        // hanlding dynamic dispatch, supports any type through a single interface
        pub fn set_info(&mut self, val: V){

            // cast the value into the Any trait, we could use Box also 
            let any_val = &val as &dyn Any;
            match any_val.downcast_ref::<String>(){
                Some(string) => {
                    // ...
                },
                None => {
                    println!("can't downcast it to string");
                }
            };

            self.info = val.getVal();
            self.publish(val); // name has changed
        }

        // moving out of a type which is behind pointer takes 
        // the ownership of the type so we can't do this if the type is    
        // being used by the function scope and is behind a pointer, 
        // like self.name which returning it takes the ownership of 
        // the &self, Rust won't allow to return a pointer from the 
        // function since once the function gets executed all the inner
        // types will be dropped out of the ram and having a pointer outside
        // of the function would be meaningless unless we specify a valid
        // lifetime for the pointer like using the self lifetime or pass
        // directly to the function call addressing this issue would be either 
        // returning a pointer to the self.name or clone the self.
        pub fn get_info(&self) -> &V{
            &self.info
        }
        
        // this method adds new subscriber closure function to the current ones
        pub fn subscribe<F: Fn(V) + Send + Sync + 'static>(&mut self, f: F){
            self.subscribers.push(std::sync::Arc::new(std::sync::Mutex::new(f)));
        }

        // this method publish a new value to all subscribers by iterating through 
        // all of them and call each closure function of them by passing the value
        // the closure function body will be executed for each of the subscriber 
        fn publish(&self, value: V){ // this method notify all subscribers by passing the new value to each subscriber function
            for subscriber in &self.subscribers{ // use & or clone the vector
                if let Ok(subscriber_function) = subscriber.lock(){
                    subscriber_function(value.clone()) // notify the subscriber with a new name, call the closure function and pass the new name to see the log
                }
            }
        }

    }


    // since the subscribe method has &mut self, in order to call 
    // the method on the instance in another thread we should wrap 
    // the instance in Mutex
    let person = std::sync::Arc::new(
        std::sync::Mutex::new(
            Person::new(String::from("wildonion"))
        )
    );

    let cloned_person = std::sync::Arc::clone(&person);
    
    // --------------------------------------------------------
    // working with person object completely in another thread.
    // --------------------------------------------------------
    let thread1 = thread::spawn(move ||{
        let mut person = cloned_person.lock().unwrap();
        person.subscribe(move |info| {
            // the subscription logic goes here
            // for now we're just logging things!
            println!("[subthread subscriber]");
            println!("subscribing > value is : {}", info);
        });

        // updating the info field, will notify all subscribers 
        // and call their functions, which the logic has written right above 
        person.set_info(String::from("new wildonion"));
    });

    // -------------------------------------------------------------
    // working with person object completely inside the main (caller) thread.
    // -------------------------------------------------------------
    // block the main (caller) thread for subscription
    // subscribe() method push a new subscriber to the vector only
    person.lock().unwrap().subscribe(move |info|{
        println!("[main (caller) thread subscriber]");
        println!("subscribing > value is : {}", info)
    });

    // set_info() change the info field as well as notify subscribers 
    // with the updated value
    // block the main (caller) thread for changing the ingo
    person.lock().unwrap().set_info(String::from("28"));

    // block the main (caller) thread to wait for the thread to complete the task
    // wait for the thread to finish, this method returns immediately if the 
    // thread has already finished, so joining on the thread can be important 
    // if we need a result coming from the thread otherwise the thread will
    // be solved in the background like tokio spawn threads. 
    thread1.join().unwrap();


}