

use crate::*;
use interfaces::task::TaskExt;
use types::Job;

// async tasks or jobs that will be executed in 
// the background inside a lightweight thread of 
// execution using tokio::spawn() task scheduler 
// and jobq based channels; actor workers will run 
// these in their own execution context.


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
    pub sender: tokio::sync::mpsc::Sender<S>, // use this to send the result of the task into the channel
    pub worker: tokio::sync::Mutex<tokio::task::JoinHandle<O>>, // execute the task inside the background worker
    pub lock: tokio::sync::Mutex<()>, // the task itself is locked and can't be used by other threads
}

impl<O, J: std::future::Future<Output = O> + Send + Sync + 'static + Clone, S> Task1<J, S, O> 
    where O: Send + Sync + 'static{

    pub async fn new(job: J, sender: tokio::sync::mpsc::Sender<S>) -> Self{
        let task = Self{
            status: TaskStatus::Initializing,
            name: String::from("KJHS923"),
            job: job.clone(),
            sender,
            worker: { // this is the worker that can execute the task inside of itself, it's basically a lightweight thread
                tokio::sync::Mutex::new(
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

    pub async fn switch_task(&mut self, taks: J){

        let mut get_worker = self.worker.lock().await;
        (*get_worker) = tokio::spawn(taks);

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