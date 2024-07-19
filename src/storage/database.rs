


use crate::*;
use super::engine::*;
use super::engine::Storage1 as Database; 




/* 
    Storage1 contains the engine, event buffer, lock and the background worker fields
        - engine is used to load and store events
        - event buffer is the buffer of all current events 
        - lock is used when the db is busy which is either the worker is working on buffer or there are unprocessed events inside it
        - background worker thread is used to execute the save events task constantly in the background, it can be tokio::spawn instead of storing a separate worker as feild
    typically any db must contains an engine, a background worker thread to execute 
    its task in the background thread and a mutex or lock to lock the db when its 
    busy working with events. 
*/
impl Database{

    pub fn connect() -> std::io::Result<std::sync::Arc<Database>>{ // returns a shareable db instance

        todo!()

    }

    // call this to start background worker, we'll execute this inside tokio::spawn()
    // it basically locks the db every 1 second to get the buffer of events then store
    // all of them by calling the save_events() method on the engine trait object.
    async fn worker(&self){

        // since this method will be executed in a tokio::spawn() 
        // hence we are using a loop in here with an 1 second interval
        // to save events constantly as long as the app is running.

    }

    pub fn busy(&self){

    }

    pub fn push_events(&self){

    }

    pub async fn load_events(&self){

    }

}

// since we have background worker thread it mus be dropped when 
// the database is dropped 
impl Drop for Database{
    fn drop(&mut self) {
        if let Ok(fut) = self.background_worker_thread.lock(){
            fut.abort() // abort the future object or the task from executing
        }
    }
}