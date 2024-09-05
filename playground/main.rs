

use deadpool_lapin::lapin::types::generation;
use rand::thread_rng;
use rand::seq::SliceRandom;
use std::{collections::HashMap, sync::atomic::AtomicUsize};
use std::net::SocketAddr;
use deadpool_redis::redis::{AsyncCommands, RedisResult};
use futures::sink::Buffer;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};
use tokio::net::TcpStream;
use deadpool_redis::{Config as DeadpoolRedisConfig, Runtime as DeadPoolRedisRuntime};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// store users along with their tcp streaming channels
pub static ID_TRACKER: AtomicUsize = AtomicUsize::new(1);
pub static USERS_TCP_STREAM: Lazy<Arc<Mutex<HashMap<usize, 
    (std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<String>>>, 
     tokio::sync::mpsc::Sender<String>)>>>> =
    Lazy::new(|| {
        Arc::new(Mutex::new(
            HashMap::new()
        ))
    }
);
pub static ONLINE_USERS: Lazy<Arc<Mutex<HashMap<String, usize>>>> = 
    Lazy::new(|| {
        let users = HashMap::default();
        Arc::new(Mutex::new(
            users
        ))
    }
);



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>{

    let redis_password = "";
    let redis_host = "";
    let redis_port = "";
    let redis_username = "";
    let redis_conn_url = if !redis_password.is_empty(){
        format!("redis://:{}@{}:{}", redis_password, redis_host, redis_port)
    } else if !redis_password.is_empty() && !redis_username.is_empty(){
        format!("redis://{}:{}@{}:{}", redis_username, redis_password, redis_host, redis_port)
    } else{
        format!("redis://{}:{}", redis_host, redis_port)
    };


    let redis_pool_cfg = DeadpoolRedisConfig::from_url(&redis_conn_url);
    let redis_pool = redis_pool_cfg.create_pool(Some(DeadPoolRedisRuntime::Tokio1)).unwrap(); 
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8753").await.unwrap();
    tokio::spawn(async move{
        while let Ok((mut stream, addr)) = listener.accept().await{

            let get_id_tracker = ID_TRACKER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            let online_users = ONLINE_USERS.clone();
            let mut get_online_users = online_users.lock().await;
            
            // try to find an existing user with this address
            // otherwise insert it into the map
            *get_online_users.entry(
                addr.to_string()
            ).and_modify(|v| { *v; } /* keep the old id */ )
            .or_insert(get_id_tracker);
        
            let cloned_redis_pool = redis_pool.clone();
            // try to connect the user to a random online one
            tokio::spawn(async move{
                let stream = std::sync::Arc::new(tokio::sync::Mutex::new(stream));
                // execute the connect logic inside another light io thread since 
                // the logic contains some async locking mechanism as well as io processing
                // it's better not to suspend any async function for the locking process 
                // and gets done in a separate thread
                connectMe(stream, addr, cloned_redis_pool).await;
            });

        }
    });

    pub async fn connectMe(mut current_user_stream: std::sync::Arc<tokio::sync::Mutex<TcpStream>>, 
        current_user: SocketAddr, redis_pool: deadpool_redis::Pool){
        let command = "connectMe";
        match command{
            "connectMe" => {

                let users_streams = USERS_TCP_STREAM.clone();
                let mut get_users_streams = users_streams.lock().await;


                let online_users = ONLINE_USERS.clone();
                let mut get_online_users = online_users.lock().await;
                let cloned_current_user_stream = current_user_stream.clone();


                let mut redis_conn = redis_pool.get().await.unwrap();
                let get_connected_users: String = redis_conn.get("connectedUsers").await.unwrap();
                let mut decoded_connected_users = serde_json::from_str::<
                        HashMap<String, usize>
                    >(&get_connected_users)
                    .unwrap();

                let mut map_keys = get_online_users
                    .clone()
                    .into_iter()
                    .map(|(user, id)| user)
                    .collect::<Vec<String>>();

                map_keys.shuffle(&mut thread_rng());
                let mut found_user: String = String::from("");
                let mut found_id: usize = 0;

                for user in map_keys.clone(){

                    let id = get_online_users.get(&user).unwrap();

                    let mut redis_conn = redis_pool.get().await.unwrap();
                    let get_connected_users: String = redis_conn.get("connectedUsers").await.unwrap();
                    let mut decoded_connected_users = serde_json::from_str::<
                            HashMap<String, usize>
                        >(&get_connected_users)
                        .unwrap();

                    // the user to connect to must not be the current user as well as
                    // must not on redis in during the 2 mins period
                    // a user can't connect to a user which was connected 2 mins ago
                    if user == current_user.to_string() || decoded_connected_users.contains_key(&user){
                        continue;
                    } else{
                        
                        // start chatting with the first found user
                        found_id = *id;
                        found_user = user;
                        break;
                    }
                }

                // store the user on redis, for 2 mins he won't be able to
                // connect to the previous user
                if !found_user.is_empty() && found_id != 0{
                    decoded_connected_users.insert(found_user, found_id);
                }
                let encoded_connected_user = serde_json::to_string(&decoded_connected_users).unwrap();
                let _: () = redis_conn.set_ex("connectedUsers", &encoded_connected_user, 120).await.unwrap();
                
                // receive msg bytes from the user tcp stream channel
                let getUserSender = get_users_streams.get(&found_id).unwrap().1.clone();
                let getUserReceiver = get_users_streams.get(&found_id).unwrap().0.clone();
                let cloned_current_user_stream = cloned_current_user_stream.clone();
                
                tokio::spawn(async move{
                    
                    let mut buff = vec![];
                    let mut getStream = cloned_current_user_stream.lock().await;

                    while let Ok(rcvd_bytes) = getStream.read(&mut buff).await{
                        if rcvd_bytes == 0{
                            getStream.shutdown().await; // shutdown the stream, disconnect the connection
                        }
                        let current_user_msg = std::str::from_utf8(&buff[..rcvd_bytes]).unwrap();
                        // send the msg bytes of the current user to the connected user channel
                        // connected user will use his receiver to receive the msg 
                        getUserSender.send(current_user_msg.to_string()).await;

                        let cloned_getUserReceiver = getUserReceiver.clone();
                        let mut getReceiver = cloned_getUserReceiver.lock().await;
                        // receive the connected user msg in here and send it through the 
                        // current user tcp stream channel to the current user 
                        while let Some(connected_user_msg) = getReceiver.recv().await{
                            getStream.write_all(connected_user_msg.as_bytes()).await;
                        }
                    }
                });


            },
            _ => {}
        }
    }

    
    // ---====---====---====---====---====---====---====---====---====---====---====---====
    
    // an eventloop is a thread safe mpsc receiver queue
    #[derive(Clone)]
    struct EventLoop<T: Clone + Send + Sync + 'static>{
        // a thread safe receiver queue
        pub queue: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<T>>>,
    }
    impl<T: Clone + Send + Sync + 'static> EventLoop<T>{
        pub async fn on<F, R>(&mut self, event_name: &str, triggerer: F) 
            where F: Fn(T) -> R + Send + Sync,
            R: std::future::Future<Output = ()> + Send + Sync
            {
            println!("triggering event {:?}", event_name);
            let mut get_queue = self.queue.lock().await;
            while let Some(event) = get_queue.recv().await{
                triggerer(event);
            }
        }
    }

    #[derive(Clone)]
    struct BufferEvent{
        pub data: std::sync::Arc<tokio::sync::Mutex<Vec<u8>>>
    }
    #[derive(Serialize, Deserialize, Clone)]
    struct EventData{
        pub owner: String,
        pub recv_time: i64
    }


    // threadpool, spawning 10 threads it can be more structural 
    // like it has a task field and eventloop receiver to recevie 
    // tasks and execute them inside spawned thread also a field to 
    // store all threads for joining them later which each of which 
    // blocking the caller thread
    let (tx, mut rx) = tokio::sync::mpsc::channel::<BufferEvent>(100);
    let mut eventloop = EventLoop::<BufferEvent>{
        queue: std::sync::Arc::new(tokio::sync::Mutex::new(rx))
    };

    for _ in 0..10{
        let mut eventloop = eventloop.clone();
        tokio::spawn(async move{
            eventloop.on("receive", |e| async move{
                // WRONG CODE: don't use std mutex in tokio spawn
                let get_event_data = e.data.lock().await;
                let string = serde_json::
                    from_slice::<EventData>(&get_event_data)
                    .unwrap();
        
            }).await;
        });
    }
    
    Ok(())

}