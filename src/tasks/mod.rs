

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
    J: std::future::Future + Send + Sync + 'static,
    J::Output: Send + Sync + 'static
    {
    pub status: TaskStatus,
    pub name: String, // like send_mail task 
    pub job: J, // use Box::pin(job) to pin it into the ram and call it; the function that needs to get executed 
    pub sender: tokio::sync::mpsc::Sender<S>, // use this to send the result of the task into the channel
}


#[derive(Clone, Debug, Default)]
pub enum TaskStatus{
    #[default]
    Executed,
    Rejected
}

impl<J: std::future::Future + Send + Sync + 'static, S> Task<J, S> 
    where J::Output: Send + Sync + 'static{

    // spawn an async task inside tokio threads upon tokio scheduler
    pub async fn spawn(&self){
        self.execute(String::from("stringified task")).await;
    }

    pub async fn none_assoc_spawn<O: Send + Sync + 'static>(
        fut: impl std::future::Future<Output = O> + Send + Sync + 'static
    ){
        tokio::spawn(fut);
    }
}

impl<J: std::future::Future + Send + Sync + 'static, S> TaskExt<String> for Task<J, S>
    where J::Output: Send + Sync + 'static{
    
    type State = String;
    type Task = Self;
    
    async fn execute(&self, t: String) {

    }

}