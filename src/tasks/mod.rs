

use crate::*;
use interfaces::task::TaskExt;
use types::Job;



/* 
    async tasks or jobs that will be executed in the background inside a lightweight thread of 
    execution using tokio::spawn() task scheduler and jobq based channels; actor workers will run 
    these tasks in their own execution context.
    typically any instance of the Task must contains:
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
*/
#[derive(Clone, Debug)]
pub struct Task<J, S> where // J is a Future object and must be executed with Box::pin(job);
    J: std::future::Future + Send + Sync + 'static + Clone,
    J::Output: Send + Sync + 'static
{
    pub status: TaskStatus,
    pub name: String, // like send_mail task 
    pub job: J, // use Box::pin(job) to pin it into the ram and call it; the function that needs to get executed 
    pub sender: tokio::sync::mpsc::Sender<S>, // use this to send the result of the task into the channel
}

// future or task, sender, background worker and lock 
pub struct Task1<J: std::future::Future<Output = O>, S, O> where // J is a Future object and must be executed with Box::pin(job);
    J: std::future::Future + Send + Sync + 'static + Clone,
    O: Send + Sync + 'static,
    J::Output: Send + Sync + 'static
{
    pub status: TaskStatus,
    pub name: String, // like send_mail task 
    pub job: J, // use Box::pin(job) to pin it into the ram and call it; the function that needs to get executed 
    pub job_tree: Vec<Task1<J, S, O>>,
    pub sender: tokio::sync::mpsc::Sender<S>, // use this to send the result of the task into the channel
    pub worker: std::sync::Mutex<tokio::task::JoinHandle<O>>, // execute the task inside the background worker, this is a thread which is safe to be mutated in other threads 
    pub lock: std::sync::Mutex<()>, // the task itself is locked and can't be used by other threads
}

impl<O, J: std::future::Future<Output = O> + Send + Sync + 'static + Clone, S> Task1<J, S, O> 
    where O: Send + Sync + 'static{

    pub async fn new(job: J, sender: tokio::sync::mpsc::Sender<S>) -> Self{
        let task = Self{
            status: TaskStatus::Initializing,
            name: String::from("KJHS923"),
            job: job.clone(),
            sender,
            job_tree: vec![],
            worker: { // this is the worker that can execute the task inside of itself, it's basically a lightweight thread
                std::sync::Mutex::new(
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

    pub async fn switch_task(&mut self, taks: J){
        
        // wailt until the lock gets freed cause we're pushing tasks into the tree 
        // if we slide down into the while loop means the method returns true which
        // means the lock couldn't get acquired
        while self.is_busy(){ 
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }

        let mut get_worker = self.worker.lock().unwrap();
        (*get_worker) = tokio::spawn(taks);

    }

    pub fn push(mut self, tasks: Vec<Task1<J, S, O>>) -> Task1<J, S, O>{
        
        // lock the instance to push tasks into the tree
        self.lock.lock().unwrap();
        self.job_tree.extend(tasks);
        self
    }

    // task lifecycles
    pub fn halt(&mut self){
        self.status = TaskStatus::Hanlted;
    }

}

// once the task gets dropped drop any incomplete futures inside the worker 
impl<O, J: std::future::Future<Output = O> + Send + Sync + 'static + Clone, S> Drop for Task1<J, S, O> where 
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
impl<J: std::future::Future + Send + Sync + 'static + Clone, S> Task<J, S> 
    where J::Output: Send + Sync + 'static{

    // spawn an async task inside tokio threads upon tokio scheduler
    pub async fn spawn(&self){
        self.execute(String::from("stringified task")).await;
    }

    pub async fn spawn_task< // none assoc method
        O: Send + Sync + 'static, R: Send + Sync + 'static,
        F: std::future::Future<Output = O> + Send + Sync + 'static,
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

impl<J: std::future::Future<Output: Send + Sync + 'static> + 
     Send + Sync + 'static + Clone, S> TaskExt<String> for Task<J, S>
    where 
        J::Output: Send + Sync + 'static, 
        <J as std::future::Future>::Output: Send + Sync + 'static{
    
    type State = String;
    type Task = Self;
    
    async fn execute(&self, t: String) {
        
        let this = self.clone();
        let job = this.clone().job.clone();
        tokio::spawn(job); // job is of type future, we're executing it inside another free thread

    }
}