

use crate::*;

/* ------------------------------------------- */
// NODEJS LIKE ASYNC METHOD ORDER OF EXECUTION
/* ------------------------------------------- */
/*  https://lunatic.solutions/blog/rust-without-the-async-hard-part/
    https://rustmagazine.org/issue-4/how-tokio-schedule-tasks/#:~:text=Tokio%20uses%20work%20stealing%20to,other%20workers'%20queues%20to%20execute.&text=In%20the%20above%20figure%2C%20there,are%20purely%20CPU%2Dbound%20tasks
    
    ----------------------------------------------------------------------
              Golang and Rust (goroutines and futures)!
    ==========>>>==========>>>==========>>>==========>>>==========>>>=====
    future objects in Rust are async taks that will be execute in a free 
    thread by the async runtime based on task or work stealing algo.
    in Rust async task in lightweight thread execution is handled by the 
    tokio runtime scheduler. it distributes workloads (tasks) across cpus
    based on work stealing approach (task scheduler algo):
    every worker or lightweight thread has its own queue allows task to be 
    queued in there for future execution which will be told by the runtime 
    to execute which task at when!? actually the runtime also stores all 
    the threads in a queue and then go for their execution based on its algo.
    tokio uses work stealing approach, so when a worker's run queue is empty, 
    it tries to “steal” tasks from other workers' queues to execute
    with tokio::spawn we're telling the runtime: schedule this task
    to be executed outside of the main or current thread. 
    the migration of threads between processors is expensive, as it involves 
    context switch operations. under the stealing paradigm, this phenomenon 
    occurs less frequently, resulting in less overhead.
    Rust has future objects but Go has goroutines in Go there is a default 
    runtime scheduler to execute computational tasks or goroutines but in 
    Rust we should use tokio runtime scheduler to execute async or future 
    object tasks, both of them execute tasks inside a lightweight thread of
    execution in the background allows us to use channels to send the task 
    result to outside of the thread.
    Rust requires more low level control over the execution flow of tasks 
    but Go's design prioritizes ease of use and simplicity over this matter.
    following are the features of future objects in Rust:
        - future trait objects 
        - capture lifetimes 
        - unsized and heap allocated 
        - Box::pin(fut) as separate type 
        - self ref types requires to pin them to breack the cycle and add indirection
        - requires runtime scheduler to execute them
        - async recursive methods requires Box::pin(method)
        - async trait methods due to having supports for generics in gat traits like generic and lifetime
    Rust's type system, including its ownership and borrowing principles, 
    provides strong guarantees about memory safety and concurrency. Rust's 
    async/await and Future abstractions fit naturally within this framework, 
    providing zero-cost abstractions for asynchronous programming.
    it aims for zero-cost abstractions, safety, and performance. its ecosystem 
    and language features, including the async/await model, are designed to 
    give programmers fine-grained control over system resources, fitting 
    well with the systems programming domain.
    to spawn tasks in a lightweight thread of execution the task must get 
    executed in a none blocking manner the Go runtime handle this automatically 
    in the background but in Rust we should put more efforts to handle this 
    manually by creating async tasks which are future objects, the tokio runtime
    however will execute each async task in its lightweight thread in a none 
    blocking way to ensure that there is no blocking process of the way around 
    the current thread. 

    ----------------------------------------------------------------------
              easons that we can't have async recursive?!
    ==========>>>==========>>>==========>>>==========>>>==========>>>=====

    the compiler can't pre determine how many recursive 
    calls you're going to have and can't reserve the perfect
    amount of space to preserve all local variables during 
    the context switching that's why we have to put the 
    the async function on the heap and since future objects
    are self-ref types that move gets broken when they're 
    doing so, we need to pin them into the heap at an stable 
    memory address, cause having future objects as separate types
    requires to pin them into the ram. 
    a context switch is the process of storing the state of a process 
    or thread, so that it can be restored and resume execution at a 
    later point, and then restoring a different, previously saved, state
    
    future objects need to live longer with 'satatic lifetime also they must be send sync so 
    we can move them between threads and scope for future solvation also they need to be pinned 
    into the ram cause they're self-ref types and pointers of self-ref types won't be updated 
    by Rust after moving to break the cycle we need to add some indirection using rc, arc, box, 
    pin, futures are placeholders with a default value which gets solved as soon as the result
    was ready then waker poll the result to update caller
 
    ----------------------------------------------------------------------
                    order of async methods execution
    ==========>>>==========>>>==========>>>==========>>>==========>>>=====

    nodejs has its own runtime by default so there is no need to await on an async method to execute
    it because in Rust futures are lazy they do nothing unless we await on them but this is not the 
    default behaviour in nodejs we can call an async method without putting await behind it:

    async function async_operation() {
        return new Promise((resolve) => {
            setTimeout(() => {
                resolve(42);
            }, 2000); // Simulating a 2-second delay
        });
    }

    async function main() {
        let result = await async_operation();
        console.log("result", result);

        let modified_result = result * 2;
        console.log("modified_result", modified_result);
    }

    main();

    execution of async methods are not purely async we should put them in tokio::spawn
    in the following each tokio::spawn() execute each task asyncly, concurrently and independently 
    in the background without specific ordering, having any disruption in execution other codes and 
    waiting for each other to be finished executing also once the http request received from client 
    the codes get executed asyncly in the bacground, the api body is executed and the response 
    sent to the client even if those async codes are not yet executed or are being exeucted
    
    ----------------------------------------------------------------------
       differences between tokio::spawn() and awaiting on a joinhanlde
    ==========>>>==========>>>==========>>>==========>>>==========>>>=====

    tokio::spawn:
        tokio::spawn is used to spawn a new asynchronous task (future) onto the Tokio runtime without 
        blocking the current task. when you spawn a task using tokio::spawn, it returns a JoinHandle 
        that represents the spawned task. The spawned task runs concurrently with the current task 
        and can execute independently.
    
    await on a JoinHandle:
        when you await on a JoinHandle, you are waiting for the completion of the asynchronous task 
        represented by the JoinHandle. By await-ing on a JoinHandle, you are suspending the current 
        task until the spawned task completes. The result of the JoinHandle future is returned when 
        the spawned task finishes execution.

    Concurrency vs. Waiting:
        tokio::spawn allows you to run tasks concurrently, enabling parallel execution of asynchronous 
        operations. await on a JoinHandle is used to wait for the completion of a specific task before 
        proceeding with the rest of the code.

    ----------------------------------------------------------------------
                    blocking and none blocking execution
    ==========>>>==========>>>==========>>>==========>>>==========>>>=====

    >_ Async runtimes like tokio and async-std manage async tasks. These runtimes use a small number of OS 
    threads to run a large number of tasks. Tasks are lightweight and do not each require their own OS-level 
    stack. Instead, they use a smaller, dynamically managed stack.

    >_ The runtime schedules tasks and polls them for progress. When a task is not ready to make progress 
    (e.g., it is waiting for I/O), it is not polled again until it is ready, this non-blocking, cooperative 
    multitasking approach allows a single OS thread to manage many tasks efficiently.

    >_ Futures are typically stack-allocated, and when they await on other futures, the state of the future 
    is saved in a state machine. This state machine is stored on the heap but is much smaller than a traditional 
    OS thread stack. When a future is polled, it uses the current thread's stack. Once it yields (using await), 
    it frees the stack space.

    >_
    Executor: The async runtime uses an executor to manage and poll tasks. The executor runs on a few OS threads 
        and uses an event loop to drive task execution.
    Wakers: When a task awaits a future, it registers a waker. The waker is notified when the future can make 
        progress, at which point the runtime will poll the task again.
    Task State: The state of each task is managed on the heap in a way that minimizes memory usage compared to 
        having a full OS thread stack per task which is what's currently is handled by the naitive threads.

    when you await on acquiring the lock of the mutex the os can decide to switch to another task that's ready to run.
    await doesn't block the current thread, awaitng allows another thread to be scheduled by the os to be run, awaiting 
    returns a future placeholder which can be used by other parts until the actual value is resolved:
        - The CPU doesn't waste time waiting for get_num() to complete as long as there are other tasks to run.
        - The placeholder acts as a reminder that the final value needs to be plugged in later.
        - while the final answer relies on the resolved value from get_num(), the CPU avoids waiting idly. 
        - The await keyword allows other tasks to take advantage of the waiting time until the asynchronous operation finishes.
        - in the following example even though the CPU doesn't actively wait, await ensures that the info! line runs only 
          after the value from get_num is available. This maintains the order of execution within the same thread:
          async fn get_num() { 32}
          let num = 10 + get_num().await;
          info!("num: {}", num);
          // other code blocks and logics
          // ...
          
    a future object is like a placeholder that need to be await to suspend the function execution until the result 
    gets polled by the waker, this allows other codes get executed and compiled at the same time and there would be 
    no disruption in code order execution later the placeholder gets filled with the solved value.
    none blocking generally means executing each lines of codes without waiting for the task or the codes to 
    completion which prevent other codes and parts from being executed at the same time for example, establishing 
    a TCP connection requires an exchange with a peer over the network, which can take a sizeable amount of time, 
    during this time, the thread is blocked.
    with asynchronous programming, operations that cannot complete immediately are suspended to the background. 
    The thread is not blocked, and can continue running other things. Once the operation completes, the task is 
    unsuspended and continues processing from where it left off, more specificly when you await on an asynchronous 
    operation, the function suspends its execution until the operation completes, but the function itself returns 
    a Future representing the result of the operation.
    when you await on a Future, you can assign the result to a variable or use it directly in the subsequent code,
    the result of the await expression is the resolved value of the Future appeared in form of a placeholder, which 
    you can use in later scopes, this means if you need the result of a future in other async codes or scopes you
    can use its placeholder to do the operations once the suspension gets ended the waker poll the actual value 
    and continues processing where it left off, results in updating the caller state with the solved value, meanwhile 
    other scopes and codes got executed and compiled and are waiting to fill the placeholder with the solved value,
    however thanks to the static type based langs allows other scopes know the exact type of the result of the future
    before it gets solved.
    tokio runtime executes task in the background thread which allows the tasks to run concurrently with other tasks 
    without blocking threads, we can execute other codes without waiting for the task to complete, putting await on 
    the future object ensures that the task is completed. std thread join blocks the current thread until the joined
    thread finishes tokio spawn on the other hand execute async io tasks in a none blocking manner inside a light thread 
    so it’s better not to block in tokio light threads although we can block it like when we’re dealing with mutexes, 
    awaiting on tokio threads won’t block the thread at all.
    we can block in both io light and cpu threads but it would be better to do none blocking async operations in io 
    light threads and heavy blocking computations like dl processing in cpu threads, it's great to use rayon cpu based 
    threadpool for cpu blocking and tokio lightweight threadpool for none blocking operations.
    tokio is a none blocking runtime scheduler and event loop like nodejs, each task falls into a thread from a threadpool, 
    each thread has its own run queue to execute future tasks, since all threads are lightweight blocking inside of them 
    is ok, we could leverage channels or mutex for data atomic syncing between thread or tasks, tokio runtime start scheduling 
    on reaching tasks come with an await. all tokio tasks must be executed in lightweight thread and must not get 
    blocked in there cause it's a none blocking io runtime in some case there might be some io thread waiting for 
    some network connections held by the cpu thread increasing cost of io task execution in lightweight thread.
    blocking operations should generally be avoided in threads that are also handling I/O tasks because it reduces 
    concurrency. For CPU-bound tasks, using a dedicated thread pool or OS threads might be more appropriate to avoid 
    blocking the main event loop. io tasks are future objects that will be executed in lightweight threads cause they 
    involve waiting for external operations to complete they spend most of their time waiting for these operations 
    to complete rather than using the CPU. in io execution the thread contains the tasks sits idle instead of blocking 
    during the wait, use async/await syntax for I/O-bound operations to keep the main thread free for other tasks and 
    avoid blocking in lightweight threads as much as possible. Mutex operations blocks the current thread to prevent 
    other tasks from using the thread or the mutating the data hence it's better to put them inside a separate thread 
    to mutate data asyncly if we want other tasks get executed simultaneously. isolate io threads from cpu threads if 
    the task is like a mathematical computations which requires cpu threads to be blocked separately rather than 
    processing them in io lightweight threads. blocking operations are generally best handled by dedicated CPU threads 
    or thread pools, while non-blocking I/O operations should be managed by lightweight threads (tasks) in an asynchronous 
    runtime like tokio this approach maximizes efficiency and concurrency.
    goroutine is a lightweight tasks executed in a light thread by the go runtime it's like tokio spawn lightweight 
    thread of execution for none blocking async io tasks, Mutex generally blocks the threads to mutate data to prevent 
    other threads from doing so at the same time in comparison with channels it's expensive and costs overhead, we 
    should use tokio mutex in none blocking io light threads and std mutex in a cpu threads, don't use std mutex in io light threads
    lightweight threads none blocking io tasks => tokio::spawn(): file, networking and db operations, streaming over jobq based channels
    cpu threads blocking tasks                 => rayon::spawn(): cryptography, dl and ml logics, mathematical operations
    
    Notes:
    don't block the lightweight thread at all, wait on them until complete the job, used for io processes  
    use channels instead of mutex for atomic syncing cause mutex blocks thread prevent mutating data by other threads at the same time
    use std thread spawn or rayon spawn to spawn heavy computational task into the cpu threads 
    separate io and cpu tasks threads from each other to avoid blocking while we're awaiting for other tasks to complete
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


    /* 
        mutex block the threads to avoid mutating data by other threads at the same time, lock gets released 
        once the mutex goes out of scope:
        atomic syncing with blocking: arc mutex send sync static 
        atomic syncing without blocking: send sync static channels
        we can't move a pointer into the tokio spawn unless the pointer lives statically we should wrap the 
        type around thread safe smart pointers like arc as an atomic immutable reference and mutex as an atomic 
        mutable reference
        atomic syncing and sharing data between lightweight threads of task execution:
        1 - channels (fastest) -> data must be send sync and have valid lifetimes during thread execution or move the entire ownership
        2 - static lazy arc mutex or rwlock (slowest) -> this can be mutated globally across the app at any time
        NOTE - a lightweight thread with its task must be executed in the background and 
               get any data inside of it using channels, there is another way to get the
               data however and is joining the thread to get the data directly without 
               using channels, this way call ask for the thread data directly
               to return the data to the caller.
        NOTE - execute task per lightweight thread then use channels to send data to other threads without
               having race conditions or use mutex to mutate data per only one thread at the same time.
    */
    static DATA: Lazy<std::sync::Arc<tokio::sync::Mutex<String>>> = Lazy::new(|| std::sync::Arc::new(tokio::sync::Mutex::new(String::from(""))));
    let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);
    let safe_rx = std::sync::Arc::new(tokio::sync::Mutex::new(rx));
    
    let cloned_tx = tx.clone();
    let cloned_tx1 = tx.clone();
    
    // sending data to channel in a lightweight thread of the tokio
    // threadpool, doing so is done in the background and we're not 
    // worry about blocking for io since we're using a light thread 
    // and blocking the thread for executing the task is being done
    // in the background by the runtime scheduler.
    tokio::spawn(async move{
        println!("inside the first spawn :::: sending data");
        let resp = String::from("output");
        cloned_tx.send(resp).await;
    });

    let cloned_rx = safe_rx.clone();
    let cloned_rx1 = safe_rx.clone();

    /////////////////////////////////////////////////
    ///////// USE CHANNELS INSTEAD OF MUTEX /////////
    /////////////////////////////////////////////////
    tokio::spawn(async move{
        println!("inside the second spawn :::: receiving data");
        while let Some(mut resp) = cloned_rx.lock().await.recv().await{
            println!("received data from channel: {:?}", resp);
            resp = String::from("muteated_output");
            
            // sending while we're receiving, we'll receive the new changed
            cloned_tx1.send(resp).await; 
        }
    });

    // we can join on each tokio scope to get the thread content directly 
    // without using channels, channels are being used to send data inside
    // the thread to ouside of the thread
    let receiving_task = tokio::spawn(async move{
        while let Some(mut resp) = cloned_rx1.lock().await.recv().await{
            println!("received data from channel: {:?}", resp);
        }
        return String::from("wildonion");
    });
    

    /////////////////////////////////////////////////
    ///////// USE MUTEX INSTEAD OF CHANNELS /////////
    /////////////////////////////////////////////////
    // note that it's better to lock on a data inside 
    // another thread to avoid blocking executions cause
    // locking can block the execution! even if it's async!
    let locking_task = tokio::spawn(async move{
        let mut get_data = DATA.lock().await;
        (*get_data) = String::from("globally_mutated");
    });
    
    // control the execution flow of async tasks
    tokio::select!{
        _ = locking_task => {
            // if the locking task solves earlier do the 
            // following and cancel other branches
            // ...
        },
        _ = receiving_task => {
            // if the receiving task solves earlier do the 
            // following and cancel other branches
            // ...
        }
    }

}

pub async fn test_code_order_exec(){

    let (heavyme_sender, mut heavyme_receiver) = tokio::sync::mpsc::channel::<u128>(1024);
    let (heavyyou_sender, mut heavyyou_receiver) = tokio::sync::mpsc::channel::<String>(1024);

    // every tokio::spawn executes in the background thus we din't 
    // await on each joinhandle returned by the tokio::spawn() instead
    // we've used channels to send and receive each async task result
    tokio::spawn(async move{
        while let Some(data) = heavyyou_receiver.recv().await{
            info!("received heavyyou data: {:?}", data);
        }
    });

    async fn heavyme() -> u128{
        let mut sum = 0;
        for i in 0..10000000000{
            sum += i;
        }
        sum
    }

    tokio::spawn(async move{
        while let Some(data) = heavyme_receiver.recv().await{
            info!("received heavyme data: {:?}", data);
        }
    });

    tokio::spawn(async move{
        let res = heavyyou().await;
        heavyyou_sender.send(res).await;
    });

    async fn heavyyou() -> String{
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        String::from("woke up after 2 secs")
    }

    tokio::spawn(async move{
        sleep4().await;
    });

    async fn sleep2(){
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        println!("wake up after 2")
    }

    async fn sleep4(){
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
        println!("wake up after 4");
    }

    tokio::spawn(async move{
        sleep2().await;
    });
    
}