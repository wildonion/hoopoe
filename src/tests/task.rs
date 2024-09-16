


use std::{ops::DerefMut, sync::atomic::{AtomicU64, AtomicUsize}};
use crate::*;
use interfaces::task::TaskExt;
use tracing::span_enabled;
use types::Job;



/* 
    async tasks or jobs that will be executed in the background inside a lightweight thread of 
    execution using tokio::spawn() task scheduler and jobq based channels; actor workers will run 
    these tasks in their own execution context like what i've simulated here.
    typically any instance of the Task which is kina actor must contains:
        - the future job itself
        - the sender to send the result of executed task to the channel for using outside of the thread
        - a thread safe (Mutex) worker as the background worker thread to execute the future job in it
        - a locker to lock the task when it's executing the task 
    the task can be awaited to complete its future or aborted during the dropping process of the
    task instance, tokio::spawn() is the backbone of each background worker, it gets a future and 
    move it into a lightweight thread of execution. 
    lock the instance using the lock field to check that if we're doing some heavy process or not
    then in switching the task or doing other heavy process check the lock that if the instance 
    is already locked or not also we should lock the worker if we want to execute something in the 
    background worker of the instance thread to tell obj caller that the worker is busy rn. 

    more details:
    locker, threadpool, worker, future io task, eventloop, 
    sender, signal condvar, job tree, dep inj future job:
        - cron scheduler method
        - execute in worker method 
        - receive from eventloop and exec in threadpool method
        - instance locker method
    
    future object
    job tree to push the job into the current tree
    sender to broadcast or publish some data to a channel
    an eventloop to receive a data from the channel or the queue to execute it in the background worker thread
    background worker to run the job
    locker to lock the task instance when a task is being executed 
    worker thread of type joinHandle to execute task or job of type async io or cpu tasks 
    threadpool to execute each task when receives them from mpsc recevier eventloop 
    atomic syncing with channels and mutex 
*/
// #[derive(Debug)] // don't implement this cause Pin doesn't implement Debug
pub struct Task<J: std::future::Future<Output = O>, S, O> where // J is a Future object and must be executed with Box::pin(job);
    J: std::future::Future + Send + Sync + 'static + Clone,
    O: Send + Sync + 'static,
    J::Output: Send + Sync + 'static
{
    pub status: TaskStatus,
    pub id: String,
    pub name: String, // like send_mail task 
    /* 
        Pin is a wrapper around some kind of pointer Ptr which makes 
        that pointer "pin" its pointee value in place, thus preventing 
        the value referenced by that pointer from being moved or otherwise 
        invalidated at that place in memory unless it implements Unpin
        which means tha type type doesn't require to be pinned into 
        the ram, self ref types must implement !Unpin or must be pinned
    */
    pub dep_injection_fut_obj: std::pin::Pin<Box<dyn std::future::Future<Output = O> + Send + Sync + 'static>>, // a future as separate type to move between scopes
    pub job: J, 
    pub job_tree: Vec<Task<J, S, O>>,
    pub sender: tokio::sync::mpsc::Sender<S>, // use this to send the result of the task into the channel to share between other lightweight thread workers
    pub eventloop_queue: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<S>>>, // use this as eventloop to execute tasks as they're coming from the channel in the background worker thread
    pub pool: Vec<tokio::task::JoinHandle<O>>,
    pub worker: std::sync::Mutex<tokio::task::JoinHandle<O>>, // execute the task inside the background worker, this is a thread which is safe to be mutated in other threads 
    pub lock: std::sync::Mutex<()>, // the task itself is locked and can't be used by other threads
}

// thread safe eventloop and queue: arc mutex vec T vs arc mutex receiver T
pub struct QueueAndEventLoop<T: Clone + Send + Sync + 'static>{
    pub eventloop: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<T>>>,
    pub queue: std::sync::Arc<tokio::sync::Mutex<Vec<T>>>,
}


impl<O, J: std::future::Future<Output = O> + Send + Sync + 'static + Clone, S: Sync + Send + 'static> 
    Task<J, S, O> 
    where O: std::any::Any + Send + Sync + 'static{

    pub async fn new(job: J, num_threads: usize,
        sender: tokio::sync::mpsc::Sender<S>, 
        eventloop_queue: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<S>>>, 
        fut_output: O) -> Self{

        // sender and receiver

        let task = Self{
            status: TaskStatus::Initializing,
            id: Uuid::new_v4().to_string(),
            name: String::from("KJHS923"),
            job: job.clone(),
            dep_injection_fut_obj: Box::pin(async move{ fut_output }), // pinning the future into the ram with the output of type O
            sender,
            pool: {
                (0..num_threads)
                    .map(|_| tokio::spawn(job.clone()))
                    .collect::<Vec<tokio::task::JoinHandle<O>>>()
            },
            eventloop_queue,
            job_tree: vec![],
            worker: { // this is the worker that can execute the task inside of itself, it's basically a lightweight thread
                std::sync::Mutex::new( // lock the worker
                    tokio::spawn(job)
                )
            },
            lock: Default::default(),
        };

        task 

    }

    pub async fn send(&self, d: S){
        let sender = self.sender.clone();
        sender.send(d).await;
    }

    pub fn is_busy(&mut self) -> bool{
        self.lock.try_lock().is_err() // is_err() can be either true or false, trying to acquire the lock
    }

    pub async fn spawn(&self){

        let job = self.job.clone();
        tokio::spawn(job);
    }

    // method to execute the job in the task worker
    pub async fn execute(&mut self){

        // wailt until the lock gets freed cause we're pushing tasks into the tree 
        // if we slide down into the while loop means the method returns true which
        // means the lock couldn't get acquired
        while self.is_busy(){ 
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }

        let t = self.job.clone(); // clone to prevent from moving
        let mut get_worker = self.worker.try_lock().unwrap(); // lock the worker
        (*get_worker) = tokio::spawn(t);
    }

    pub async fn switch_task(&mut self, task: J){
        
        // wailt until the lock gets freed cause we're pushing tasks into the tree 
        // if we slide down into the while loop means the method returns true which
        // means the lock couldn't get acquired
        while self.is_busy(){ 
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }

        let mut get_worker = self.worker.lock().unwrap();
        (*get_worker) = tokio::spawn(task);

    }

    pub fn push(mut self, tasks: Vec<Task<J, S, O>>) -> Task<J, S, O>{
        
        // lock the instance to push tasks into the tree
        self.lock.lock().unwrap();
        self.job_tree.extend(tasks);
        self
    }

    // task lifecycles
    pub fn halt(&mut self){
        self.status = TaskStatus::Hanlted;
    }

    /* -------------------------------------------------------------
        since futures are object safe trait hence they have all traits 
        features we can pass them to the functions in an static or dynamic 
        dispatch way using Arc or Box or impl Future or event as the return 
        type of a closure trait method:
            returning reference or box to dyn trait by casting the type who impls the trait into the trait 
            dep injection object safe trait using & and smart pointers dyn
            future as generic in return type of closure or function or pinning its box
            std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync + 'static>
            Arc<dyn Fn() -> R + Send + Sync + 'static> where R: std::future::Future<Output = ()> + Send + Sync + 'static
            Box<dyn Fn() -> R + Send + Sync + 'static> where R: std::future::Future<Output = ()> + Send + Sync + 'static
            Arc<Mutex<dyn Fn() -> R + Send + Sync + 'static>> where R: std::future::Future<Output = ()> + Send + Sync + 'static
            F: std::future::Future<Output = ()> + Send + Sync + 'static
            param: impl std::future::Future<Output = ()> + Send + Sync + 'static

        NOTE: mutex requires the type to be Sized and since traits are 
        not sized at compile time we should annotate them with dyn keyword
        and put them behind a pointer with valid lifetime or Box and Arc smart pointers
        so for the mutexed_job we must wrap the whole mutex inside an Arc or annotate it
        with something like &'valid tokio::sync::Mutex<dyn Fn() -> R + Send + Sync + 'static>
        the reason is that Mutex is a guard and not an smart pointer which can hanlde 
        an automatic pointer with lifetime 
    */
    pub async fn cron_scheduler<F, R>(&mut self, 
        boxed_job: Box<dyn Fn() -> R + Send + Sync + 'static>,
        mutexed_job: std::sync::Arc<tokio::sync::Mutex<dyn Fn() -> R + Send + Sync + 'static>>,
        arced_job: std::sync::Arc<dyn Fn() -> R + Send + Sync + 'static>) 
    where
        R: std::future::Future<Output = O> + Send + Sync + 'static{

            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            tokio::spawn(async move{
                loop{
                    interval.tick().await;  
                    arced_job().await;
                }
            });
    }

}

// once the task gets dropped drop any incomplete futures inside the worker 
impl<O, J: std::future::Future<Output = O> + Send + Sync + 'static + Clone, S> Drop for Task<J, S, O> where 
    O: Send + Sync + 'static{
    fn drop(&mut self) { // use std::sync::Mutex instead of tokio cause drop() method is not async 
        if let Ok(job) = self.worker.lock(){
            job.abort(); // abort the current future inside the joinhandle
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum TaskStatus{
    #[default]
    Initializing,
    Executed,
    Hanlted
}

type FutureTraitObject<O: Send + Sync + 'static> = std::pin::Pin<Box<dyn std::future::Future<Output = O> + Send + Sync + 'static>>;
impl<J: std::future::Future<Output = O> + Send + Sync + 'static + Clone, 
     S: Send + Sync + 'static, O: Send + Sync + 'static> 
    Task<J, S, O> 
    where J::Output: Send + Sync + 'static{

    pub async fn spawn_task< // none assoc method
        F: std::future::Future<Output = O> + Send + Sync + 'static,
        R: Send + Sync + 'static,
        V: FnOnce(O) -> R + Send + Sync + 'static
        >(
            fut1: F,
            pinned_fut: FutureTraitObject<O>, // future as trait object
            input: O,
            fut: impl std::future::Future<Output = O> + Send + Sync + 'static,
            func: V
    ){
        tokio::spawn(fut);
        tokio::spawn(async move{ func(input) });
    }

}

impl<O: Send + Sync + 'static, J: std::future::Future<Output = O> + 
     Send + Sync + 'static + Clone, S> TaskExt<String> for Task<J, S, O>
    where 
        J::Output: Send + Sync + 'static,
        <J as std::future::Future>::Output: Send + Sync + 'static{
    
    type State = String;
    type Task = Self;
    
    async fn execute_this(&self, t: String) {
         
        let this = self.clone();
        let job = this.job.clone();
        tokio::spawn(job); // job is of type future, we're executing it inside another free thread

    }
}

/* ----------------------------------------------------- */
//          a thread safe task tree executor
/* ----------------------------------------------------- 
|   use smart pointers to break the cycle of self ref 
|   types, in here we're creating a node for the entire 
|   task tree which contains a reference to the itself
|   it gets executed in BFS order.
|
*/

pub struct TaskTree<
    J: std::future::Future<Output = O> + Send + Sync + 'static + Clone, 
    S, O: Send + Sync + 'static>{
    // wrap it around mutex to share the task between threads cause we
    // want to execute the task in a light thread without blocking so 
    // we need to move the reference of the task into the thread which 
    // can be done via mutex since it's an smart pointer for sharing data
    // safely between threads
    pub task: tokio::sync::Mutex<Task<J, S, O>>, 
    pub weight: std::sync::atomic::AtomicU8,
    pub parent: std::sync::Arc<TaskTree<J, S, O>>, // the parent itself
    pub children: std::sync::Mutex<Vec<std::sync::Arc<TaskTree<J, S, O>>>> // vector of children
}

impl<J: std::future::Future<Output = O> + Send + Sync + 'static + Clone + std::fmt::Debug, 
    S: std::fmt::Debug + Send + Sync + 'static, O: std::fmt::Debug + Send + Sync + 'static> 
    TaskTree<J, S, O>{
    
    // execute all tasks in bfs order in none binary tree
    pub fn execute_all_tasks(&mut self, root: std::sync::Arc<TaskTree<J, S, O>>){
        let mut queue = vec![root]; 
        while !queue.is_empty(){
            let get_node = queue.pop(); // pop the child out
            if get_node.is_some(){
                let node = get_node.unwrap();
                let cloned_node = node.clone();
                
                // executing the task in the background light thread in a none 
                // blocking io manner, we tried to acquire the lock of the value 
                // in a separate thread to avoid blocking the current thread for doing so
                tokio::spawn(async move{
                    let mut task = cloned_node.task.lock().await;
                    println!("[*] executing the task with id: {:?}", task.id);
                    // this method contains a locking process on the task itself so it's better
                    // to execute it in a separate light io thread
                    task.execute().await;
                });

                let get_children = node.children.try_lock().unwrap();
                let children = get_children.to_vec();
                for child in children{
                    queue.push(child);
                }
            }
        }
    }

    pub fn push_task(&mut self, child: std::sync::Arc<TaskTree<J, S, O>>){
        let mut get_children = self.children.try_lock().unwrap();
        (*get_children).push(child);
    }

    pub fn pop_task(&mut self) -> Option<std::sync::Arc<TaskTree<J, S, O>>>{
        let mut get_children = self.children.try_lock().unwrap();
        let poped_task = (*get_children).pop();
        poped_task
    }

}