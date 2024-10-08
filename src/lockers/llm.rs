
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
//-----------------------------------------------------------------------------------------------------

use std::sync::atomic::{AtomicBool, Ordering};
use constants::PRODUC_IDS;
use interfaces::product::ProductExt;
use salvo::concurrency_limiter;
use serde::{Deserialize, Serialize};
use crate::{constants::PURCHASE_DEMO_LOCK_MUTEX, *};


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

    in our case Product struct is defined here but ProductExt trait definition
    is inside another crate and there would be no problem with implementing the trait
    for the Product struct but the following is not ok:
    impl anothercrate1::ProductExt for anothercrate2::Product{}
*/
#[derive(Extractible)]
#[salvo(extract(default_source(from="body")))]
#[derive(Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct Product{
    #[salvo(extract(source(from="query")))]
    pub pid: i32, // pid gets filled from the query
    pub buyer_id: i32,
    pub is_minted: bool,
}

#[derive(Debug, PartialEq)]
pub enum PurchasingStatus{
    Minting, // someone is minting
    Locked, // pid is locked and ready to gets minted, notify client
    Done
}

impl ProductExt for Product{
    type Product = Self;
    async fn atomic_purchase_status(&mut self) -> (PurchasingStatus, tokio::sync::mpsc::Receiver<Product>) {
        start_minting(self.clone()).await
    }
    async fn mint(&mut self) -> (bool, Product){ 
        
        let Product{pid, buyer_id, is_minted} = self.clone();
        log::info!("minting product with id {}", pid);

        /////-------------- logic1
        /////------------------------------

        // eg: it takes approximately 10 seconds or more to mint a product
        // meanwhile we MUST reject any request coming to the api from 
        // other clients upon purchasing this product, until the time
        // is over
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        /////-------------- logic2
        /////------------------------------
        // send the product into to the rmq channel, we receive it 
        // inside the mint service and start minting then produce 
        // the result into the exchange so the consumer in here can 
        // consume it and update the product info in here.
        tokio::spawn(async move{
            
            // -----------------------
            //// FUTURE IMPROVEMENTS
            // -----------------------
            /*  https://github.com/wildonion/nftport-minter-service-actor
                step0) up and run the nftport-minter-service-actor service actor
                step1) in main service: send product info to notif producer actor to send to rmq
                step2) lock the product id in main service so other users can't start minting at
                    the time of minting the product
                step3) in mint service: 
                    notif consumer actor receives the product info and starts the minting process
                    once it finishes with the process the result will be sent to rmq using its
                    notif producer actor
                step4) in main service: 
                    notif consumer actor begins to start consuming in the background it can be either
                    where the http server is being started, by callig an api to register it or inside
                    a loop{} to keep the app running constantly to receive messages from the queue as
                    they're generating by the producer in meanwhile, however in either way we know the 
                    queue! as soon as its actor gets started, it receives all products constantly from 
                    the rmq, if the product was minted or there was any error then we release the id 
                    from the global locker in main service
                t1 => every locking process on the ids must be inside tokio spawn to avoid blocking
                t2 => get notif_owner:{} key which contains all notif data values for the owner
                t3 => notify client with short polling
            */
            
        });

        // doesn't matter to update the is_minted in here cause it gets updated
        // after a successful minting
        let pinfo = Product{pid, buyer_id, is_minted }; // some minted product info
        (false, pinfo)

    }
}


/* -------------------------------------------------------------------------------------------
        atomic synchronisation and handling mutual exclusion state on purchasing product 
   -------------------------------------------------------------------------------------------
    
    sub-tasking solutions:
        
        ╰┈➤ Using multi-task concurrency: each smaller of work would be its own task. The user 
            would spawn each of these tasks onto an executor. The results of the task would be 
            collected with a synchronization primitive like a channel, or the tasks would be 
            awaited together with a JoinSet.

        ╰┈➤ Using intra-task concurrency: each smaller unit will be a future run concurrently 
            within the same task. The user would construct all of the futures and then use a 
            concurrency primitive like join! or select! to combine them into a single future, 
            depending on the exact access pattern.

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

    channel is useful to send tasks into threads to invoke them in a separate threads
    as well as sending their results into the channel to receive them outside of the 
    threads, a local type can be mutated safely using an atomic synchronisation logic
    like Arc<Mutex<. it can be mutated in two different ways, one is cloning it and 
    then move the cloned one into threads or methods or send it to a channel from multiple 
    different threads and scopes and receive it only in one scope or thread. an static 
    arc mutex type however can be mutated across all scopes and threads in the app safely 
    without using channels. in the context of http routers we can use Data<Arc<Mutex<
    to share data between routers' threads safely

    step1) first acquire the lock inside tokio::spawn to avoid blocking 
    step2) second check the id is already inside the lock_ids or not 
    step3) if it's in there then we must reject the request
    step4) otherwise we can proceed to minting process
*/
pub(self) async fn start_minting(product: Product) -> (PurchasingStatus, tokio::sync::mpsc::Receiver<Product>){

    /* ___ IMPORTANT
      ╰┈➤ in handling async future io tasks remember to use Mutex in a separate light io threads
        to avoid blocking the main or actual thread the request is being handled in like always 
        do the lock process of PURCHASE_DEMO_LOCK_MUTEX inside a separate thread of tokio::spawn()
    */

    let Product { pid, buyer_id, is_minted } = product.clone();

    // cloning the static mutex, having a global data in Rust requires to define 
    // an static type and if it wants to be mutated we should use arc mutex which 
    // converts it into a thread safe mutable data since Rust doesn't support raw
    // static for mutation due to avoiding deadlocks and race conditions.
    let lock_ids = constants::PURCHASE_DEMO_LOCK_MUTEX.clone();
    
    let (tx, mut rx) = 
        tokio::sync::mpsc::channel::<PurchasingStatus>(1024);

    let (psender, preceiver) = 
        tokio::sync::mpsc::channel::<Product>(1024);
    
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
    // ===================================== locking task
    tokio::spawn(
        { // cloning necessary data before moving into async move{} scope
            let clonedTx = tx.clone();
            let lock_ids = lock_ids.clone();
            async move{
                let mut write_lock = lock_ids.lock().await;
                if (*write_lock).contains(&pid){
                    log::info!("rejecting client request cause the id is still being minted!");
                    // reject the request since the product is being minted
                    clonedTx.send(PurchasingStatus::Minting).await; // sending the minting flag as rejecting the request
                } else{
                    clonedTx.send(PurchasingStatus::Locked).await; // sending the locked flag as the product is already locked and ready to gets minted
                    (*write_lock).push(pid); // save the id for future minters to reject their request during the first minting process
                }
            }
        }
    );


    // ===================================== minting task
    // we'll await on this task until it gets solved completely 
    // by the runtime which means the product has been minted 
    // successfully.
    let clonedTx1 = tx.clone();
    // second spawn, minting process and releasing the pid lock
    let mintTask = tokio::spawn(
        {
            let psender = psender.clone();
            let lock_ids = lock_ids.clone();
            let mut product_info = product.clone();
            async move{

                let (err, mut pinfo) = product_info.mint().await;
                
                // set the minting flag to true, if there was no error
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
                (*write_lock).retain(|&p_id| p_id != pid); // product got minted, don't keep its id in the locks vector, other clients can demand the minting process easily on the next request
                clonedTx1.send(PurchasingStatus::Done).await; // sending done flag as product is not locked any more and the minting is done
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

        get a task that is solved sooner than the other async jobs or tasks 
        since we have only two different async tasks: minting and checking 

        each tokio::spawn() contains an async io task which will be executed in 
        the background by awaiting on them we'll tell the runtime to suspend the
        function execution but don't block the thread during executing other tasks
    */
    tokio::select! {
        // if this branch is selected means the product minting
        // process is not finished or not started to mint yet,
        // if we receive something from the channel we know 100 percent 
        // it's a true flag or a rejecting flag cause we're only sending 
        // true flag to the channel.
        status = rx.recv() => {
            return (status.unwrap(), preceiver);
        },
        // if this branch is selected means the product is inside 
        // the minting process and we should release the lock after
        // it completes and notify the client later, note that the 
        // whole logic of the minting process is inside the mintTask
        // spawned task, mintTask does return nothing!
        _ = mintTask => { // if mintTask was solved then we simply return false and the receiver
            return (PurchasingStatus::Done, preceiver); // during the minting process, if we're here means the minting is done
        },
    }

}