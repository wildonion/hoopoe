
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

use crate::actors::consumers::notif::ConsumeNotif;
use crate::actors::producers::notif::ProduceNotif;
use crate::consts::APP_NAME;
use std::env;
use std::net::SocketAddr;
use actix_web::middleware::Logger;
use consts::SERVERS;
use clap::Parser;
use dotenv::dotenv;
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
use hyper::StatusCode;
use uuid::Uuid;
use log::{info, error};
use actix_redis::{Command, RedisActor, resp_array, RespValue};
use actix::{Actor, StreamHandler};
use actix_web_actors::ws;
use actix_cors::Cors;
use actix_web::{App, Error, web, cookie::{self, Cookie, time::Duration, time::OffsetDateTime}, 
                web::Data, http::header, HttpRequest, middleware,
                HttpServer, Responder, HttpResponse, get, post, ResponseError};
use actix_multipart::Multipart;
use env_logger::Env;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
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


// all macros in other crates will be loaded in here 
// so accessing them from other crates and modules is
// like use crate::macro_name;

mod actors;
mod plugins;
mod s3;
mod config;
mod consts;
mod lockers;
mod error;
mod server;
mod types;
mod appstate;
mod services;
mod requests;
mod apis;
mod extractors;
mod cli;
mod tests;
mod models;
mod entities;



/* ******************************* IMPORTANT *******************************
    can't start tokio stuffs or actix stuffs like actors inside the context
    of actix or tokio runtime, so we can't have a pure TCP server with 
    actix_web::main HTTP server with tokio::main. 
 ************************************************************************* */
#[actix_web::main]
async fn main() -> std::io::Result<()>{

    /* -ˋˏ✄┈┈┈┈ logging
        >_
    */
    let args = cli::ServerKind::parse();
    dotenv::dotenv().expect("expected .env file be there!");
    env::set_var("RUST_LOG", "trace");
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    // env::set_var("RUST_LOG", "actix_web=debug");

    /* -ˋˏ✄┈┈┈┈ initializing appstate actor workers
        >_ run actor workers, app_state contains the whole app data 
        which will be used globally during the execution of the app
    */
    let app_state = appstate::AppState::init().await;


    /* -ˋˏ✄┈┈┈┈ migrating on startup
        >_ ORM checks on its own that the db is up to create the pool connection
        it won't start the app if the db is off, makes sure you've started
        the pg db server
    */
    let connection = sea_orm::Database::connect(
        &app_state.config.as_ref().unwrap().vars.DATABASE_URL
    ).await.unwrap();
    let fresh = args.fresh;
    if fresh{
        Migrator::fresh(&connection).await.unwrap();
    } else{
        Migrator::up(&connection, None).await.unwrap(); // executing database tasks like creating tables on startup
    }
            
    /* -ˋˏ✄┈┈┈┈ bootstrapping http server
        >_ 
    */
    info!("➔ 🚀 {} HTTP+WebSocket server has launched from [{}:{}] at {}", 
        APP_NAME, app_state.config.as_ref().unwrap().vars.HOST, 
        app_state.config.as_ref().unwrap().vars.HTTP_PORT.parse::<u16>().unwrap(), 
        chrono::Local::now().naive_local());

    bootsteap_http!{
        app_state.clone(),
    }
        

}