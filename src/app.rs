


/*

Coded by


 █     █░ ██▓ ██▓    ▓█████▄  ▒█████   ███▄    █  ██▓ ▒█████   ███▄    █ 
▓█░ █ ░█░▓██▒▓██▒    ▒██▀ ██▌▒██▒  ██▒ ██ ▀█   █ ▓██▒▒██▒  ██▒ ██ ▀█   █ 
▒█░ █ ░█ ▒██▒▒██░    ░██   █▌▒██░  ██▒▓██  ▀█ ██▒▒██▒▒██░  ██▒▓██  ▀█ ██▒
░█░ █ ░█ ░██░▒██░    ░▓█▄   ▌▒██   ██░▓██▒  ▐▌██▒░██░▒██   ██░▓██▒  ▐▌██▒
░░██▒██▓ ░██░░██████▒░▒████▓ ░ ████▓▒░▒██░   ▓██░░██░░ ████▓▒░▒██░   ▓██░
░ ▓░▒ ▒  ░▓  ░ ▒░▓  ░ ▒▒▓  ▒ ░ ▒░▒░▒░ ░ ▒░   ▒ ▒ ░▓  ░ ▒░▒░▒░ ░ ▒░   ▒ ▒ 
  ▒ ░ ░   ▒ ░░ ░ ▒  ░ ░ ▒  ▒   ░ ▒ ▒░ ░ ░░   ░ ▒░ ▒ ░  ░ ▒ ▒░ ░ ░░   ░ ▒░
  ░   ░   ▒ ░  ░ ░    ░ ░  ░ ░ ░ ░ ▒     ░   ░ ░  ▒ ░░ ░ ░ ▒     ░   ░ ░ 
    ░     ░      ░  ░   ░        ░ ░           ░  ░      ░ ░           ░ 
                      ░                                                  


    -----------------------------------------------------------------
   |          NOTE ON CODE ORDER EXECUTION OF ASYNC METHODS
   |-----------------------------------------------------------------
   | in rust the order of execution is not async by default but rather it's thread safe 
   | and without having race conditions due to its rules of mutable and immutable pointers 
   | of types although if there might be async methods but it's not expected that they must 
   | be executed asyncly, the early one gets executed first and then the second one goes, 
   | an example of that would be calling async_method_one() method with async operations 
   | inside, and other_async_method_two() method, both of them are async however, but the 
   | code waits till all the async operations inside the first one get executed then run the 
   | second one, this gets criticized if we have some delay and sleep methods inside the first 
   | one which gets us into trouble with the whole process of code order execution if we don't 
   | want to have disruption in their execution, though in some cases it's needed to have this 
   | logic but in here it would be a bad idea, the solution to this is running both of them 
   | asyncly in their own seprate threadpool which can be done by putting each of them inside 
   | tokio::spawn() in this case there would be no disruption in their order execution at all 
   | and we'd have a fully async execution of methods in the background.
   | to catch any result data inside the tokio::spawn() we would have to use mpsc channel to
   | send the data to the channel inside the tokio::spawn() and receive it outside of tokio
   | scope and do the rest of the logics with that.
   | notice of putting .await after async task call, it consumes the future objcet by pinning
   | it into the ram for future solvation also it suspend the function execution until the future
   | gets compeleted allows other parts of the app get executed without having any disruption,
   | later once the future has completed waker sends the value back to the caller to update the 
   | state and its value, this behaviour is called none blocking completion but based on the 
   | beginning notes the order of async task execution is not async by itself and if we have 
   | async_task1 first followed by async_task2 the order of executing them is regular and it's 
   | not pure async like goroutines in Go or event loop in NodeJS, to achieve this we should spawn 
   | them insie tokio::spawn which runs all the futures in the background like what goroutines 
   | and NodeJS event loop do
   |
   | conclusion: 
   | use tokio::spawn() to execute any async task in the background without having
   | any disruption in other order execution of async methods.
   | 


    -----------------------------------------------------------------
   |                  ACTIX RUNTIME VS TOKIO RUNTIME
   |-----------------------------------------------------------------
   | tokio runtime is an async task scheduler mainly schedule the time of execution of async tasks in the 
   | whole app, to use actix actors, http and ws we need to add #[actix_web::main] on top of main method 
   | but this doesn't mean we can't use the tokio::spawn() to execute task in the background putting #[tokio::main] 
   | doesn't allow us to run the actix tasks in actix runtime itself, note that we should avoid starting 
   | any actor in the background inside tokio::spawn otherwise we'll face the error of `spawn_local` called 
   | from outside of a `task::LocalSet` which says that we can't start an actor inside the tokio runtime 
   | instead we should use the actix runtime and start the actor in the context of actix runtime itself 
   | where there is no #[tokio::main] on top of main method, good to know that #[tokio::main] simplifies 
   | the sentup and execution of async tasks within the tokio runtime on the other hans tokio::spawn is 
   | used for manually spawning, managing and scheduling async task in the background you need to handle 
   | task lifecycles, cancellation, and coordination between tasks explicitly.
   |

*/

use crate::constants::APP_NAME;
use std::env;
use std::net::SocketAddr;
use clap::Parser;
use dotenv::dotenv;
use indexmap::IndexMap;
use models::event::NotifData;
use salvo::http::response;
use serde_json::Value;
use workers::notif::{self, NotifBrokerActor};
use std::io::BufWriter;
use std::str::FromStr;
use std::{fs::OpenOptions, io::BufReader};
use rand::Rng;
use rand::random;
use sha2::{Digest, Sha256};
use redis::Client as RedisClient;
use redis::AsyncCommands; // this trait is required to be imported in here to call set() methods on the cluster connection
use redis::RedisResult;
use redis::Commands;
use redis_async::client::{self, PubsubConnection, ConnectionBuilder};
use redis::RedisError;
use uuid::Uuid;
use log::{info, error};
use env_logger::Env;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use std::fmt::Write;
use tokio::io::AsyncReadExt;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt};
use tokio::time::Duration as TokioDuration;
use chrono::Utc;
use std::time::Instant;
use std::collections::HashSet;
use rand::rngs::ThreadRng;
use futures::StreamExt; /* is required to call the next() method on the streams */
use once_cell::sync::Lazy;
use std::rc::Weak;
use tokio::sync::RwLock;
use migration::{Migrator, MigratorTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use salvo::Router;
use salvo::prelude::*;
use tracing::subscriber;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use serde::{Serialize, Deserialize};
use salvo::logging::Logger;
use salvo::oapi::extract::*;
use crate::server::HoopoeServer;

mod server;
mod apis;
mod models;
mod types;
mod helpers;
mod tests;
mod constants;
mod requests;
mod entities;
mod config;
mod cli;
mod tasks;
mod interfaces;
mod context;
mod storage;
mod workers;
mod lockers;
mod middlewares;
mod error;
mod routers;



/* -------------
  by using tokio runtime as the async task scheduler we can 
  spawn async task in tokio lightweight threads, the runtime
  schedule the execution time automatically so handling each 
  socket inside a tokio lightweight thread is a great logic
  for handling each async api concurrently.
*/
#[tokio::main]
async fn main(){

    /* ------------- 
      new() method accepts the address and the domain to add 
      ssl using let's encrypt, so if you want to make an 
      internal ssl just pass your app domain name.
    */
    let domain = Some(constants::APP_DOMAIN.to_string());
    let mut app = HoopoeServer::new(None).await;
    
    /* ------------- 
      configure the app instance, initializing and building
      logs, app context, routers, the main service, apply 
      migrations finally run the app
    */
    app.initLog(); // trace the logs
    app.buildAppContext().await; // build the entire app context (actors, configs and shared data across routers)
    app.buildRouter(); // build the app router from apis + swagger ui 
    
    app.buildService(); // build salvo service
    app.applyMigrations().await; // execute db migration files
    app.run().await; // run server; after this call we can't use app since the run() method isn't &self and takes the ownership of the app instance

}