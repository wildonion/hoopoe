
// https://stackoverflow.com/questions/15056237/which-is-more-efficient-basic-mutex-lock-or-atomic-integer
/* -------------------------------------------------------------------------------
  ╰┈➤          Synchronisation Package: Arc vs Mutex/RwLock vs Atomic
    
    besides borrowing using & we got some smart and wrapper pointers allows us to share 
    the ownership of type between scopes and threads like Box, Rc, Arc (shared ownership
    without state mutation) however if we want to mutate the underlying data (shared mutable 
    state) we use &mut, RefCell, Atomic, Mutex and RwLock to make synchronisation between 
    scopes and threads safely.

    ⪼ note on pointers:
    ╰┈➤ single thread borrow: &mut | multithreaded borrow: Arc<Mutex<
        borrow requries lifetime of the type to be valid and don't gets dropped in other scopes cause 
        if the undelying type goes out of the scope the borrow gets invalidated although Rust updates 
        it with new address of the moved type but can't use after moving and dropping the type, Rust
        moves data around memory for ram optimisations by moving data they get new ownership address 
        inside the new scope this is why we shouldn't move a type if it's behind a pointer cause we 
        won't be able to use pointer after moving.

    Atomic                         ▶ single threaded/multithreaded shared reference with atomic synchronisation state mutation, supports only primitive types like atomic u32
    Rc/Arc                         ▶ single threaded/multithreaded shared reference without synchronisation and state mutation, requires Mutex or RwLock, use to build atomic type from complex types
    Rc<RefCell</Arc<Mutex</RwLock< ▶ single threaded/multithreaded shared reference with atomic synchronisation can be shared between threads safely even without using channels for state mutation and syncing
    redis redlock and zookeeper    ▶ can be used in a cluster to implement a distributed locking mechanism
    
    Arc: 
    is used for shared ownership of data across multiple threads so it's thread
    safe, it allows multiple threads to have read-only access to the same data without the 
    need for mutable access, it uses reference counting to track the number of references 
    to the shared data, it is a non-atomic operation and requires synchronization tools like
    Mutex or RwLock when modifying the reference count.
    
    Atomic:
    atomic types provide atomic operations on primitive types for shared mutable state, they
    are used for low-level synchronization and atomic operations on shared data, Atomic types 
    provide atomic read-modify-write operations on the underlying data, operations like 
    compare-and-swap, fetch-and-add, and atomic load/store are performed atomically without 
    the need for explicit synchronization.
    Atomic types ensure that operations on shared data are performed atomically without the 
    risk of data races, they are suitable for scenarios where fine-grained control over shared 
    mutable state is required they're also are typically more efficient than using locks or 
    mutexes for synchronization, especially for simple operations on primitive types.

    Key Differences:

    ╰┈➤ Ownership vs. Atomic Operations:
        Arc is used for shared ownership and reference counting, while atomic types are used for 
        atomic operations on shared mutable state.
    
    ╰┈➤ Complexity:
        Arc provides higher-level shared ownership semantics, while atomic types offer low-level 
        atomic operations on primitive types.
    
    ╰┈➤ Use Cases:
        Use Arc for shared ownership of complex data structures across threads.
        Use atomic types for fine-grained synchronization and atomic operations on primitive types.

    Atomic vs Mutex/RwLock:
    PG and Redis transactions are atomic by default (all or none) and they will be executed in a 
    multithreaded context like tokio::spawn(), the scheduler knows when to execute each transaction 
    asyncly but since they're atomic there will be no two mutator transactions executing at the 
    same time or if a transaction is being executed the second one must wait for the first one until 
    it finishes execution, it's difference with Mutex, atomics usually try attempting to do operation 
    until succeed and they're run in user mode unlike mutexes which run in kernel mode, mutexes 
    eventually end up being implemented with atomics since you need at least one atomic operation 
    to lock a Mutex, and one atomic operation to unlock a mutex, it takes at least twice long to do 
    a mutex lock. RwLock in comparison with Mutex, Mutex does not distinguish between readers or writers 
    that acquire the lock, therefore causing any tasks waiting for the lock to become available to 
    yield. An RwLock will allow any number of readers to acquire the lock as long as a writer is not 
    holding the lock.

    ╰┈➤ Mutex (Mutual Exclusion Lock):
        A Mutex allows only one thread to access some data at any given time.
        Every time a thread wants to access the data, it must first lock the Mutex.
        If you have a situation where you have more writes than reads, or if the 
        reads and writes are roughly equal, a Mutex is usually the better choice.
        tokio::sync::Mutex is an asynchronous Mutex that is designed to work 
        with async code within tokio.
    
    ╰┈➤ RWLock (Read-Write Lock / multiple readers or a single writer at a time):
        A RWLock allows any number of threads to read the data if there isn't a thread writing to it.
        If a thread wants to write to the data, it must wait for all the readers to finish before it 
        can obtain the lock. If you have a situation where you have many more reads than writes, a RWLock 
        can be more efficient because it allows multiple readers to access the data simultaneously.
        tokio::sync::RwLock is an asynchronous Read-Write Lock that works within the async ecosystem of tokio.
    
    ╰┈➤ When to choose tokio::sync::Mutex:
        You have frequent writes.
        The critical section (the part of the code that needs exclusive access to the data) is quick.
        You want simplicity. Using a Mutex is straightforward and avoids the complexity of dealing with 
        multiple lock types.
    
    ╰┈➤ When to choose tokio::sync::RwLock:
        You have many more reads than writes.
        You want to allow concurrent reads for efficiency.
        The critical section for reads is fast, but it’s still beneficial to have multiple readers at the same time.
        In many scenarios, a Mutex might be sufficient and can be the simpler choice, especially if write operations 
        are as common as reads or if the critical section is very short, thus not justifying the overhead of managing 
        a RWLock.

        However, if your specific case involves a lot of concurrent reads with infrequent writes and the 
        read operations are substantial enough to create a bottleneck, a RWLock might be a better choice.
*/

use std::sync::atomic::{AtomicBool, Ordering};
use actix_web::web::route;
use serde::{Deserialize, Serialize};
use plugins::purchase::AtomicPurchase;
use crate::*;



/*  
    orphan rule:
    we can't implement a trait from outside of the current crate for an struct from
    another crate unless the definition of one of them is in the crate we're implementing 
    the trait for the struct

    this doesn't work cause HttpRequest belongs to another crate
    and Passport definition is inside another crate but it's ok if we 
    have Passport definition in the current trait which the trait
    is being implemented for the struct.
    impl anothercrate1::Passport for anothercrate2::HttpRequest{} 

    in our case Product struct is defined here but AtomicPurchase trait definition
    is inside another crate and there would be no problem with implementing the trait
    for the Product struct but the following is not ok:
    impl anothercrate1::AtomicPurchase for anothercrate2::Product{}
*/
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Product{
    pub pid: i32,
    pub buyer_id: i32,
    pub is_minted: bool
}


impl AtomicPurchase for Product{
    type Product = Self;
    async fn atomic_purchase_status(&self) -> (bool, tokio::sync::mpsc::Receiver<Self::Product>) {
        mint_product_demo(self.pid, self.buyer_id).await
    }
}


/* -------------------------------------------------------------------------------------------
        atomic synchronisation and handling mutual exclusion state on purchasing product 
   -------------------------------------------------------------------------------------------
    same logic goes for any other purchasing process in which 
    multiple users want to do it simultaneously following logic 
    can be implemented on a cluster using redis set_nx() command 
    or redlock logic to acquire the lock then delete the key 
    after purchasing to release it (see dlm.rs).

    every lock operation must be spawned as an async task into tokio
    threadpool using tokio::spawn() to avoid blocking clients requests
    and current threads processes

    we spawn every lock task in a tokio threadpool to avoid blokcing current thread
    of actix worker then we decide to execute which async tokio thread joinhandle based 
    on the flow of the app using the event loop of the tokio

    atomic transaction syncing:
    concurrent requests buying same data requires a lock to be acquired per each 
    request and reject others until the purchase process compeltes for the first one
    during the purchasing process the id must be locked to reject other requests
    once the purchase completes the lock is released, both locking and releasing 
    must be considered as async tasks spawned inside separate tokio spawn scope 
    to avoid blocking current thread so other requests can be handled concurrently.
*/
pub(self) async fn mint_product_demo(pid: i32, buyer_id: i32) -> (bool, tokio::sync::mpsc::Receiver<Product>){

    // cloning the static mutex, having a global data in Rust requires to define 
    // an static type and if it wants to be mutated we should use arc mutex which 
    // converts it into a thread safe mutable data since Rust doesn't support raw
    // static for mutation due to avoiding deadlocks and race conditions.
    let lock_ids = consts::PURCHASE_DEMO_LOCK_MUTEX.clone();
    
    let (tx, mut rx) = 
        tokio::sync::mpsc::channel::<bool>(1024);
    
    /* ___ IMPORTANT
      ╰┈➤ each actix worker thread contains the whole application factory instance each of them
        handles async apis as async tasks in the background thread to avoid blocking issues
        it doesn't mean requests to the same api all will be handle in a same thread it's like
        client1 request to an api is inside worker thread1 and client2 request to the same 
        api may go inside worker thread2 so using any lock mechanism in the api causes 
        the second client request to be blocked cause the nature of the lock is to block the 
        thread using the shared data for mutation therefore every locking mechanism must 
        be as an async task and solved in tokio spawn. as a matter of fact we should acquire 
        the lock in a separate thread to avoid blocking actix requests in its server worker 
        thread otherwise the second request coming to the server gets blocked while the frist 
        client is minting the product and second client won't be responded until the minting 
        process of first client is over. actix sends its application factory into 4 worker 
        threads by default to process requests concurrently and if each api is an async task 
        they get solved asyncly without blocking the thread.

        since each request gets handled in a free thread choosed by the actix runtime which contains
        the application factory instance so any locking mechanism forces no other actix thread woker
        can use this data which causes to block the client request and makes him to get halted until
        the lock gets released.
    */
    tokio::spawn(
        { // cloning necessary data before moving into async move{} scope
            let tx = tx.clone();
            let lock_ids = lock_ids.clone();
            async move{
                let mut write_lock = lock_ids.lock().await;
                if (*write_lock).contains(&pid){
                    log::info!("rejecting client request another one is minting it!");
                    // reject the request
                    tx.send(true).await;
                } else{
                    (*write_lock).push(pid); // save the id for later readers to reject their request during the minting process
                }

            }
        }
    );


    let (psender, preceiver) = 
        tokio::sync::mpsc::channel::<Product>(1024);

    // second spawn, minting process and pop the pid out on any error 
    // so other clients can go for minting product again
    let start_minting_task = tokio::spawn(
        {
            let psender = psender.clone();
            let lock_ids = lock_ids.clone();
            async move{

                let (err, mut pinfo) = mint(pid, buyer_id).await;
                
                // set the minting flag to true so if there was no error
                if !err{
                    pinfo.is_minted = true;
                }
                
                // send the pinfo to the channel, we'll check the minting flag in the main api
                psender.send(pinfo).await;
                
                /* ___ IMPORTANT
                  ╰┈➤ if you have plan to put the following codes into a separate tokio spawn
                    then you need to do it on a condition like if there was an error then
                    execute the tokio::spawn task related to retaining ids this is IMPORTANT
                    to do cause every spawned task using tokio::spawn will be executed 
                    in the background without being dependent to other code blocks so 
                    executing the following inside a separate tokio::spawn without checking 
                    conditions results allowing two clients mint a product at the same time 
                    cause we're releasing the pid in here and releasing it in the background 
                    with no base conditions brings us deadlocks and race conditions or minting 
                    a product by two clients at the same time, cause tokio spawn tasks are
                    executing asyncly and independently means the spawn scope for the retaining
                    process might be executed first! 
                    eventually rollback the pid either way (on error or success), 
                    cause first we've done with minting and second we'll check the
                    minting flag in the main api and if it's true we'll proceed 
                    with storing product info in db and notify the client with the status
                */
                let mut write_lock = lock_ids.lock().await;
                (*write_lock).retain(|&p_id| p_id != pid);

            }
        }
    );

    /* ___ IMPORTANT
      ╰┈➤ the following event loop must cover the whole
        logics inside this function cause any code after that is unreachable.
        use tokio::select!{} event loop to prevent thread from blocking cause
        every async task kinda blocks the thread if they're not in tokio::spawn
        and executing all tokio::spawns in here is not what we want, we want to
        contro lthe flow of the function based only one task not running all
        of them in the background.
        we're waiting on mpsc channel of the check task and minting task when one of 
        them completes cancel the other cause there is either a product is being
        minted or is not minted yet,
        we're returning the preceiver back to the caller, it can receives the 
        product info for future processes.
    */
    tokio::select! {
        // if this branch is selected means the product minting
        // process is not finished or not started to mint yet
        Some(flag) = rx.recv() => {
            if flag{
                return (true, preceiver); // product is being minted
            } else{
                return (false, preceiver); // no one is minting the product
            }
        },
        // if this branch is selected means the product is inside 
        // the minting process and we should release the lock after
        // it completes and notify the client later, note that the 
        // whole logic of the minting process is inside the start_minting_task
        // spawned task
        _ = start_minting_task => {
            return (false, preceiver); // during the minting process, if we're here means the minting is done
        }
    }
    

}

pub(self) async fn mint(pid: i32, buyer_id: i32) -> (bool, Product){
    log::info!("minting product with id {}", pid);

    // it takes approximately 10 seconds or more to mint a product
    // meanwhile we MUST reject any request coming to the api from 
    // other clients upon purchasing this product, until the time
    // is over
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    
    let pinfo = Product{pid, buyer_id, is_minted: false}; // some product info
    (false, pinfo)
}

/* -------------------------------------------------------------------------------------------
         atomic synchronisation and handling mutual exclusion state on hashmap data 
  -------------------------------------------------------------------------------------------
    channel is useful to send tasks into threads to invoke them in a separate threads
    as well as sending their results into the channel to receive them outside of the 
    threads, a local type can be mutated safely using an atomic synchronisation logic
    like Arc<Mutex<. it can be mutated in two different ways, one is cloning it and 
    then move the cloned one into threads or methods or send it to a channel from multiple 
    different threads and scopes and receive it only in one scope or thread. an static 
    arc mutex type however can be mutated across all scopes and threads in the app safely 
    without using channels. in the context of http routers we can use Data<Arc<Mutex<
    to share data between routers' threads safely
*/
pub async fn atomic_map_demo(){

    // ----- joining thread vs executing in the background -----
    tokio::spawn(async move{
        
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let res = std::thread::spawn(
            {
                let tx = tx.clone();
                move ||{
                    let name = String::from("wildonion");
                    tx.send(name.clone());
                    name
                }
            }
        ).join().unwrap(); // waits for the associated thread to finish
        println!("joined thread result {:?}", res);

        // use rx to receive data in here without unwrapping the thread
        while let Ok(data) = rx.recv(){
            println!("mpsc channel result {:?}", data);
        }

    });

    // ----- atomic bool and channel -----
    // by default atomic types can be mutated in threads safely
    let atomic_bool = AtomicBool::new(true);
    // need channel to share atomic bool and generally any type of data
    // between threads, enables us to have it outsife of the threads
    let (atom_tx, mut atomc_rx) 
        = tokio::sync::mpsc::channel::<AtomicBool>(1);
    atom_tx.send(atomic_bool);
    let cloned_atom_tx = atom_tx.clone();

    // ----- atomic map -----
    // if you want to mutate a type inside tokio threads you have 
    // to share it between those threads using channels
    let mut map = std::sync::Arc::new( // Arc is atomic reference counting
        tokio::sync::Mutex::new(
            std::collections::HashMap::new()
        )
    );

    // ----- channel to share map -----
    // use channels to share the data owned by a threadpool between 
    // other threads otherwise Arc<Mutex is good for atomic syncing
    let (tx, mut rx) 
        = tokio::sync::mpsc::channel(1024);
    let cloned_tx = tx.clone();
    let cloned_map = map.clone();


    // locking in first threadpool after sending the data for mutation
    tokio::spawn(
        { // begin scope
            let tx = tx.clone();
            async move{ // return type of the scope
                println!("first spawn last state of the map: {:#?}", cloned_map);
                let mut _map = cloned_map.lock().await;
                (*_map).insert(String::from("wildonionkey"), String::from("wildonionval"));
                // channel is useful when we need to send data owned by the tokio scope 
                // or is a result of invoking an async task moved to tokio scope to 
                // different threads, however the content of the mutex has changed in
                // this tokio scope and we have access it in other scopes without receiving
                // from the channel
                tx.send(cloned_map.clone()).await; 
            }
        } // end scope
    );

    // since map is an atomic type we can dereference it in here to see
    // the its mutated content without receiving it from an mpsc channel
    println!("map has changed since it's an atomic type: {:#?}", *map);

    // instead of cloning the map again we've used channels to send the mutated map
    // into the channel so we can receive it inside another thread
    // locking in second threadpool after receiving the data 
    tokio::spawn(async move{

        // receiving atomic bool
        while let Some(atom) = atomc_rx.recv().await{
            println!("atom bool received");
            atom.store(false, Ordering::Relaxed);
            println!("atom bool mutated: {:#?}", atom);
        }

        // receiving mutexed data
        while let Some(map_data) = rx.recv().await{
            println!("second spawn last state of the map: {:#?}", map_data);
            let mut _map = map_data.lock().await;
            (*_map).insert(String::from("wildonionkey3"), String::from("wildonionval3"));
            cloned_tx.clone().send(map_data.clone()).await; // later catch it in other threads
        }
    });

    // locking in main thread
    println!("main thread last state of the map: {:#?}", map.clone());
    let mut map = map.lock().await;
    (*map).insert(String::from("wildonionkey2"), String::from("wildonionval2"));
    println!("main thread last state of the map: {:#?}", map.clone());


    /* results different on every run based on the tokio runtime scheduler
    
        first spawn last state of the map: Mutex {
            data: {},
        }
        main thread last state of the map: Mutex {
            data: {
                "wildonionkey": "wildonionval",
            },
        }
        second spawn last state of the map: Mutex {
            data: {
                "wildonionkey": "wildonionval",
            },
        }
        main thread last state of the map: {
            "wildonionkey": "wildonionval",
            "wildonionkey2": "wildonionval2",
        }

        -------
        
        main thread last state of the map: Mutex {
            data: {},
        }
        first spawn last state of the map: Mutex {
            data: <locked>,
        }
        main thread last state of the map: {
            "wildonionkey2": "wildonionval2",
        }

        -------

        main thread last state of the map: Mutex {
            data: {},
        }
        main thread last state of the map: {
            "wildonionkey2": "wildonionval2",
        }
        first spawn last state of the map: Mutex {
            data: <locked>,
        }
    
    */

}
