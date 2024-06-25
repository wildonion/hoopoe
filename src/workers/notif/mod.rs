


/* ========================================================================================
            a sexy actor to produce/consume messages to/from RMQ broker exchange
            use rmq to fetch a massive data from the exchange in realtime manner

    -ˋˏ✄┈┈┈┈
    >_ the consuming task has been started by sending the ConsumeNotif message 
    to this actor which will execute the streaming loop over the queue in 
    either the notif consumer actor context itself or the tokio spawn thread:

        notif consumer -----Q(Consume Payload)-----> Exchange -----notif CQRS writer-----> cache/store on Redis & db

    -ˋˏ✄┈┈┈┈
    >_ the producing task has been started by sending the ProduceNotif message 
    to this actor which will execute the publishing process to the exchange 
    in either the notif producer actor context itself or the tokio spawn thread:

        notif producer -----payload-----> Exchange
    ======================================================================================== */

use crate::*;
use constants::CRYPTER_THEMIS_ERROR_CODE;
use constants::FILE_ERROR_CODE;
use models::event::NotifData;
use actix::prelude::*;
use actix::Handler as ActixHandler;
use actix::{Actor, AsyncContext, Context, Addr};
use config::EnvExt;
use constants::{MAILBOX_CHANNEL_ERROR_CODE, PING_INTERVAL};
use deadpool_lapin::lapin::options::{BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions};
use deadpool_lapin::lapin::protocol::exchange;
use deadpool_lapin::lapin::types::FieldTable;
use deadpool_lapin::lapin::BasicProperties;
use interfaces::*;
use storage::engine::Storage;
use wallexerr::misc::SecureCellConfig;
use wallexerr::misc::Wallet;
use workers::cqrs::mutators::notif::NotifMutatorActor;
use workers::cqrs::mutators::notif::StoreNotifEvent;
use workers::zerlog::ZerLogProducerActor;
use std::error::Error;
use std::io::Read;
use std::sync::Arc;
use crate::models::event::*;




#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct ProduceNotif{
    pub local_spawn: bool, // either spawn in actor context or tokio threadpool
    pub notif_data: NotifData,
    pub exchange_name: String,
    pub exchange_type: String,
    pub routing_key: String,
    pub encryption_config: Option<CryptoConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct CryptoConfig{
    pub secret: String,
    pub passphrase: String,
    pub path: String, // <>.config.json
}


#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct ConsumeNotif{
    /* -ˋˏ✄┈┈┈┈ 
        following queue gets bounded to the passed in exchange type with its 
        routing key, when producer wants to produce notif data it sends them 
        to the exchange with a known routing key, any queue that is bounded 
        to that exchange routing key will be filled up with messages coming 
        from the producer and they stay in there until a consumer read them
    */
    pub queue: String,
    pub exchange_name: String,
    /* -ˋˏ✄┈┈┈┈ 
        routing_key is pattern for the exchange to route the messages to the 
        bounded queue.
        multiple producers can send their messages to a single exchange but 
        each of with different routing keys.
        any queue that is bounded to the exchange routing key will receive 
        all the messages that follows the pattern inside the routing_key.
        a message can be sent from producer to an exchange in a topic way with 
        an sepecific routing key which tells the exchange this is the way of 
        receiving messages that a bounded queue does since we might have 
        sent messages to the same exchange with multiple different routing 
        keys per each message and for a queue that is bounded to the exchange 
        with the passed in routing key can only receives the messages that 
        follow the pattern in the selected routing key. so the routing key in 
        consumer is the patterns for this queue to tell exchange to what 
        messages this queue is interested in:

        1) producer produces messages and send them to the exchange with an specific routing key
        2) a consumer create its own queue and bind it to the exchange with the bind key that 
           is interested to receive the message from the exchange based on that key.
                                                                                                                 --------          ---------
                                                                                                                | queue1 | <----- |consumer1|
                                                                        ------> routing_key1 <---------------------------          ---------
                                                                       |                                            
        producer1 ----------                                       -----------------> routing_key0               
                            |____ messages > routing_key1 ------> | exchange|                                                
                             ____ messages > routing_key4 ------>  -----------------> routing_key2                                     
                            |                                          |                                --------        -----------
       producer2 -----------                                           |                               | queue2 | <----| consumer2 |
                                                                        ------> routing_key4 <------------------        -----------

    */
    pub routing_key: String, // patterns for this queue to tell exchange what messages this queue is interested in
    pub tag: String,
    pub redis_cache_exp: u64,
    pub local_spawn: bool, // either spawn in actor context or tokio threadpool
    pub cache_on_redis: bool,
    pub store_in_db: bool,
    pub decryption_config: Option<CryptoConfig>
}


#[derive(Clone)]
pub struct NotifBrokerActor{
    pub app_storage: std::option::Option<Arc<Storage>>, // REQUIRED: communicating with third party storage
    pub notif_mutator_actor: Addr<NotifMutatorActor>, // REQUIRED: communicating with mutator actor to write into redis and db 
    pub zerlog_producer_actor: Addr<ZerLogProducerActor> // REQUIRED: send any error log to the zerlog queue
} 

impl Actor for NotifBrokerActor{
    
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        log::info!("🎬 NotifBrokerActor has started");

        ctx.run_interval(PING_INTERVAL, |actor, ctx|{
            
            let this = actor.clone();
            let addr = ctx.address();

            tokio::spawn(async move{

                // check something constantly, schedule to be executed 
                // at a certain time in the background
                // ...
                
            });

        });

    }
}

impl NotifBrokerActor{

    /* ********************************************************************* */
    /* ***************************** CONSUMING ***************************** */
    /* ********************************************************************* */
    // if a consumer service wants to read notifs received from the rmq it 
    // needs to either fetch from redis or db the method doesn't return 
    // anything back to the caller
    pub async fn consume(&self, exp_seconds: u64,
        consumer_tag: &str, queue: &str, 
        binding_key: &str, exchange: &str,
        cache_on_redis: bool, store_in_db: bool,
        decryption_config: Option<CryptoConfig>
    ){

        let storage = self.app_storage.clone();
        let rmq_pool = storage.as_ref().unwrap().get_lapin_pool().await.unwrap();
        let redis_pool = storage.unwrap().get_redis_pool().await.unwrap();
        let notif_mutator_actor = self.notif_mutator_actor.clone();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        
        match rmq_pool.get().await{
            Ok(conn) => {

                let create_channel = conn.create_channel().await;
                match create_channel{
                    Ok(chan) => {

                        let mut q_options = QueueDeclareOptions::default();
                        q_options.durable = true; // durability is the ability to restore data on node shutdown

                        // -ˋˏ✄┈┈┈┈ making a queue inside the broker per each consumer, 
                        let create_queue = chan
                            .queue_declare(
                                &queue,
                                q_options,
                                FieldTable::default(),
                            )
                            .await;

                        let Ok(q) = create_queue else{
                            let e = create_queue.unwrap_err();
                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                            let error_content = &e.to_string();
                            let error_content = error_content.as_bytes().to_vec();
                            let mut error_instance = HoopoeErrorResponse::new(
                                *constants::STORAGE_IO_ERROR_CODE, // error code
                                error_content, // error content
                                ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                "NotifBrokerActor.queue_declare", // method
                                Some(&zerlog_producer_actor)
                            ).await;
                            return; // terminate the caller
                        };

                        // binding the queue to the exchange routing key to receive messages it interested in
                        /* -ˋˏ✄┈┈┈┈ 
                            if the exchange is not direct or is fanout or topic we should bind the 
                            queue to the exchange to consume the messages from the queue. binding 
                            the queue to the passed in exchange, if the exchange is direct every 
                            queue that is created is automatically bounded to it with a routing key 
                            which is the same as the queue name, the direct exchange is "" and 
                            rmq doesn't allow to bind any queue to that manually
                        */
                        match chan
                            .queue_bind(q.name().as_str(), &exchange, &binding_key, 
                                QueueBindOptions::default(), FieldTable::default()
                            )
                            .await
                            { // trying to bind the queue to the exchange
                                Ok(_) => {},
                                Err(e) => {
                                    use crate::error::{ErrorKind, HoopoeErrorResponse};
                                    let error_content = &e.to_string();
                                    let error_content = error_content.as_bytes().to_vec();
                                    let mut error_instance = HoopoeErrorResponse::new(
                                        *constants::STORAGE_IO_ERROR_CODE, // error code
                                        error_content, // error content
                                        ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                        "NotifBrokerActor.queue_bind", // method
                                        Some(&zerlog_producer_actor)
                                    ).await;
                                    return; // terminate the caller
                                }
                            }

                        // since &str is not lived long enough to be passed to the tokio spawn
                        // if it was static it would be good however we're converting them to
                        // String to pass the String version of them to the tokio spawn scope
                        let cloned_consumer_tag = consumer_tag.to_string();
                        let cloned_queue = queue.to_string();
                        /* =================================================================================== */
                        /* ================================ CONSUMING PROCESS ================================ */
                        /* =================================================================================== */
                        // start consuming in the background in a lightweight thread of execution
                        // receiving is considered to be none blocking which won't block the current thread. 
                        tokio::spawn(async move{

                            // -ˋˏ✄┈┈┈┈ consuming from the queue owned by this consumer
                            match chan
                                .basic_consume(
                                    /* -ˋˏ✄┈┈┈┈
                                        the queue that is bounded to the exchange to receive messages based on the routing key
                                        since the queue is already bounded to the exchange and its routing key, it only receives 
                                        messages from the exchange that matches and follows the passed in routing pattern like:
                                        message routing key "orders.processed" might match a binding with routing key "orders.#
                                        if none the messages follow the pattern then the queue will receive no message from the 
                                        exchange based on that pattern! although messages are inside the exchange.
                                    */
                                    &cloned_queue, 
                                    &cloned_consumer_tag, // custom consumer name
                                    BasicConsumeOptions::default(), 
                                    FieldTable::default()
                                )
                                .await
                            {
                                Ok(mut consumer) => {
                                    // stream over consumer to receive data from the queue
                                    while let Some(delivery) = consumer.next().await{
                                        match delivery{
                                            Ok(delv) => {

                                                log::info!("[*] received delivery from queue: {}", q.name());

                                                // if the consumer receives this data from the queue
                                                match delv.ack(BasicAckOptions::default()).await{
                                                    Ok(ok) => {

                                                        /* ---------------------------------------------------------------------- */
                                                        /* --------------------- storing and caching logics --------------------- */
                                                        /* ---------------------------------------------------------------------- */
                                                        let buffer = delv.data;


                                                        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                                        // ===>>>===>>>===>>>===>>>===>>> data decryption logic ===>>>===>>>===>>>===>>>===>>>
                                                        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                                        // if we have a config means the data has been encrypted
                                                        let data = if let Some(config) = decryption_config.clone(){
                                                            
                                                            match std::fs::File::open(config.path){
                                                                Ok(mut file) => {
                                                                    let mut buffer = vec![];
                                                                    let read_bytes = file.read(&mut buffer).unwrap();
                                                                    let mut secure_cell_config = serde_json::from_slice::<SecureCellConfig>(&buffer).unwrap();

                                                                    match Wallet::secure_cell_decrypt(&mut secure_cell_config){
                                                                        Ok(data) => {
                                                                            std::str::from_utf8(&buffer).unwrap().to_string()
                                                                        },
                                                                        Err(e) => {

                                                                            tokio::spawn(
                                                                                {
                                                                                    let zerlog_producer_actor = zerlog_producer_actor.clone(); // clone the old one in each iteration
                                                                                    async move{
                                                                                        let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                                                                                        let err_instance = crate::error::HoopoeErrorResponse::new(
                                                                                            *CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                                                                                            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                                                                            crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                                                                                            &String::from("NotifBrokerActor.consume.Wallet::secure_cell_encrypt"), // current method name
                                                                                            Some(&zerlog_producer_actor)
                                                                                        ).await;
                                                                                    }
                                                                                }
                                                                            );

                                                                            // can't decrypt return the raw base58 string of encrypted data
                                                                            // this can't be decoded to NotifData we'll get serde error!
                                                                            std::str::from_utf8(&buffer).unwrap().to_string()
                                                                        }
                                                                    }
                                                                },
                                                                Err(e) => {

                                                                    tokio::spawn(
                                                                        {
                                                                            let zerlog_producer_actor = zerlog_producer_actor.clone(); // clone the old one in each iteration
                                                                            async move{
                                                                                let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                                                                                let err_instance = crate::error::HoopoeErrorResponse::new(
                                                                                    *FILE_ERROR_CODE, // error hex (u16) code
                                                                                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                                                                    crate::error::ErrorKind::File(crate::error::FileEror::ReadWrite(e)), // the actual source of the error caused at runtime
                                                                                    &String::from("NotifBrokerActor.consume.std::fs::File::open"), // current method name
                                                                                    Some(&zerlog_producer_actor)
                                                                                ).await;
                                                                            }
                                                                        }
                                                                    );
                                                                    
                                                                    // can't open the file perhaps it's deleted!
                                                                    // return the raw base58 string of encrypted data, this can't 
                                                                    // be decoded to NotifData we'll get serde error!
                                                                    std::str::from_utf8(&buffer).unwrap().to_string()
                                                                }
                                                            }

                                                        } else{
                                                            // no decryption config is needed
                                                            std::str::from_utf8(&buffer).unwrap().to_string()
                                                        };
                                                        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                                        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                                        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>

                                                        // either decrypted or the raw data
                                                        log::info!("[*] received data: {}", data);
                                                        
                                                        let get_notif_event = serde_json::from_str::<NotifData>(&data);
                                                        match get_notif_event{
                                                            Ok(notif_event) => {

                                                                log::info!("[*] decoded data: {:?}", notif_event);

                                                                // =============================================================================
                                                                // ------------- if the cache on redis flag was activated we then store on redis
                                                                // =============================================================================
                                                                if cache_on_redis{
                                                                    match redis_pool.get().await{
                                                                        Ok(mut redis_conn) => {

                                                                            // key: String::from(notif_receiver.id) | value: Vec<NotifData>
                                                                            let redis_notif_key = format!("notif_owner:{}", &notif_event.receiver_info);
                                                                            
                                                                            // -ˋˏ✄┈┈┈┈ extend notifs
                                                                            let get_events: RedisResult<String> = redis_conn.get(&redis_notif_key).await;
                                                                            let events = match get_events{
                                                                                Ok(events_string) => {
                                                                                    let get_messages = serde_json::from_str::<Vec<NotifData>>(&events_string);
                                                                                    match get_messages{
                                                                                        Ok(mut messages) => {
                                                                                            messages.push(notif_event.clone());
                                                                                            messages
                                                                                        },
                                                                                        Err(e) => {
                                                                                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                                                            let error_content = &e.to_string();
                                                                                            let error_content_ = error_content.as_bytes().to_vec();
                                                                                            let mut error_instance = HoopoeErrorResponse::new(
                                                                                                *constants::CODEC_ERROR_CODE, // error code
                                                                                                error_content_, // error content
                                                                                                ErrorKind::Codec(crate::error::CodecError::Serde(e)), // error kind
                                                                                                "NotifBrokerActor.decode_serde_redis", // method
                                                                                                Some(&zerlog_producer_actor)
                                                                                        ).await;
                                                                                            return; // terminate the caller
                                                                                        }
                                                                                    }
                                                                    
                                                                                },
                                                                                Err(e) => {

                                                                                    // ------------------------------------------------------------------
                                                                                    // followings are commented to prevent producing high amounts of logs
                                                                                    // ------------------------------------------------------------------
                                                                                    /* 
                                                                                    use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                                                    let error_content = &e.to_string();
                                                                                    let error_content_ = error_content.as_bytes().to_vec();
                                                                                    let mut error_instance = HoopoeErrorResponse::new(
                                                                                        *constants::STORAGE_IO_ERROR_CODE, // error code
                                                                                        error_content_, // error content
                                                                                        ErrorKind::Storage(crate::error::StorageError::Redis(e)), // error kind
                                                                                        "NotifBrokerActor.redis_get", // method
                                                                                        Some(&zerlog_producer_actor)
                                                                                    ).await;
                                                                                    */

                                                                                    // we can't get the key means this is the first time we're creating the key
                                                                                    // or the key is expired already, we'll create a new key either way and put
                                                                                    // the init message in there.
                                                                                    let init_message = vec![
                                                                                        notif_event.clone()
                                                                                    ];

                                                                                    init_message

                                                                                }
                                                                            };

                                                                            // -ˋˏ✄┈┈┈┈ caching the notif event in redis with expirable key
                                                                            // chaching in redis is an async task which will be executed 
                                                                            // in the background with an expirable key
                                                                            tokio::spawn(async move{
                                                                                let events_string = serde_json::to_string(&events).unwrap();
                                                                                let is_key_there: bool = redis_conn.exists(&redis_notif_key.clone()).await.unwrap();
                                                                                if is_key_there{ // update only the value
                                                                                    let _: () = redis_conn.set(&redis_notif_key.clone(), &events_string).await.unwrap();
                                                                                } else{ // initializing a new expirable key containing the new notif data
                                                                                    /*
                                                                                        make sure you won't get the following error:
                                                                                        called `Result::unwrap()` on an `Err` value: MISCONF: Redis is configured to 
                                                                                        save RDB snapshots, but it's currently unable to persist to disk. Commands that
                                                                                        may modify the data set are disabled, because this instance is configured to 
                                                                                        report errors during writes if RDB snapshotting fails (stop-writes-on-bgsave-error option). 
                                                                                        Please check the Redis logs for details about the RDB error. 
                                                                                        SOLUTION: restart redis :)
                                                                                    */
                                                                                    let _: () = redis_conn.set_ex(&redis_notif_key.clone(), &events_string, exp_seconds).await.unwrap();
                                                                                }
                                                                            });

                                                                        },
                                                                        Err(e) => {
                                                                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                                            let error_content = &e.to_string();
                                                                            let error_content_ = error_content.as_bytes().to_vec();
                                                                            let mut error_instance = HoopoeErrorResponse::new(
                                                                                *constants::STORAGE_IO_ERROR_CODE, // error code
                                                                                error_content_, // error content
                                                                                ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // error kind
                                                                                "NotifBrokerActor.redis_pool", // method
                                                                                Some(&zerlog_producer_actor)
                                                                        ).await;
                                                                            return; // terminate the caller
                                                                        }
                                                                    }
                                                                }

                                                                // =============================================================================
                                                                // ------------- if the store in db flag was activated we then store in database
                                                                // =============================================================================
                                                                if store_in_db{
                                                                    // -ˋˏ✄┈┈┈┈ store notif in db by sending message to the notif mutator actor worker
                                                                    // sending StoreNotifEvent message to the notif event mutator actor
                                                                    // spawning the async task of storing data in db in the background
                                                                    // worker of lightweight thread of execution using tokio threadpool
                                                                    tokio::spawn(
                                                                        {
                                                                            let cloned_message = notif_event.clone();
                                                                            let cloned_mutator_actor = notif_mutator_actor.clone();
                                                                            let zerlog_producer_actor = zerlog_producer_actor.clone();
                                                                            async move{
                                                                                match cloned_mutator_actor
                                                                                    .send(StoreNotifEvent{
                                                                                        message: cloned_message,
                                                                                        local_spawn: true
                                                                                    })
                                                                                    .await
                                                                                    {
                                                                                        Ok(_) => { () },
                                                                                        Err(e) => {
                                                                                            let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                                                                                            let err_instance = crate::error::HoopoeErrorResponse::new(
                                                                                                *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                                                                                                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                                                                                crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                                                                                                &String::from("NotifBrokerActor.notif_mutator_actor.send"), // current method name
                                                                                                Some(&zerlog_producer_actor)
                                                                                            ).await;
                                                                                            return;
                                                                                        }
                                                                                    }
                                                                            }
                                                                        }
                                                                    );
                                                                }

                                                            },
                                                            Err(e) => {
                                                                use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                                let error_content = &e.to_string();
                                                                let error_content_ = error_content.as_bytes().to_vec();
                                                                let mut error_instance = HoopoeErrorResponse::new(
                                                                    *constants::CODEC_ERROR_CODE, // error code
                                                                    error_content_, // error content
                                                                    ErrorKind::Codec(crate::error::CodecError::Serde(e)), // error kind
                                                                    "NotifBrokerActor.decode_serde", // method
                                                                    Some(&zerlog_producer_actor)
                                                                ).await;
                                                                return; // terminate the caller
                                                            }
                                                        }
                                                    },
                                                    Err(e) => {
                                                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                        let error_content = &e.to_string();
                                                        let error_content = error_content.as_bytes().to_vec();
                                                        let mut error_instance = HoopoeErrorResponse::new(
                                                            *constants::STORAGE_IO_ERROR_CODE, // error code
                                                            error_content, // error content
                                                            ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                                            "NotifBrokerActor.consume_ack", // method
                                                            Some(&zerlog_producer_actor)
                                                        ).await;
                                                        return; // terminate the caller
                                                    }
                                                }
                    
                                            },
                                            Err(e) => {
                                                use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                let error_content = &e.to_string();
                                                let error_content = error_content.as_bytes().to_vec();
                                                let mut error_instance = HoopoeErrorResponse::new(
                                                    *constants::STORAGE_IO_ERROR_CODE, // error code
                                                    error_content, // error content
                                                    ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                                    "NotifBrokerActor.consume_getting_delivery", // method
                                                    Some(&zerlog_producer_actor)
                                                ).await;
                                                return; // terminate the caller 
                                            }
                                        }
                                    }
                                },
                                Err(e) => {
                                    use crate::error::{ErrorKind, HoopoeErrorResponse};
                                    let error_content = &e.to_string();
                                    let error_content = error_content.as_bytes().to_vec();
                                    let mut error_instance = HoopoeErrorResponse::new(
                                        *constants::STORAGE_IO_ERROR_CODE, // error code
                                        error_content, // error content
                                        ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                        "NotifBrokerActor.consume_basic_consume", // method
                                        Some(&zerlog_producer_actor)
                                    ).await;
                                    return; // terminate the caller 
                                }
                            }

                        });

                    },
                    Err(e) => {
                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                        let error_content = &e.to_string();
                        let error_content = error_content.as_bytes().to_vec();
                        let mut error_instance = HoopoeErrorResponse::new(
                            *constants::STORAGE_IO_ERROR_CODE, // error code
                            error_content, // error content
                            ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                            "NotifBrokerActor.consume_create_channel", // method
                            Some(&zerlog_producer_actor)
                        ).await;
                        return; // terminate the caller   
                    }
                }

            },
            Err(e) => {
                use crate::error::{ErrorKind, HoopoeErrorResponse};
                let error_content = &e.to_string();
                let error_content = error_content.as_bytes().to_vec();
                let mut error_instance = HoopoeErrorResponse::new(
                    *constants::STORAGE_IO_ERROR_CODE, // error code
                    error_content, // error content
                    ErrorKind::Storage(crate::error::StorageError::RmqPool(e)), // error kind
                    "NotifBrokerActor.consume_pool", // method
                    Some(&zerlog_producer_actor)
                ).await;
                return; // terminate the caller
            }
        };

    }

    /* ********************************************************************* */
    /* ***************************** PRODUCING ***************************** */
    /* ********************************************************************* */
    pub async fn produce(&self, data: &str, exchange: &str, routing_key: &str, exchange_type: &str){

        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        let this = self.clone();

        // these are must be converted into String first to make longer lifetime 
        // cause &str can't get moved into tokio spawn as its lifetime it's not 
        // static the tokio spawn lives longer than the &str and the &str gets 
        // dropped out of the ram once the function is finished with executing
        let exchange = exchange.to_string();
        let routing_key = routing_key.to_string();
        let exchange_type = exchange_type.to_string();
        let data = data.to_string();

        tokio::spawn(async move{

            let storage = this.clone().app_storage.clone();
            let rmq_pool = storage.unwrap().get_lapin_pool().await.unwrap();
            
            // trying to ge a connection from the pool
            match rmq_pool.get().await{
                Ok(pool) => {

                    // -ˋˏ✄┈┈┈┈ creating a channel in this thread
                    match pool.create_channel().await{
                        Ok(chan) => {

                            let mut ex_options = ExchangeDeclareOptions::default();
                            ex_options.auto_delete = true; // the exchange can only be deleted automatically if all bindings are deleted
                            ex_options.durable = true; // durability is the ability to restore data on node shutdown

                            // -ˋˏ✄┈┈┈┈ creating exchange
                            /* 
                                you should set the auto_delete flag to True when declaring the exchange. This will 
                                automatically delete the exchange when all channels are done with it.
                                Keep in mind that this means that it will stay as long as there is an active binding 
                                to the exchange. If you delete the binding, or queue, the exchange will be deleted.
                                if you need to keep the queue, but not the exchange you can remove the binding once 
                                you are done publishing. This should automatically remove the exchange.
                                so when all bindings (queues and exchanges) get deleted the exchange will be deleted.
                            */
                            match chan
                                .exchange_declare(&exchange, {
                                    match exchange_type.as_str(){
                                        "fanout" => deadpool_lapin::lapin::ExchangeKind::Fanout,
                                        "direct" => deadpool_lapin::lapin::ExchangeKind::Direct,
                                        "headers" => deadpool_lapin::lapin::ExchangeKind::Headers,
                                        _ => deadpool_lapin::lapin::ExchangeKind::Topic,
                                    }
                                },
                                ex_options, FieldTable::default()
                                )
                                .await
                                {
                                    Ok(ok) => {ok},
                                    Err(e) => {
                                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                                        let e_string = &e.to_string();
                                        let error_content = e_string.as_bytes().to_vec();
                                        let mut error_instance = HoopoeErrorResponse::new(
                                            *constants::STORAGE_IO_ERROR_CODE, // error code
                                            error_content, // error content
                                            ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                            "NotifBrokerActor.exchange_declare", // method
                                            Some(&zerlog_producer_actor)
                                        ).await;
                                        
                                    }

                                };

                            /* =================================================================================== */
                            /* ================================ PRODUCING PROCESS ================================ */
                            /* =================================================================================== */
                            // async task: publish messages to exchange in the background in a lightweight thread of execution
                            tokio::spawn(async move{
                                
                                // -ˋˏ✄┈┈┈┈ publishing to exchange from this channel,
                                // later consumer bind its queue to this exchange and its
                                // routing key so messages go inside its queue, later they 
                                // can be consumed from the queue by the consumer
                                use deadpool_lapin::lapin::options::BasicPublishOptions;
                                let payload = data.as_bytes();
                                match chan
                                    .basic_publish(
                                        &exchange, // the way of sending messages
                                        &routing_key, // the way that message gets routed to the queue based on a unique routing key
                                        BasicPublishOptions::default(),
                                        payload, // this is the ProduceNotif data,
                                        BasicProperties::default().with_content_type("application/json".into()),
                                    )
                                    .await
                                    {
                                        Ok(pc) => {
                                            let get_confirmation = pc.await;
                                            let Ok(confirmation) = get_confirmation else{
                                                use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                let error_content_ = get_confirmation.unwrap_err();
                                                let e_string = &error_content_.to_string();
                                                let error_content = e_string.as_bytes().to_vec();
                                                let mut error_instance = HoopoeErrorResponse::new(
                                                    *constants::STORAGE_IO_ERROR_CODE, // error code
                                                    error_content, // error content
                                                    ErrorKind::Storage(crate::error::StorageError::Rmq(error_content_)), // error kind
                                                    "NotifBrokerActor.get_confirmation", // method
                                                    Some(&zerlog_producer_actor)
                                                ).await;

                                                return; // needs to terminate the caller in let else pattern
                                            };

                                        },
                                        Err(e) => {
                                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                                            let error_content = &e.to_string();
                                            let error_content = error_content.as_bytes().to_vec();
                                            let mut error_instance = HoopoeErrorResponse::new(
                                                *constants::STORAGE_IO_ERROR_CODE, // error code
                                                error_content, // error content
                                                ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                                "NotifBrokerActor.basic_publish", // method
                                                Some(&zerlog_producer_actor)
                                            ).await;

                                        }
                                    }
                            });

                        },
                        Err(e) => {
                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                            let error_content = &e.to_string();
                            let error_content = error_content.as_bytes().to_vec();
                            let mut error_instance = HoopoeErrorResponse::new(
                                *constants::STORAGE_IO_ERROR_CODE, // error code
                                error_content, // error content
                                ErrorKind::Storage(crate::error::StorageError::Rmq(e)), // error kind
                                "NotifBrokerActor.create_channel", // method
                                Some(&zerlog_producer_actor)
                            ).await;

                        }
                    }
                },
                Err(e) => {

                    use crate::error::{ErrorKind, HoopoeErrorResponse};
                    let error_content = &e.to_string();
                    let error_content = error_content.as_bytes().to_vec();
                    let mut error_instance = HoopoeErrorResponse::new(
                        *constants::STORAGE_IO_ERROR_CODE, // error code
                        error_content, // error content
                        ErrorKind::Storage(crate::error::StorageError::RmqPool(e)), // error kind
                        "NotifBrokerActor.produce_pool", // method
                        Some(&zerlog_producer_actor)
                    ).await;

                }
            };
            
        });
        
    
    }

    pub fn new(app_storage: std::option::Option<Arc<Storage>>, 
        notif_mutator_actor: Addr<NotifMutatorActor>,
        zerlog_producer_actor: Addr<ZerLogProducerActor>) -> Self{
        Self { app_storage, notif_mutator_actor, zerlog_producer_actor}
    }

}

/* ********************************************************************************* */
/* ***************************** PRODUCE NOTIF HANDLER ***************************** */
/* ********************************************************************************* */
impl ActixHandler<ProduceNotif> for NotifBrokerActor{
    
    type Result = ();
    fn handle(&mut self, msg: ProduceNotif, ctx: &mut Self::Context) -> Self::Result {

        // unpacking the notif data
        let ProduceNotif { 
                exchange_name,
                exchange_type,
                routing_key,
                local_spawn,
                notif_data,
                encryption_config,

            } = msg.clone();
        
        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
        // ===>>>===>>>===>>>===>>>===>>> data encryption logic ===>>>===>>>===>>>===>>>===>>>
        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
        let stringified_data = if let Some(config) = encryption_config{

            let mut secure_cell_config = &mut wallexerr::misc::SecureCellConfig{
                secret_key: config.secret,
                passphrase: config.passphrase,
                data: serde_json::to_vec(&notif_data).unwrap(),
            };

            // store the config as json, since we don't need output or any result 
            // from the task inside the thread thus there is no channel to send
            // data to outside of tokio::spawn
            tokio::spawn(
                {
                    let secure_cell_config = secure_cell_config.clone();
                    async move{
                        let config_path = format!("{}", config.path);
                        let config_file = tokio::fs::File::create(config_path).await;
                        config_file.unwrap().write_all(
                            &serde_json::to_string_pretty(&secure_cell_config).unwrap().as_bytes()
                        ).await;
                    }
                }
            );
            
            match Wallet::secure_cell_encrypt(secure_cell_config){
                Ok(data) => {
                    use base58::ToBase58;
                    let base58_data = data.to_base58();
                    base58_data               
                },
                Err(e) => {
                    let zerlog_producer_actor = self.zerlog_producer_actor.clone();
                    // log the error in the a lightweight thread of execution inside tokio threads
                    // since we don't need output or any result from the task inside the thread thus
                    // there is no channel to send data to outside of tokio::spawn
                    tokio::spawn(async move{
                        let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                        let err_instance = crate::error::HoopoeErrorResponse::new(
                            *CRYPTER_THEMIS_ERROR_CODE, // error hex (u16) code
                            source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                            crate::error::ErrorKind::Crypter(crate::error::CrypterError::Themis(e)), // the actual source of the error caused at runtime
                            &String::from("NotifBrokerActor.ActixHandler<ProduceNotif>.Wallet::secure_cell_encrypt"), // current method name
                            Some(&zerlog_producer_actor)
                        ).await;
                    });
                    // return the actual data with no encryption
                    serde_json::to_string_pretty(&notif_data).unwrap()
                }
            }


        } else{
            // return the actual data with no encryption
            serde_json::to_string_pretty(&notif_data).unwrap()
        };
        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
        // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>

        let this = self.clone();

        // spawn the future in the background into the given actor context thread
        // by doing this we're executing the future inside the actor thread since
        // every actor has its own thread of execution.
        if local_spawn{
            async move{
                this.produce(&stringified_data, &exchange_name, &routing_key, &exchange_type).await;
            }
            .into_actor(self) // convert the future into an actor future of type NotifBrokerActor
            .spawn(ctx); // spawn the future object into this actor context thread
        } else{ // spawn the future in the background into the tokio lightweight thread
            tokio::spawn(async move{
                this.produce(&stringified_data, &exchange_name, &routing_key, &exchange_type).await;
            });
        }
        
        return;
        
    }

}

/* ********************************************************************************* */
/* ***************************** CONSUME NOTIF HANDLER ***************************** */
/* ********************************************************************************* */
impl ActixHandler<ConsumeNotif> for NotifBrokerActor{
    
    type Result = ();
    fn handle(&mut self, msg: ConsumeNotif, ctx: &mut Self::Context) -> Self::Result {

        // unpacking the consume data
        let ConsumeNotif { 
                queue, 
                tag,
                exchange_name,
                routing_key,
                redis_cache_exp,
                local_spawn,
                cache_on_redis,
                store_in_db,
                decryption_config

            } = msg.clone(); // the unpacking pattern is always matched so if let ... is useless
        
        let this = self.clone();
        
        // spawn the future in the background into the given actor context thread
        // by doing this we're executing the future inside the actor thread since
        // every actor has its own thread of execution.
        if local_spawn{
            async move{
                this.consume(
                    redis_cache_exp, 
                    &tag, 
                    &queue, 
                    &routing_key, 
                    &exchange_name,
                    cache_on_redis,
                    store_in_db,
                    decryption_config
                ).await;
            }
            .into_actor(self) // convert the future into an actor future of type NotifBrokerActor
            .spawn(ctx); // spawn the future object into this actor context thread
        } else{ // spawn the future in the background into the tokio lightweight thread
            tokio::spawn(async move{
                this.consume(
                    redis_cache_exp, 
                    &tag, 
                    &queue, 
                    &routing_key, 
                    &exchange_name,
                    cache_on_redis,
                    store_in_db,
                    decryption_config
                ).await;
            });
        }
        return; // terminate the caller

    }

}