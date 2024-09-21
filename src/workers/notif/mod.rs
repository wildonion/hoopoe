

/*  ====================================================================================
       REALTIME NOTIF EVENT STREAMING DESIGN PATTERN (README files inside docs folder)
    ====================================================================================
                   MAKE SURE YOU'VE STARTED CONSUMERS BEFORE PRODUCING
                   THEY MUST BE READY FOR CONSUMING WHILE PRODUCERS ARE
                   SENDING MESSAGES TO THE BROKER. USUALLY RUN THE PRODUCER
                   USING CLI AND THE CONSUMER AT STARTUP.

    NotifBrokerActor is the worker of handling the process of publishing and consuming 
    messages through rmq, redis and kafka, talking to the NotifBrokerActor can be done 
    by sending it a message contains the setup either to publish or consume something 
    to and from an specific broker, so generally it's a sexy actor to produce/consume 
    messages from different type of brokers it uses RMQ, Redis and Kafka to produce and 
    consume massive messages in realtime, kindly it supports data AES256 encryption 
    through producing messages to the broker. we can send either producing or consuming 
    message to this actor to start producing or consuming in the background.

    ************************************************************************************
    it's notable that for realtime push notif streaming we MUST start consuming from
    the specified broker passed in to the message structure when talking with actor, in
    a place where the application logic which is likely a server is being started.
    ************************************************************************************
    
    ====================================================================================
*/

use constants::STORAGE_IO_ERROR_CODE;
use rdkafka::consumer::CommitMode;
use rdkafka::consumer::Consumer;
use rdkafka::consumer::StreamConsumer;
use rdkafka::error::KafkaError;
use rdkafka::message::Header;
use rdkafka::message::Headers;
use rdkafka::message::OwnedHeaders;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use rdkafka::ClientConfig;
use rdkafka::Message;
use redis_async::resp::FromResp;
use tokio::spawn;
use workers::scheduler::CronScheduler;
use crate::*;
use deadpool_lapin::lapin::protocol::channel;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::redis::RedisResult;
use futures::executor;
use models::event::NotifData;
use actix::prelude::*;
use actix::Handler as ActixMessageHandler;
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
use crate::interfaces::crypter::Crypter;


/*  ========================================================================================
    brokering is all about queueing, sending and receiving messages way more faster, 
    safer and reliable than a simple eventloop or a tcp based channel. 
    all brokers contains message/task/job queue to handle communication between services
    asyncly, they've been designed specially for decoupling tight long running services 
    due to their durability nature like giving predicting output of an ai model service
    while the ai model is working on other input prediction in the background we can 
    receive the old outputs and pass them through the brokers to receive them in some 
    http service for responding the caller.   
    In rmq producer sends message to exchange the a consumer can bind its queue to 
    the exchange to receive the messages, routing key determines the pattern of receiving 
    messages inside the bounded queue from the exchange 
    In kafka producer sends messages to topic the consumer can receives data from 
    the topic, Rmq adds an extra layer on top of msg handling logic which is creating 
    queue per each consumer.
    offset in kafka is an strategy which determines the way of tracking the sequential 
    order of receiving messages by kafka topics it's like routing key in rmq 
    in kafka you should create consumer and producer separately but in rmq everything is 
    started from a channel, we'll create a channel to declare the queue, exchange, consumer 
    and producer in that channel, channel is a thread that can manage multiple connection 
    to the broker through a single tcp connection.

    BROKER TYPES: (preferred stack: RMQ + RPC + WebSocket + ShortPollingJobId)
        → REDIS PUBSUB => light task queue
        → KAFKA        => high latency hight throughput
            -ˋˏ✄┈┈┈┈
            >_ topic contains messages allows consumers to consume from topics each topic 
            can be divided into multiple partitions for example a topic might have 10 
            partitions, each message can have its own unique partition key which specifies 
            to which partition the message will go. Offset can be assigned to each message
            within a partition it specifies the position of the message in that partition
            it's useful for the consumers to resume consuming where they've left.
            a consumer can commit the message after processing it which tells kafka that
            the consumer has received and processed the message completely.
            single consumer consumes messages from specific partitions but in group of 
            consumers kafka ensures that each partition is consumed by only one consumer 
            within the group like if a topic with 4 partitions and 2 consumers are in the 
            same group, Kafka will assign 2 partitions to each consumer:

                                                                 ----------> partition-key1 queue(m1, m2, m3, ...) - all messages with key1
             ---------                       ------------       |
            |consumer1| <-----consume-----> |Kafka Broker| <-----topic-----> partition-key3 queue(m1, m2, m3, ...) - all messages with key3
             ---------                      ------------        |
                |_______partition1&2______________|      |      |----------> partition-key2 queue(m1, m2, m3, ...) - all messages with key2
             ---------                            |      |      |
            |consumer2|                           |      |       ----------> partition-key4 queue(m1, m2, m3, ...) - all messages with key4
             ---------                            |      |
                 |                                |   ---------
                  --------------partition3&4------   | producer|
                                                      ---------
            it's notable that committing the offset too early, instead, might cause message 
            loss, since upon recovery the consumer will start from the next message, skipping 
            the one where the failure occurred.

        → RMQ          => low latency low throughput
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
            
            -ˋˏ✄┈┈┈┈
            >_ client uses a short polling technique or websocket streaming to fetch notifications 
            for an specific owner from redis or db, this is the best solution to implement a
            realtiming strategy on the client side to fetch what's happening on the 
            server side in realtime.

             _________                                      _________
            | server1 | <------- RMQ notif broker -------> | server2 |
             ---------                                      ---------
                | ws                                          | ws
                 ------------------- client ------------------
    ======================================================================================== 
*/

#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct PublishNotifToKafka{
    pub topic: String,
    pub brokers: String,
    pub partitions: u64,
    pub headers: Vec<KafkaHeader>,
    pub local_spawn: bool,
    pub notif_data: NotifData, 
    pub encryptionConfig: Option<CryptoConfig>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct KafkaHeader{
    pub key: String, 
    pub val: String,
}

#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct ConsumeNotifFromKafka{ // we must build a unique consumer per each consuming process 
    pub topics: Vec<String>,
    pub consumerGroupId: String,
    pub brokers: String,
    pub redis_cache_exp: u64,
    pub decryptionConfig: Option<CryptoConfig>
}

#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct PublishNotifToRedis{
    pub channel: String,
    pub local_spawn: bool,
    pub notif_data: NotifData, 
    pub encryptionConfig: Option<CryptoConfig>
}

#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct ConsumeNotifFromRedis{
    pub channel: String,
    pub redis_cache_exp: u64,
    pub decryptionConfig: Option<CryptoConfig>
}

#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct HealthMsg{
    pub shutdown: bool
}

#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct PublishNotifToRmq{
    pub local_spawn: bool, // either spawn in actor context or tokio threadpool
    pub notif_data: NotifData,
    pub exchange_name: String,
    pub exchange_type: String,
    pub routing_key: String,
    pub encryptionConfig: Option<CryptoConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct ProducerInfo{
    pub Rmq: Option<PublishNotifToRmq>,
    pub Kafka: Option<PublishNotifToKafka>,
    pub Redis: Option<PublishNotifToRedis>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct ConsumerInfo{
    pub Rmq: Option<ConsumeNotifFromRmq>,
    pub Kafka: Option<ConsumeNotifFromKafka>,
    pub Redis: Option<ConsumeNotifFromRedis>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct CryptoConfig{
    pub secret: String,
    pub passphrase: String,
}

#[derive(Message, Clone, Serialize, Deserialize, Debug, Default, ToSchema)]
#[rtype(result = "()")]
pub struct ConsumeNotifFromRmq{ // we'll create a channel then start consuming by binding a queue to the exchange
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
        3) it's notable that a queue can be bounded to multiple exchange at the same time 
           it allows to receive different messages based on each exchange routing key.
                                                                                                                 --------          ---------
                                                                                                                | queue1 | <----- |consumer1|
                                                                        ------> routing_key1 <---------------------------          ---------
                                                                       |                                            
        producer1 ----------                                       -----------------> routing_key0  .........        
                            |____ messages > routing_key1 ------> | exchange1|                                                
                             ____ messages > routing_key4 ------>  -----------------> routing_key2  .........                                   
                            |                                          |                                --------        -----------
       producer2 -----------                                           |                               | queue2 | <----| consumer2 |
                                                                        ------> routing_key4 <------------------        -----------
                                                                     ----------                             |
                                                                    | exchange2| -----bind(routing_key)-----
                                                                     ----------
    */
    pub routing_key: String, // patterns for this queue to tell exchange what messages this queue is interested in
    pub tag: String,
    pub redis_cache_exp: u64,
    pub local_spawn: bool, // either spawn in actor context or tokio threadpool
    pub store_in_db: bool,
    pub decryptionConfig: Option<CryptoConfig>
}


#[derive(Clone)]
pub struct NotifBrokerActor{
    pub notif_broker_sender: tokio::sync::mpsc::Sender<String>, // use to send notif data to mpsc channel for ws
    pub app_storage: std::option::Option<Arc<Storage>>, // REQUIRED: communicating with third party storage
    pub notif_mutator_actor: Addr<NotifMutatorActor>, // REQUIRED: communicating with mutator actor to write into redis and db 
    pub zerlog_producer_actor: Addr<ZerLogProducerActor> // REQUIRED: send any error log to the zerlog rmq queue
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
                // at a certain time in the background lightweight thread
                // ...
                
            });

        });

    }
}

impl NotifBrokerActor{

    /* ******************************************************************************* */
    /* ***************************** PUBLISHING TO REDIS ***************************** */
    /* ******************************************************************************* */
    pub async fn publishToRedis(&self, channel: &str, notif_data: NotifData, encryptionConfig: Option<CryptoConfig>){
        
        let storage = self.app_storage.clone();
        let rmq_pool = storage.as_ref().unwrap().get_lapin_pool().await.unwrap();
        let redis_pool = storage.unwrap().get_redis_pool().await.unwrap();
        let notif_mutator_actor = self.notif_mutator_actor.clone();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        // will be used to send received notif data from the broker to ws mpsc channel, 
        // later we can receive the notif in ws server setup and send it to the owner
        let notif_data_sender = self.notif_broker_sender.clone(); 
        
        match redis_pool.get().await{
            Ok(mut redis_conn) => {
                
                let channel = channel.to_string();
                tokio::spawn(async move{
                    let mut dataString = serde_json::to_string(&notif_data).unwrap();
                    let finalData = if encryptionConfig.is_some(){
                        
                        let CryptoConfig{ secret, passphrase } = encryptionConfig.unwrap();
                        let mut secure_cell_config = &mut wallexerr::misc::SecureCellConfig{
                            secret_key: hex::encode(secret),
                            passphrase: hex::encode(passphrase),
                            data: vec![],
                        };
    
                        // after calling encrypt method dataString has changed and contains the hex encrypted data
                        dataString.encrypt(secure_cell_config);
                        
                        dataString
    
                    } else{
                        dataString
                    };
    
                    // we should keep sending until a consumer receive the data!
                    tokio::spawn(async move{
                        let mut int = tokio::time::interval(tokio::time::Duration::from_secs(1));
                        loop{
                            int.tick().await;
                            let getSubs: RedisResult<u64> = redis_conn.publish(channel.clone(), finalData.clone()).await;
                            let subs = getSubs.unwrap();
                            if subs >= 1{
                                log::info!("Message has been published to Redis PubSub Channel");
                                break;
                            }
                        }
                    });

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
                    "NotifBrokerActor.publishToRedis.redis_pool", // method
                    Some(&zerlog_producer_actor)
                ).await;
            }
        }

    }
    
    /* ******************************************************************************** */
    /* ***************************** CONSUMING FROM REDIS ***************************** */
    /* ******************************************************************************** */
    pub async fn consumeFromRedis(&self, channel: &str, decryption_config: Option<CryptoConfig>, redis_cache_exp: u64){

        let storage = self.app_storage.clone();
        let rmq_pool = storage.as_ref().unwrap().get_lapin_pool().await.unwrap();
        let redis_pubsubs_conn = storage.as_ref().unwrap().get_async_redis_pubsub_conn().await.unwrap();
        let redis_pool = storage.unwrap().get_redis_pool().await.unwrap();
        let notif_mutator_actor = self.notif_mutator_actor.clone();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        // will be used to send received notif data from the broker to ws mpsc channel, 
        // later we can receive the notif in ws server setup and send it to the owner
        let notif_data_sender = self.notif_broker_sender.clone(); 

        // first thing first check the redis is up!
        match redis_pool.get().await{
            Ok(mut redis_conn) => {

                // try to create the secure cell config and 
                // do passphrase and secret key validation logic before
                // consuming messages
                let mut secure_cell_config = if let Some(mut config) = decryption_config.clone(){
            
                    config.secret = hex::encode(config.secret);
                    config.passphrase = hex::encode(config.passphrase);

                    // return the loaded instance from redis
                    SecureCellConfig{
                        secret_key: config.secret,
                        passphrase: config.passphrase,
                        data: vec![],
                    }

                } else{
                    SecureCellConfig::default()
                };

                // use redis async for handling realtime streaming of events
                let get_streamer = redis_pubsubs_conn
                    .subscribe(channel)
                    .await;

                let Ok(mut pubsubstream) = get_streamer else{
                    let e = get_streamer.unwrap_err();
                    let source = &e.source().unwrap().to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                    let err_instance = crate::error::HoopoeErrorResponse::new(
                        *STORAGE_IO_ERROR_CODE, // error hex (u16) code
                        source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                        crate::error::ErrorKind::Storage(crate::error::StorageError::RedisAsync(e)), // the actual source of the error caused at runtime
                        &String::from("NotifBrokerActor.consumeFromRedis.get_streamer"), // current method name
                        Some(&zerlog_producer_actor)
                    ).await;
                    return; // terminate the caller
                };
                
                let cloned_notif_data_sender_channel = notif_data_sender.clone();
                tokio::spawn(async move{
                    // realtime streaming over redis channel to receive emitted notifs 
                    while let Some(message) = pubsubstream.next().await{
                        let resp_val = message.unwrap();
                        let mut channelData = String::from_resp(resp_val).unwrap(); // this is the expired key
                        
                        let mut secure_cell_config = secure_cell_config.clone();
                        let cloned_notif_data_sender_channel = cloned_notif_data_sender_channel.clone();
                        let zerlog_producer_actor = zerlog_producer_actor.clone();
                        let notif_mutator_actor = notif_mutator_actor.clone();
                        let redis_pool = redis_pool.clone();

                        // potentially we're handling each message in light thread in the 
                        // background asyncly to have concurrency
                        tokio::spawn(async move{

                            log::info!("Message has been Received from Redis PubSub Channel: {}", channelData);
                            // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                            // ===>>>===>>>===>>>===>>>===>>> data decryption logic ===>>>===>>>===>>>===>>>===>>>
                            // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                            // if we have a config means the data has been encrypted
                            let finalData = if 
                                !secure_cell_config.clone().secret_key.is_empty() && 
                                !secure_cell_config.clone().passphrase.is_empty(){
                                    
                                // after calling decrypt method has changed and now contains raw string
                                channelData.decrypt(&mut secure_cell_config);
        
                                // channelData is now raw string which can be decoded into the NotifData structure
                                channelData
        
                            } else{
                                // no decryption config is needed, just return the raw data
                                // there would be no isse with decoding this into NotifData
                                log::error!("secure_cell_config is empty, data is not encrypted");
                                channelData
                            };
                            // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                            // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                            // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>

                            // either decrypted or the raw data as string
                            log::info!("[*] received data: {}", finalData);
                                                            
                            // decoding the string data into the NotifData structure (convention)
                            let get_notif_event = serde_json::from_str::<NotifData>(&finalData);
                            match get_notif_event{
                                Ok(notif_event) => {

                                    log::info!("[*] deserialized data: {:?}", notif_event);

                                    // =================================================================
                                    /* -------------------------- send to mpsc channel for ws streaming
                                    // =================================================================
                                        this is the most important part in here, we're slightly sending the data
                                        to downside of a jobq mpsc channel, the receiver eventloop however will 
                                        receive data in websocket handler which enables us to send realtime data 
                                        received from RMQ to the browser through websocket server: RMQ over websocket
                                        once we receive the data from the mpsc channel in websocket handler we'll 
                                        send it to the browser through websocket channel.
                                    */
                                    if let Err(e) = cloned_notif_data_sender_channel.send(finalData).await{
                                        log::error!("can't send notif data to websocket channel due to: {}", e.to_string());
                                    }

                                    // =============================================================================
                                    // ------------- if the cache on redis flag was activated we then store on redis
                                    // =============================================================================
                                    if redis_cache_exp != 0{
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
                                                                    "NotifBrokerActor.consumeFromRedis.decode_serde_redis", // method
                                                                    Some(&zerlog_producer_actor)
                                                                ).await;
                                                                return; // terminate the caller
                                                            }
                                                        }
                                        
                                                    },
                                                    Err(e) => {
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
                                                        let _: () = redis_conn.set_ex(&redis_notif_key.clone(), &events_string, redis_cache_exp).await.unwrap();
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
                                                    "NotifBrokerActor.consumeFromRedis.redis_pool", // method
                                                    Some(&zerlog_producer_actor)
                                            ).await;
                                                return; // terminate the caller
                                            }
                                        }
                                    }

                                    // =============================================================================
                                    // ------------- store in database
                                    // =============================================================================
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
                                                            let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                                                            let err_instance = crate::error::HoopoeErrorResponse::new(
                                                                *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                                                                source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                                                crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                                                                &String::from("NotifBrokerActor.consumeFromRedis.notif_mutator_actor.send"), // current method name
                                                                Some(&zerlog_producer_actor)
                                                            ).await;
                                                            return;
                                                        }
                                                    }
                                            }
                                        }
                                    );

                                },
                                Err(e) => {
                                    log::error!("[!] can't deserialized into NotifData struct");
                                    use crate::error::{ErrorKind, HoopoeErrorResponse};
                                    let error_content = &e.to_string();
                                    let error_content_ = error_content.as_bytes().to_vec();
                                    let mut error_instance = HoopoeErrorResponse::new(
                                        *constants::CODEC_ERROR_CODE, // error code
                                        error_content_, // error content
                                        ErrorKind::Codec(crate::error::CodecError::Serde(e)), // error kind
                                        "NotifBrokerActor.consumeFromRedis.decode_serde", // method
                                        Some(&zerlog_producer_actor)
                                    ).await;
                                    return; // terminate the caller
                                }
                            }
                        });
                        
                        
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
                    "NotifBrokerActor.consumeFromRedis.redis_pool", // method
                    Some(&zerlog_producer_actor)
                ).await;
            }
        }

    }

    /* ******************************************************************************** */
    /* ***************************** PUBLISHING TO KAFKA ****************************** */
    /* ******************************************************************************** */
    pub async fn publishToKafka(&self, 
        topic: &str, notif_data: NotifData, 
        encryptionConfig: Option<CryptoConfig>, 
        partitions: u64, brokers: &str, 
        kafkaHeaders: Vec<KafkaHeader>){

        let storage = self.app_storage.clone();
        let rmq_pool = storage.as_ref().unwrap().get_lapin_pool().await.unwrap();
        let redis_pool = storage.unwrap().get_redis_pool().await.unwrap();
        let notif_mutator_actor = self.notif_mutator_actor.clone();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        // will be used to send received notif data from the broker to ws mpsc channel, 
        // later we can receive the notif in ws server setup and send it to the owner
        let notif_data_sender = self.notif_broker_sender.clone(); 

        // since the redis is important, so we can't move forward without it 
        match redis_pool.get().await{
            Ok(mut redis_conn) => {

                // we can't move a pointer (&str in this case) which has shorter liftime 
                // into tokio spawn scope we should either convert it into String or 
                // use 'static lifetime to live longer than the spawn scope. 
                let topic = topic.to_string();
                let brokers = brokers.to_string();
                let kafkaHeaders = kafkaHeaders.clone();
                
                tokio::spawn(async move{

                    // this will be used as a unique key for the partition to send messages to
                    // it means that send all messages related to this owner to a partition
                    // with the ownerId as the key. partitioning per owner it's like creating 
                    // a queue for each user consumer to receive messages from RMQ 
                    let partitionKey = notif_data.clone().receiver_info; 
                    
                    let mut dataString = serde_json::to_string(&notif_data).unwrap();
                    let finalData = if encryptionConfig.is_some(){
                        
                        let CryptoConfig{ secret, passphrase } = encryptionConfig.clone().unwrap();
                        let mut secure_cell_config = &mut wallexerr::misc::SecureCellConfig{
                            secret_key: hex::encode(secret),
                            passphrase: hex::encode(passphrase),
                            data: vec![],
                        };
    
                        // after calling encrypt method dataString has changed and contains the hex encrypted data
                        dataString.encrypt(secure_cell_config);
                        
                        dataString
    
                    } else{
                        dataString
                    };

                    // creating the producer to produce the message asyncly
                    let createProducer: Result<FutureProducer, KafkaError> = ClientConfig::new()
                        .set("bootstrap.servers", brokers)
                        .set("message.timeout.ms", "5000")
                        .set("queue.buffering.max.ms", "0")
                        .create();
                    
                    match createProducer{
                        Ok(producer) => {

                            /* -ˋˏ✄┈┈┈┈
                                >_ produce notif_data in a ligth thread in the background asyncly
                                tokio runtime schedule each async task of producing in a light
                                thread in the background ensures truely async execution without
                                blocking happens. 
                                we're not awaiting on the spawn task, this let the task which 
                                contains async move{} block gets executed in the background thread 
                                the async move{} block however might contains some await operations
                                but the whole task will be executed asyncly in a none blocking manner
                                in the background light thread.
                                don't await on a future object let that to get executed in the 
                                background inside a light thread like tokio::spawn(async move{});
                                if you need data inside the spawn scope use channels or you can 
                                await on it which it tells runtime suspend the function execution and 
                                schedule to notify the caller once the result is ready but don't block 
                                the thread let the thread continue executing other tasks in itself.
                            */
                            tokio::spawn(async move{

                                // sending the message to the topic queue, topic in here is like the exchange 
                                // in rmq, it collects all the messages in itself.
                                let deliveryStatus = producer.send(
                                    FutureRecord::to(&topic)
                                        .payload(&finalData)
                                        .key(&partitionKey)
                                        .timestamp(chrono::Local::now().timestamp())
                                        .headers(
                                            {
                                                let mut headers = OwnedHeaders::new();
                                                for header in kafkaHeaders.iter(){
                                                    headers.clone().insert(Header{
                                                        key: &header.key,
                                                        value: Some(&header.val)
                                                    });
                                                }
                                                headers
                                            }
                                        ),
                                        std::time::Duration::from_secs(0) // no timeout for the queue
                                ).await;

                                match deliveryStatus{
                                    /* 
                                        messages go to a topic then topic divided into partitions (has been specified)
                                        so it means messages will be sent to partitions based on specified rules like 
                                        number of partitions and partition key, hence if a message is deliverred successfully
                                        kafka returns partition number that the message was sent to and the offset of 
                                        the message which is the id of the message shows the message was stored. 
                                    */
                                    Ok((partition, offset)) => {
                                        /* 
                                            partition: The partition number the message was delivered to.
                                            offset   : The offset within that partition where the message was stored.
                                        */
                                        log::info!("Message delivered to partition: {}", partition);
                                        log::info!("Message offset: {}", offset);
                                    },
                                    Err((e, om)) => {
                                        
                                        // load the Message trait enables us to call payload() and key() methods
                                        // on the owned message instance.
                                        use rdkafka::Message;
                                        // om is the message that is failed to deliver
                                        log::error!("Kafa couldn't send the message:");
                                        log::error!("Original message payload: {:?}", om.payload());
                                        log::error!("Original message key: {:?}", om.key());

                                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                                        let error_content = &e.to_string();
                                        let error_content_ = error_content.as_bytes().to_vec();
                                        let mut error_instance = HoopoeErrorResponse::new(
                                            *constants::STORAGE_IO_ERROR_CODE, // error code
                                            error_content_, // error content
                                            ErrorKind::Storage(crate::error::StorageError::Kafka(e)), // error kind
                                            "NotifBrokerActor.publishToKafka.deliveryStatus", // method
                                            Some(&zerlog_producer_actor)
                                        ).await;
                                    }
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
                                ErrorKind::Storage(crate::error::StorageError::Kafka(e)), // error kind
                                "NotifBrokerActor.publishToKafka.createProducer", // method
                                Some(&zerlog_producer_actor)
                            ).await;
                        }
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
                    "NotifBrokerActor.publishToKafka.redis_pool", // method
                    Some(&zerlog_producer_actor)
                ).await;
            }
        }

    }

    /* ******************************************************************************** */
    /* ***************************** CONSUMING FROM KAFKA ***************************** */
    /* ******************************************************************************** */
    // this message handler creates a new consumer everytime for consuming messages 
    // from the kafka topics, useful for grouping consumerws
    pub async fn consumeFromKafka(&self, 
        topics: &[String], consumerGroupId: &str, 
        decryptionConfig: Option<CryptoConfig>, 
        redis_cache_exp: u64, brokers: &str){

        let storage = self.app_storage.clone();
        let rmq_pool = storage.as_ref().unwrap().get_lapin_pool().await.unwrap();
        let redis_pool = storage.unwrap().get_redis_pool().await.unwrap();
        let notif_mutator_actor = self.notif_mutator_actor.clone();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        // will be used to send received notif data from the broker to ws mpsc channel, 
        // later we can receive the notif in ws server setup and send it to the owner
        let notif_data_sender = self.notif_broker_sender.clone(); 

        // since the redis is important, so we can't move forward without it 
        match redis_pool.get().await{
            Ok(mut redis_conn) => {

                let mut secure_cell_config = if let Some(mut config) = decryptionConfig.clone(){

                    config.secret = hex::encode(config.secret);
                    config.passphrase = hex::encode(config.passphrase);

                    // return the loaded instance from redis
                    SecureCellConfig{
                        secret_key: config.secret,
                        passphrase: config.passphrase,
                        data: vec![],
                    }

                } else{
                    SecureCellConfig::default()
                };

                let cloned_notif_data_sender_channel = notif_data_sender.clone();

                // creating the consumer to consume messages from an specific topic asyncly
                let createConsumer: Result<StreamConsumer, KafkaError> = ClientConfig::new()
                    .set("bootstrap.servers", brokers)
                    .set("enable.partition.eof", "false")
                    .set("session.timeout.ms", "6000")
                    .set("enable.auto.commit", "true")
                    .set("group.id", format!("NotifBrokerActor-{}", consumerGroupId)) // consumers with this ID will be grouped together
                    .create();
                
                match createConsumer{
                    Ok(consumer) => {

                        let topics = {
                            topics
                                .into_iter()
                                .map(|t| t.as_str())
                                .collect::<Vec<&str>>()
                        };
                        consumer.subscribe(&topics).unwrap();

                        /* -ˋˏ✄┈┈┈┈
                            >_ consume notif_data in a ligth thread in the background asyncly
                            tokio runtime schedule each async task of producing in a light
                            thread in the background ensures truely async execution without
                            blocking happens. 
                            we're not awaiting on the spawn task, this let the task which 
                            contains async move{} block gets executed in the background thread 
                            the async move{} block however might contains some await operations
                            but the whole task will be executed asyncly in a none blocking manner
                            in the background light thread.
                            don't await on a future object let that to get executed in the 
                            background inside a light thread like tokio::spawn(async move{});
                            if you need data inside the spawn scope use channels or you can 
                            await on it which it tells runtime suspend the function execution and 
                            schedule to notify the caller once the result is ready but don't block 
                            the thread let the thread continue executing other tasks in itself.
                        */
                        tokio::spawn(async move{

                            // streaming over the consumer to receive messages from the topics
                            while let Ok(message) = consumer.recv().await{

                                // it uses std::str::from_utf8() to convert utf8 bytes into string
                                let mut consumedBuffer = message.payload().unwrap();
                                let hexed_data = std::str::from_utf8(consumedBuffer).unwrap();
                                let mut payload = hexed_data.to_string();

                                log::info!("Received Message from Kafka Broker");
                                log::info!("key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                                    message.key(), payload, message.topic(), message.partition(), message.offset(), message.timestamp());

                                if let Some(headers) = message.headers(){
                                    for header in headers.iter(){
                                        log::info!("Message Header {:#?}: {:?}", header.key, header.value);
                                    }
                                }
                                
                                // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                // ===>>>===>>>===>>>===>>>===>>> data decryption logic ===>>>===>>>===>>>===>>>===>>>
                                // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                // if we have a config means the data has been encrypted
                                let finalData = if 
                                    !secure_cell_config.clone().secret_key.is_empty() && 
                                    !secure_cell_config.clone().passphrase.is_empty(){
                                        
                                    // after calling decrypt method has changed and now contains raw string
                                    // payload must be mutable since the method mutate the content after decrypting
                                    payload.decrypt(&mut secure_cell_config);

                                    // payload is now raw string which can be decoded into the NotifData structure
                                    payload

                                } else{
                                    // no decryption config is needed, just return the raw data
                                    // there would be no isse with decoding this into NotifData
                                    log::error!("secure_cell_config is empty, data is not encrypted");
                                    payload
                                };
                                // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>

                                // either decrypted or the raw data as string
                                log::info!("[*] received data: {}", finalData);
                                                                
                                // decoding the string data into the NotifData structure (convention)
                                let get_notif_event = serde_json::from_str::<NotifData>(&finalData);
                                match get_notif_event{
                                    Ok(notif_event) => {
        
                                        log::info!("[*] deserialized data: {:?}", notif_event);
        
                                        // =================================================================
                                        /* -------------------------- send to mpsc channel for ws streaming
                                        // =================================================================
                                            this is the most important part in here, we're slightly sending the data
                                            to downside of a jobq mpsc channel, the receiver eventloop however will 
                                            receive data in websocket handler which enables us to send realtime data 
                                            received from RMQ to the browser through websocket server: RMQ over websocket
                                            once we receive the data from the mpsc channel in websocket handler we'll 
                                            send it to the browser through websocket channel.
                                        */
                                        if let Err(e) = cloned_notif_data_sender_channel.send(finalData).await{
                                            log::error!("can't send notif data to websocket channel due to: {}", e.to_string());
                                        }
        
                                        // =============================================================================
                                        // ------------- if the cache on redis flag was activated we then store on redis
                                        // =============================================================================
                                        if redis_cache_exp != 0{
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
                                                                        "NotifBrokerActor.consumeFromKafka.decode_serde_redis", // method
                                                                        Some(&zerlog_producer_actor)
                                                                    ).await;
                                                                    return; // terminate the caller
                                                                }
                                                            }
                                            
                                                        },
                                                        Err(e) => {
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
                                                            let _: () = redis_conn.set_ex(&redis_notif_key.clone(), &events_string, redis_cache_exp).await.unwrap();
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
                                                        "NotifBrokerActor.consumeFromKafka.redis_pool", // method
                                                        Some(&zerlog_producer_actor)
                                                ).await;
                                                    return; // terminate the caller
                                                }
                                            }
                                        }
        
                                        // =============================================================================
                                        // ------------- store in database
                                        // =============================================================================
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
                                                                let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
                                                                let err_instance = crate::error::HoopoeErrorResponse::new(
                                                                    *MAILBOX_CHANNEL_ERROR_CODE, // error hex (u16) code
                                                                    source.as_bytes().to_vec(), // text of error source in form of utf8 bytes
                                                                    crate::error::ErrorKind::Actor(crate::error::ActixMailBoxError::Mailbox(e)), // the actual source of the error caused at runtime
                                                                    &String::from("NotifBrokerActor.consumeFromKafka.notif_mutator_actor.send"), // current method name
                                                                    Some(&zerlog_producer_actor)
                                                                ).await;
                                                                return;
                                                            }
                                                        }
                                                }
                                            }
                                        );

                                        // commit the message since it has been processed
                                        consumer.commit_message(&message, CommitMode::Async).unwrap();
        
                                    },
                                    Err(e) => {
                                        log::error!("[!] can't deserialized into NotifData struct");
                                        use crate::error::{ErrorKind, HoopoeErrorResponse};
                                        let error_content = &e.to_string();
                                        let error_content_ = error_content.as_bytes().to_vec();
                                        let mut error_instance = HoopoeErrorResponse::new(
                                            *constants::CODEC_ERROR_CODE, // error code
                                            error_content_, // error content
                                            ErrorKind::Codec(crate::error::CodecError::Serde(e)), // error kind
                                            "NotifBrokerActor.consumeFromKafka.decode_serde", // method
                                            Some(&zerlog_producer_actor)
                                        ).await;
                                        return; // terminate the caller
                                    }
                                }
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
                            ErrorKind::Storage(crate::error::StorageError::Kafka(e)), // error kind
                            "NotifBrokerActor.consumeFromKafka.createConsumer", // method
                            Some(&zerlog_producer_actor)
                        ).await;
                    }
                }

            },
            Err(e) => {
                use crate::error::{ErrorKind, HoopoeErrorResponse};
                let error_content = &e.to_string();
                let error_content_ = error_content.as_bytes().to_vec();
                let mut error_instance = HoopoeErrorResponse::new(
                    *constants::STORAGE_IO_ERROR_CODE, // error code
                    error_content_, // error content
                    ErrorKind::Storage(crate::error::StorageError::RedisPool(e)), // error kind
                    "NotifBrokerActor.consumeFromKafka.redis_pool", // method
                    Some(&zerlog_producer_actor)
                ).await;
            }
        }

    }
    
    /* ********************************************************************* */
    /* ***************************** CONSUMING ***************************** */
    /* ********************************************************************* */
    // if a consumer service wants to read notifs received from the rmq it 
    // needs to either fetch from redis or db the method doesn't return 
    // anything back to the caller
    pub async fn consumeFromRmq(&self, exp_seconds: u64,
        consumer_tag: &str, queue: &str, 
        binding_key: &str, exchange: &str,
        store_in_db: bool,
        decryptionConfig: Option<CryptoConfig>
    ){

        let storage = self.app_storage.clone();
        let rmq_pool = storage.as_ref().unwrap().get_lapin_pool().await.unwrap();
        let redis_pool = storage.unwrap().get_redis_pool().await.unwrap();
        let notif_mutator_actor = self.notif_mutator_actor.clone();
        let zerlog_producer_actor = self.clone().zerlog_producer_actor;
        let notif_data_sender = self.notif_broker_sender.clone();
        
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
                        let cloned_notif_data_sender_channel = notif_data_sender.clone();

                        /* =================================================================================== */
                        /* ================================ CONSUMING PROCESS ================================ */
                        /* =================================================================================== */
                        // start consuming in the background in a lightweight thread of execution
                        // receiving is considered to be none blocking which won't block the current thread. 
                        tokio::spawn(async move{

                            // try to create the secure cell config and 
                            // do passphrase and secret key validation logic before
                            // consuming messages
                            let mut secure_cell_config = if let Some(mut config) = decryptionConfig.clone(){
                        
                                config.secret = hex::encode(config.secret);
                                config.passphrase = hex::encode(config.passphrase);

                                // return the loaded instance from redis
                                SecureCellConfig{
                                    secret_key: config.secret,
                                    passphrase: config.passphrase,
                                    data: vec![],
                                }

                            } else{
                                SecureCellConfig::default()
                            };

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

                                                log::info!("[*] received delivery from queue#<{}>", q.name());
                                                let consumedBuffer = delv.data.clone();
                                                let hexed_data = std::str::from_utf8(&consumedBuffer).unwrap();
                                                let mut payload = hexed_data.to_string();

                                                let mut secure_cell_config = secure_cell_config.clone();
                                                let cloned_notif_data_sender_channel = cloned_notif_data_sender_channel.clone();
                                                let redis_pool = redis_pool.clone();
                                                let notif_mutator_actor = notif_mutator_actor.clone();
                                                let cloned_zerlog_producer_actor = zerlog_producer_actor.clone();

                                                // handle the message in the background light thread asyncly and concurrently
                                                // by spawning a light thread for the async task
                                                tokio::spawn(async move{

                                                    // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                                    // ===>>>===>>>===>>>===>>>===>>> data decryption logic ===>>>===>>>===>>>===>>>===>>>
                                                    // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                                    // if we have a config means the data has been encrypted
                                                    let finalData = if 
                                                        !secure_cell_config.clone().secret_key.is_empty() && 
                                                        !secure_cell_config.clone().passphrase.is_empty(){
                                                            
                                                        // after calling decrypt method has changed and now contains raw string
                                                        // payload must be mutable since the method mutate the content after decrypting
                                                        payload.decrypt(&mut secure_cell_config);

                                                        // payload is now raw string which can be decoded into the NotifData structure
                                                        payload

                                                    } else{
                                                        // no decryption config is needed, just return the raw data
                                                        // there would be no isse with decoding this into NotifData
                                                        log::error!("secure_cell_config is empty, data is not encrypted");
                                                        payload
                                                    };
                                                    // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                                    // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>
                                                    // ===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>>===>>

                                                    // either decrypted or the raw data as string
                                                    log::info!("[*] received data: {}", finalData);
                                                    
                                                    // decoding the string data into the NotifData structure (convention)
                                                    let get_notif_event = serde_json::from_str::<NotifData>(&finalData);
                                                    match get_notif_event{
                                                        Ok(notif_event) => {

                                                            log::info!("[*] deserialized data: {:?}", notif_event);

                                                            // =================================================================
                                                            /* -------------------------- send to mpsc channel for ws streaming
                                                            // =================================================================
                                                                this is the most important part in here, we're slightly sending the data
                                                                to downside of a jobq mpsc channel, the receiver eventloop however will 
                                                                receive data in websocket handler which enables us to send realtime data 
                                                                received from RMQ to the browser through websocket server: RMQ over websocket
                                                                once we receive the data from the mpsc channel in websocket handler we'll 
                                                                send it to the browser through websocket channel.
                                                            */
                                                            if let Err(e) = cloned_notif_data_sender_channel.send(finalData).await{
                                                                log::error!("can't send notif data to websocket channel due to: {}", e.to_string());
                                                            }

                                                            // =============================================================================
                                                            // ------------- if the cache on redis flag was activated we then store on redis
                                                            // =============================================================================
                                                            if exp_seconds != 0{
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
                                                                                            Some(&cloned_zerlog_producer_actor)
                                                                                        ).await;
                                                                                        return; // terminate the caller
                                                                                    }
                                                                                }
                                                                
                                                                            },
                                                                            Err(e) => {
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
                                                                            Some(&cloned_zerlog_producer_actor)
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
                                                                        let zerlog_producer_actor = cloned_zerlog_producer_actor.clone();
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
                                                                                        let source = &e.to_string(); // we know every goddamn type implements Error trait, we've used it here which allows use to call the source method on the object
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
                                                            log::error!("[!] can't deserialized into NotifData struct");
                                                            use crate::error::{ErrorKind, HoopoeErrorResponse};
                                                            let error_content = &e.to_string();
                                                            let error_content_ = error_content.as_bytes().to_vec();
                                                            let mut error_instance = HoopoeErrorResponse::new(
                                                                *constants::CODEC_ERROR_CODE, // error code
                                                                error_content_, // error content
                                                                ErrorKind::Codec(crate::error::CodecError::Serde(e)), // error kind
                                                                "NotifBrokerActor.decode_serde", // method
                                                                Some(&cloned_zerlog_producer_actor)
                                                            ).await;
                                                            return; // terminate the caller
                                                        }
                                                    }
                                                });

                                                // acking here means we have processed payload successfully
                                                match delv.ack(BasicAckOptions::default()).await{
                                                    Ok(ok) => { /* acked */ },
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
    pub async fn publishToRmq(&self, data: &str, exchange: &str, routing_key: &str, exchange_type: &str){

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
            let rmq_pool = storage.as_ref().unwrap().get_lapin_pool().await.unwrap();
            let redis_pool = storage.as_ref().unwrap().get_redis_pool().await.unwrap();
            
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
                                // can be consumed from the queue by the consumer.
                                // in direct exchange ("") it has assumed that the queue
                                // name is the same as the routing key name.
                                use deadpool_lapin::lapin::options::BasicPublishOptions;
                                let payload = data.as_bytes();
                                match chan
                                    .basic_publish(
                                        &exchange, // messages go in there
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

                                            if confirmation.is_ack(){
                                                log::info!("publisher confirmation is acked");
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
        zerlog_producer_actor: Addr<ZerLogProducerActor>,
        notif_broker_sender: tokio::sync::mpsc::Sender<String>) -> Self{
        Self { app_storage, notif_mutator_actor, zerlog_producer_actor, notif_broker_sender }
    }

}

/* ********************************************************************************* */
/* ***************************** PRODUCE NOTIF HANDLER ***************************** */
/* ********************************************************************************* */
impl ActixMessageHandler<PublishNotifToRmq> for NotifBrokerActor{
    
    type Result = ();
    fn handle(&mut self, msg: PublishNotifToRmq, ctx: &mut Self::Context) -> Self::Result {;

        // unpacking the notif data
        let PublishNotifToRmq { 
                exchange_name,
                exchange_type,
                routing_key,
                local_spawn,
                notif_data,
                encryptionConfig,

            } = msg.clone();
        
        let this = self.clone();
        
        let mut stringData = serde_json::to_string(&notif_data).unwrap();
        let finalData = if encryptionConfig.is_some(){
                        
            let CryptoConfig{ secret, passphrase } = encryptionConfig.clone().unwrap();
            let mut secure_cell_config = &mut wallexerr::misc::SecureCellConfig{
                secret_key: hex::encode(secret),
                passphrase: hex::encode(passphrase),
                data: vec![],
            };

            // after calling encrypt method stringData has changed and contains the hex encrypted data
            stringData.encrypt(secure_cell_config);
            
            stringData

        } else{
            stringData
        };


        // spawn the future in the background into the given actor context thread
        // by doing this we're executing the future inside the actor thread since
        // every actor has its own thread of execution.
        if local_spawn{
            async move{
                this.publishToRmq(&finalData, &exchange_name, &routing_key, &exchange_type).await;
            }
            .into_actor(self) // convert the future into an actor future of type NotifBrokerActor
            .spawn(ctx); // spawn the future object into this actor context thread
        } else{ // spawn the future in the background into the tokio lightweight thread
            tokio::spawn(async move{
                this.publishToRmq(&finalData, &exchange_name, &routing_key, &exchange_type).await;
            });
        }
        
        return;
        
    }

}

/* ********************************************************************************* */
/* ***************************** CONSUME NOTIF HANDLER ***************************** */
/* ********************************************************************************* */
impl ActixMessageHandler<ConsumeNotifFromRmq> for NotifBrokerActor{
    
    type Result = ();
    fn handle(&mut self, msg: ConsumeNotifFromRmq, ctx: &mut Self::Context) -> Self::Result {

        // unpacking the consume data
        let ConsumeNotifFromRmq { 
                queue, 
                tag,
                exchange_name,
                routing_key,
                redis_cache_exp,
                local_spawn,
                store_in_db,
                decryptionConfig

            } = msg.clone(); // the unpacking pattern is always matched so if let ... is useless
        
        let this = self.clone();
        
        // spawn the future in the background into the given actor context thread
        // by doing this we're executing the future inside the actor thread since
        // every actor has its own thread of execution.
        if local_spawn{
            async move{
                this.consumeFromRmq(
                    redis_cache_exp, 
                    &tag, 
                    &queue, 
                    &routing_key, 
                    &exchange_name,
                    store_in_db,
                    decryptionConfig
                ).await;
            }
            .into_actor(self) // convert the future into an actor future of type NotifBrokerActor
            .spawn(ctx); // spawn the future object into this actor context thread
        } else{ // spawn the future in the background into the tokio lightweight thread
            tokio::spawn(async move{
                this.consumeFromRmq(
                    redis_cache_exp, 
                    &tag, 
                    &queue, 
                    &routing_key, 
                    &exchange_name,
                    store_in_db,
                    decryptionConfig
                ).await;
            });
        }
        return; // terminate the caller

    }

}

impl ActixMessageHandler<HealthMsg> for NotifBrokerActor{
    type Result = ();
    fn handle(&mut self, msg: HealthMsg, ctx: &mut Self::Context) -> Self::Result {
        if msg.shutdown{
            ctx.stop(); // stop the already running actor
        }
    }
}

/* **************************************************************************************** */
/* ***************************** REDIS PRODUCER NOTIF HANDLER ***************************** */
/* **************************************************************************************** */
impl ActixMessageHandler<PublishNotifToRedis> for NotifBrokerActor{
   
    type Result = ();
    
    fn handle(&mut self, msg: PublishNotifToRedis, ctx: &mut Self::Context) -> Self::Result { 

        
        let PublishNotifToRedis{ 
            channel, 
            local_spawn,
            notif_data, 
            encryptionConfig 
        } = msg.clone();

        let this = self.clone();
        let task = async move{
            
            // await on the publishing task, tells runtime we need the result 
            // of this task, if it's ready runtime return the result back to 
            // the caller otherwise suspend the this.publishToRedis() function
            // until the task is ready to be polled, meanwhile it executes other
            // tasks (won't block the thread)
            this.publishToRedis(&channel, notif_data, encryptionConfig).await;

        };  

        // spawn the task in the background ligh thread or 
        // the actor thread itself.
        // don't await on the spawn; let the task execute 
        // in the background unless you want to use select
        // or tell the runtime someone needs the result right
        // now but notify later once the task is completed 
        // and don't block the thread
        if local_spawn{
            task
                .into_actor(self)
                .spawn(ctx);
        } else{
            spawn(task);
        }
        
    }

}

/* **************************************************************************************** */
/* ***************************** REDIS CONSUMER NOTIF HANDLER ***************************** */
/* **************************************************************************************** */
impl ActixMessageHandler<ConsumeNotifFromRedis> for NotifBrokerActor{
    
    type Result = ();
    
    fn handle(&mut self, msg: ConsumeNotifFromRedis, ctx: &mut Self::Context) -> Self::Result {

        let ConsumeNotifFromRedis { channel, decryptionConfig, redis_cache_exp } = msg.clone();
        let this = self.clone();

        let task = async move{

            // await on the consuming task, tells runtime we need the result 
            // of this task, if it's ready runtime return the result back to 
            // the caller otherwise suspend the this.publishToRedis() function
            // until the task is ready to be polled, meanwhile it executes other
            // tasks (won't block the thread)
            this.consumeFromRedis(&channel, decryptionConfig, redis_cache_exp).await;

        };

        // spawn the task in the background ligh thread 
        // don't await on the spawn; let the task execute 
        // in the background unless you want to use select
        // or tell the runtime someone needs the result right
        // now but notify later once the task is completed 
        // and don't block the thread
        spawn(task);
    }
}

/* **************************************************************************************** */
/* ***************************** KAFKA PRODUCER NOTIF HANDLER ***************************** */
/* **************************************************************************************** */
impl ActixMessageHandler<PublishNotifToKafka> for NotifBrokerActor{
    
    type Result = ();
    
    fn handle(&mut self, msg: PublishNotifToKafka, ctx: &mut Self::Context) -> Self::Result {

        let PublishNotifToKafka{ 
            topic, 
            local_spawn, 
            brokers, 
            notif_data, 
            encryptionConfig, 
            partitions,
            headers, 
        } = msg.clone();

        log::info!("PublishNotifToKafka : {:#?}", msg.clone());
        
        let this = self.clone();
        let task = async move{

            // await on the publishing task, tells runtime we need the result 
            // of this task, if it's ready runtime return the result back to 
            // the caller otherwise suspend the this.publishToRedis() function
            // until the task is ready to be polled, meanwhile it executes other
            // tasks (won't block the thread)
            this.publishToKafka(&topic, notif_data, encryptionConfig, partitions, &brokers, headers).await;

        };

        // spawn the task in the background ligh thread or 
        // the actor thread itself.
        // don't await on the spawn; let the task execute 
        // in the background unless you want to use select
        // or tell the runtime someone needs the result right
        // now but notify later once the task is completed 
        // and don't block the thread
        if local_spawn{
            task
                .into_actor(self)
                .spawn(ctx);
        } else{
            spawn(task);
        }
        
    }
}

/* **************************************************************************************** */
/* ***************************** KAFKA CONSUMER NOTIF HANDLER ***************************** */
/* **************************************************************************************** */
impl ActixMessageHandler<ConsumeNotifFromKafka> for NotifBrokerActor{
    
    type Result = ();
    
    fn handle(&mut self, msg: ConsumeNotifFromKafka, ctx: &mut Self::Context) -> Self::Result {
        
        let ConsumeNotifFromKafka{ topics, brokers, consumerGroupId, decryptionConfig, redis_cache_exp } = msg.clone();

        let this = self.clone();
        let task = async move{
            
            // await on the consuming task, tells runtime we need the result 
            // of this task, if it's ready runtime return the result back to 
            // the caller otherwise suspend the this.publishToRedis() function
            // until the task is ready to be polled, meanwhile it executes other
            // tasks (won't block the thread)
            this.consumeFromKafka(&topics, &consumerGroupId, decryptionConfig, redis_cache_exp, &brokers).await;

        };

        // spawn the task in the background ligh thread or 
        // the actor thread itself.
        // don't await on the spawn; let the task execute 
        // in the background unless you want to use select
        // or tell the runtime someone needs the result right
        // now but notify later once the task is completed 
        // and don't block the thread
        spawn(task);

    }

}
