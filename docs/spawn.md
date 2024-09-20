### proper way to write and handle async task execution! 

> [!IMPORTANT]
> every async task must be spawned in a light io thread using a background worker which can be done via `tokio::spawn()`, it's like creating a worker per each async task, async tasks or jobs are none blocking io operations which requires a none blocking scope to execute them.

```rust
async fn heavy_io_process_(){
    // it can be one of the following tasks:
    // --> consuming task: send consume message to consumer actor
    // --> producing task: send produce message to producer actor
    // --> storing in db: send data to mutator actor 
    // --> storing in redis: cache on redis with exp key
    // --> locking logic
    // --> db operations 
}

let (tx, rx) = channel::<String>(); // channel for stringified data
let tx = tx.clone();
// spawn in the background and use channel to receive 
// data whenever the data is sent to the channel
tokio::spawn(async move{
    let res = heavy_io_process_().await;
    tx.send(res).await;
});
while let Some(data) = rx.recv().await{
    // do the rest of the logics in here 
    // whenever we receive data
    // ...
}

// ------------
//      or 
// ------------

// spawn in the background but wait to gets solved 
// and once it's solved we then proceed with the rest
// of flow and cacnel other branches or async tasks
let task = tokio::spawn(async move{
    let res = heavy_io_process_().await;
    tx.send(res).await;
});

tokio::select! {
    // choose this if it can gets completed soon
    _ = task => {
        // proceed with this flow
        // ...
    }, 
    // or if you think this gets completed soonly
    data = rx.recv() => {
        // proceed with this flow
        // ...
    }
}
```

### proper way to produce and consume data from RMQ broker

> [!IMPORTANT]
> start producing or consuming in the background by sending related message to their actors inside the `tokio::spawn()` scope, this way can be used to execute any async task or job gently in the background threads using tokio scheduler.

#### to produce data in the background: 

```rust
#[derive(Clone)]
struct SomeData{}
let data = SomeData{};
tokio::spawn( // running the producing notif job in the background in a free thread
    {
        let cloned_app_state = app_state.clone();
        let cloned_notif = ProduceNotif{
            "local_spawn": true,
            "notif_data": { 
                "receiver_info": "1",
                "id": "unqie-id0",
                "action_data": {
                    "pid": 200.4
                }, 
                "actioner_info": "2", 
                "action_type": "ProductPurchased", 
                "fired_at": 1714316645, 
                "is_seen": false
            }, 
            "exchange_name": "SavageEx",
            "exchange_type": "fanout", // amq.topic for pubsub
            "routing_key": "" // routing pattern or key - will be ignored if type is fanout
        };
        async move{
            match cloned_app_ctx.clone().unwrap().actors.as_ref().unwrap()
                    .broker_actors.notif_actor.send(cloned_notif).await
                {
                    Ok(_) => { 
                        
                        // in here you could access the notif for an owner using 
                        // a redis key like: notif_owner:3 which retrieve all data
                        // on the redis for the receiver with id 3   
                        () 

                    },
                    Err(e) => {
                        let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                        let err_instance = crate::error::HoopoeErrorResponse::new(
                            *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                            crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                            &String::from("register_notif.producer_actor.notif_actor.send"), // current method name
                            Some(&zerlog_producer_actor)
                        ).await;
                        return;
                    }
                }
        }
);
```

> to consume data in the background:

```rust
tokio::spawn( // running the consuming notif job in the background in a free thread
    {
        let cloned_app_state = app_state.clone();
        let cloned_notif = ConsumeNotif{
            "queue": "TestOnion",
            "exchange_name": "SavageEx",
            "routing_key": "",
            "tag": "cons_tag0",
            "redis_cache_exp": 300,
            "local_spawn": true,
            "cache_on_redis": true,
            "store_in_db": true
        };
        async move{
            // consuming notif by sending the ConsumeNotif message to 
            // the consumer actor,
            match cloned_app_ctx.clone().unwrap().actors.as_ref().unwrap()
                    .broker_actors.notif_actor.send(cloned_notif).await
                {
                    Ok(_) => { () },
                    Err(e) => {
                        let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                        let err_instance = crate::error::HoopoeErrorResponse::new(
                            *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                            crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                            &String::from("register_notif.consumer_actor.notif_actor.send"), // current method name
                            Some(&zerlog_producer_actor)
                        ).await;
                        return;
                    }
                }

        }
    }
);
```