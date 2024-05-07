


// https://crates.io/crates/testcontainers
// https://nexte.st/


/* ------------------------------------------- */
// NODEJS LIKE ASYNC METHOD ORDER OF EXECUTION
/* ------------------------------------------- */
// future objects need to live longer with 'satatic lifetime also they must be send sync so 
// we can move them between threads and scope for future solvation also they need to be pinned 
// into the ram cause they're self-ref types and pointers of self-ref types won't be updated 
// by Rust after moving to break the cycle we need to add some indirection using rc, arc, box, 
// pin, futures are placeholders with a default value which gets solved as soon as the result
// was ready then waker poll the result to update caller
/* 
    ----------------------------------------------------------------------
                    order of async methods execution

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
*/

use std::sync::atomic::{AtomicBool, Ordering};

use crate::*;

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