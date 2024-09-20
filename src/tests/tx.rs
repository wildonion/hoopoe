

use std::sync::atomic::AtomicU8;
use interfaces::payment::PaymentProcess;
use models::event::ActionType;
use notif::PublishNotifToRmq;
use rand_chacha::ChaCha12Core;
use salvo::rate_limiter::QuotaGetter;
use wallexerr::misc::Wallet;
use crate::interfaces::tx::TransactionExt;
use crate::*;
use actix::prelude::*;
use actix::Handler as ActixMessageHandler;



#[derive(Message, Clone)]
#[rtype(result = "()")]
struct Execute{
    pub tx: Transaction,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
struct Send2Pool{
    pub tx: Transaction,
    pub tx_producer: Addr<NotifBrokerActor>, // use this actor to send produce message to it
    pub spawn_local: bool
}

pub static WALLET: Lazy<std::sync::Arc<tokio::sync::Mutex<wallexerr::misc::Wallet>>> = Lazy::new(||{
    let wallet = Wallet::new_ed25519();
    std::sync::Arc::new(
        tokio::sync::Mutex::new( // since most of the wallet methods are mutable we need to define this to be mutex 
            wallet
        )
    ) // a safe and shareable wallet object between threads
});

/* --------------------- a fintech object solution to do any financial process
    atomic tx syncing execution to avoid deadlocks, race conditions and double spendings using tokio select spawn arc mutex channels
    produce tx actor objects to rmq then consume them in txpool service and update the balance in there
    use notif prodcons actor worker to produce and consume tx objects
    send each tx object into the exchange 
    tx pool service concumes every incoming tx and execute them in the background
    safe tx execution without double spending issue using tokio spawn select mutex and channels
    finally tx pool service publishes the result of executed each tx into the exchange 


    once a tx object is made publish it to the rmq exchange so consumer 
    can consume it for committing and executing all tx objects finally 
    produce the result to the TxResultExchange so main service can consume 
    it and update the platform based on the result

    1 => create ed25591 wallet keypairs with high entropy seed for its rng
    2 => then build tx object and use sha256 to make a hash of the stringified of tx object
    3 => sign the hash of the tx object with prvkey to create the tx signature
    4 => use pubkey and hash of tx object to verify the signature
    use hex::encode() to generate hex string from utf8 bytes
    use hex::decode() to generate utf8 bytes from hex string
    use base58 and base64 to generate base58 or base64 from utf8 bytes  

    every tx object must atomic: Arc<Mutex<Transaction>> or Arc<RwLock<Transaction>>

*/
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Transaction{
    amount: f64, 
    from: String, 
    to: String, 
    tax: f64,
    action: TxAction,
    memo: String, // this field is required when the exchange is using a single address to deposit tokens instead of a new one per each user 
    data: Vec<u8>, // every tx object can store data
    tx_type: TxType,
    treasury_type: TreasuryType,
    status: TxStatus,
    hash: Option<String>, // sha256 ash of the transaction
    tx_sig: Option<String>, // the signature result of signing the tx hash with private key, this will use to verify the tx along with the pubkey of the signer
    signer: String, // the one who has signed the tx
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum TxAction{
    UpdateCode,
    #[default]
    CallMethod,
    UpdateLibs
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum TxType{
    Withdraw,
    #[default]
    Deposit,
    Buy,
    Airdrop,
    Claim,
    Sell
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum TreasuryType{
    #[default]
    Debit, 
    Credit
}

impl Transaction{
    pub async fn new(last_tx: Self, memo: &str, amount: f64, from: &str, to: &str, tax: f64, data: &[u8]) -> Self{ // a tx object might have some data inside of itself

        // create a new tx object
        let mut tx_data = Self{
            amount,
            from: from.to_string(),
            to: to.to_string(),
            tax,
            action: TxAction::CallMethod,
            memo: memo.to_string(),
            data: data.to_vec(), 
            status: TxStatus::Started,
            tx_type: TxType::Deposit,
            treasury_type: TreasuryType::Credit,
            hash: Some(String::from("")), // stringify the whole tx object then hash it
            tx_sig: Some(String::from("")), // sign the the stringified_tx_object with prvkey
            signer: String::from("") // the one who has signed with the prv key usually the server
        };

        tx_data
        
    }

    pub fn on_error<E>(e: E) where E: FnMut() -> () + Send + Sync + 'static{ // error is of type a closure trait

    }

    pub fn on_success<S>(s: S) where S: FnMut() -> () + Send + Sync + 'static{ // success is of type a closure trait
        
    }

    pub fn on_reject<R>(r: R) where R: FnMut() -> () + Send + Sync + 'static{ // reject is of type a closure trait
        
    }

    pub fn queueTx(&mut self){
        self.status = TxStatus::Queued
    }

    pub fn pendingTx(&mut self){
        self.status = TxStatus::Pending
    }

    pub fn commitTx(&mut self){
        self.status = TxStatus::Committed
    }
    
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub enum TxStatus{
    #[default]
    Started,
    Queued,
    Pending,
    Committed,
    Dropped,
    Mined,
    Rejected(FailedTx),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct FailedTx{
    time: i64,
    cause: String
}

// drop the transaction object from the ram
// note that for Rc shared reference, the total reference count 
// of type (type pointers used by different scopes) must reaches 
// zero so the type can gets dropped out of the ram but not for weak. 
impl Drop for Transaction{
    fn drop(&mut self) {
        self.status = TxStatus::Dropped
    }
}

impl TransactionExt for Transaction{
    
    type Tx = Self;

    async fn commit(&self) -> Self {
        
        let mut tx_data = self.clone();
        let mut wallet = WALLET.clone();
        let tx_json = serde_json::json!({
            "amount": tx_data.amount,
            "from": tx_data.from,
            "to": tx_data.to,
            "tax": tx_data.tax,
            "data": tx_data.data,
            "tx_type": tx_data.tx_type,
            "treasury_type": tx_data.treasury_type,
            "signer": tx_data.signer,
        });
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Option<String>, Option<String>)>(1024);
        
        // use a lightweight thread to lock on the wallet in order to avoid blocking issues
        // basically we should do our async stuffs inside a lightweight thread then use 
        // channels to send the result to outside of the thread scope.
        tokio::spawn(
            {
                let tx = tx.clone();
                async move{
                    let data = serde_json::to_string(&tx_json).unwrap();
                    let mut locked_wallet = wallet.lock().await;
                    let prvkey = locked_wallet.ed25519_secret_key.clone().unwrap();
                    let sig = locked_wallet.self_ed25519_sign(&data, &prvkey);
                    let tx_data_hash = locked_wallet.self_generate_sha256_from(&data);
                    let hex_tx_data_hash = Some(hex::encode(&tx_data_hash));
                    tx.send((sig, hex_tx_data_hash)).await;
                }
            }
        );

        // update tx_sig and hash field simultaneously as they're coming from the background thread
        while let Some((tx_signature, tx_hash)) = rx.recv().await{
            tx_data.tx_sig = tx_signature;
            tx_data.hash = tx_hash;
        }

        // update from and to balances
        // calculate user and sys treasury
        // 2.5 percent of each tx must goes to sys treasury
        // destination amount = current amount - 2.5 % of current amount
        // sys treasury = 2.5 % of current amount 
        // ...

        tx_data.commitTx(); // commit the tx
        tx_data

    }

    async fn get_status(&self) -> Self {
        todo!()
    }

    async fn started(&self){}

    async fn aborted(&self){}

}


struct StatelessTransactionPool{
    pub lock: tokio::sync::Mutex<()>, // the pool is locked and busy 
    pub worker: tokio::sync::Mutex<tokio::task::JoinHandle<()>>, // background worker thread to execute a transaction
    pub queue: std::sync::Arc<tokio::sync::Mutex<Vec<Transaction>>>, // a queue contains all queued transactions
    pub broadcaster: tokio::sync::mpsc::Sender<Transaction>
}

impl StatelessTransactionPool{
    
    pub fn new(broadcaster: tokio::sync::mpsc::Sender<Transaction>) -> Self{
        Self { 
            lock: tokio::sync::Mutex::new(()), 
            worker: tokio::sync::Mutex::new(tokio::spawn(async move{ () })), 
            broadcaster,
            queue: std::sync::Arc::new(tokio::sync::Mutex::new(vec![]))
        }
    }

    pub async fn push(&mut self, tx: Transaction){
        let mut get_queue = self.queue.lock().await;
        (*get_queue).push(tx.clone());
        let sender = self.broadcaster.clone();
        sender.send(tx).await;
    }

}
impl Actor for StatelessTransactionPool{
    
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

    }
}

// msg handler to send the tx object to the rmq, the consumer service
// which is the tx-pool service will consume each one and execute them
// asyncly in the background worker thread
impl ActixMessageHandler<Send2Pool> for StatelessTransactionPool{
    
    type Result = ();
    fn handle(&mut self, msg: Send2Pool, ctx: &mut Self::Context) -> Self::Result {
        
        let tx = msg.tx;
        let producer = msg.tx_producer;
        let spawn_local = msg.spawn_local;
        let cloned_tx = tx.clone();

        // send only queued transaction to the pool
        if tx.status != TxStatus::Queued{
            return;
        }

        let prod_notif = 
            PublishNotifToRmq{
                local_spawn: true,
                notif_data: NotifData{ 
                    id: Uuid::new_v4().to_string(), 
                    receiver_info: String::from("txpool"), 
                    action_data: serde_json::to_value(&cloned_tx).unwrap(), 
                    actioner_info: String::from("wallet-service"), 
                    action_type: ActionType::EventCreated, 
                    fired_at: chrono::Local::now().timestamp(), 
                    is_seen: false 
                },
                exchange_name: format!("{}.notif:TxPool", APP_NAME),
                exchange_type: String::from("fanout"),
                routing_key: String::from(""),
                encryptionConfig: None, // don't encrypt the data 
            };

        // background worker thread using tokio
        let cloned_producer = producer.clone();
        let cloned_prod_notif = prod_notif.clone();
        tokio::spawn(async move{
            cloned_producer.send(cloned_prod_notif).await;
        });

        if spawn_local{
            // spawn in local thread of the actor itself
            async move{
                producer.send(prod_notif).await;
            }.into_actor(self) // convert the future into the actor so we can call the spawn method to execute the future in actor thread
            .spawn(ctx);
        }

    }

}

// send Execute message to the actor to execute a tx 
impl ActixMessageHandler<Execute> for StatelessTransactionPool{
    type Result = ();

    fn handle(&mut self, msg: Execute, ctx: &mut Self::Context) -> Self::Result {
        
        let tx = msg.tx;
        
        let cloned_tx = tx.clone();
        tokio::spawn(async move{
            cloned_tx.clone().commit().await;
        });

    }

}

pub async fn any_type_dyn_stat_dispatch(){
    
    // ---------------------------------------------------------
    //         SOLID BASED DESIGN PATTERN FOR PAYMENT
    // ---------------------------------------------------------
    // there should be always a rely solution on abstractions like 
    // implementing trait for struct and extending its behaviour 
    // instead of changing the actual code base and concrete impls. 
    struct PayPal; // paypal gateway
    struct ZarinPal; // zarinpal gateway
    
    struct PaymentWallet{
        pub owner: String,
        pub id: uuid::Uuid,
        pub transactions: Vec<Transaction> // transactions that need to be executed
    }
    
    impl PaymentProcess<PayPal> for PaymentWallet{
        type Status = AtomicU8;
    
        // future traits as objects must be completelly a separate type
        // which can be achieved by pinning the boxed version of future obj 
        type Output<O: Send + Sync + 'static> = std::pin::Pin<Box<dyn std::future::Future<Output = O>>>;
        
        type Wallet = Self;
    
        async fn pay<O: Send + Sync + 'static>(&self, gateway: PayPal) -> Self::Output<O> {
    
            // either clone or borrow it to avoid from moving out of the self 
            // cause self is behind reference which is not allowed by Rust 
            // to move it around or take its ownership.
            let txes: &Vec<Transaction> = self.transactions.as_ref();
            
            // process all transactions with the paypal gateway
            // ...
    
            todo!()
    
        }
    }
    
    impl PaymentProcess<ZarinPal> for PaymentWallet{
        type Status = AtomicU8;
    
        // future traits as objects must be completelly a separate type
        // which can be achieved by pinning the boxed version of future obj 
        type Output<O: Send + Sync + 'static> = std::pin::Pin<Box<dyn std::future::Future<Output = O>>>;
        
        type Wallet = Self;
    
        async fn pay<O: Send + Sync + 'static>(&self, gateway: ZarinPal) -> Self::Output<O> {
    
            // either clone or borrow it to avoid from moving out of the self 
            // cause self is behind reference which is not allowed by Rust 
            // to move it around or take its ownership.
            let txes: &Vec<Transaction> = self.transactions.as_ref();
            
            // process all transactions with the paypal gateway
            // ...
            
    
            todo!()
    
        }
    }

    // spawn a tokio thread for every request in a lightweight
    async fn getCode<O: Send + Sync + 'static>(param: O) 
        -> impl std::future::Future<Output=O> + Send + Sync + 'static{
        // the return type is a type which impls the trait directly through 
        // static dispatch
        async move{
            param
        }
    }
    tokio::spawn(getCode(String::from("wildonion")));

    /* 
        Access different types through a single interface to use common method of traits with either default 
        or trait implementations we can impl the trait broadly for any possible types using impl Trair for T{} 
        instead of implementing for every single type manually box pin, box dyn trait impl trait for dyn stat 
        and poly implementations.
        is, the type has been erased. As such, a dyn Trait reference contains two pointers. One pointer goes 
        to the data (e.g., an instance of a struct). Another pointer goes to a map of method call names to 
        function pointers (known as a virtual method table or vtable).
        At run-time, when a method needs to be called on the dyn Trait, the vtable is consulted to get the 
        function pointer and then that function pointer is called.
        See the Reference for more information on trait objects and object safety.
        Trade-offs
        The above indirection is the additional runtime cost of calling a function on a dyn Trait. Methods 
        called by dynamic dispatch generally cannot be inlined by the compiler.
        However, dyn Trait is likely to produce smaller code than impl Trait / generic parameters as the 
        method won't be duplicated for each concrete type.
    */
    trait AnyTypeCanBe1<T>: Send + Sync + 'static{
        fn getNickName(&self) -> String{
            String::from("")
        }
    }
    impl<T: Send + Sync + 'static> AnyTypeCanBe1<T> for T{}
    struct InGamePlayer{}
    let player = InGamePlayer{};
    player.getNickName(); // don't need to impl AnyTypeCanBe1 for InGamePlayer cause it's already implemented for any T

    
    // handling pushing into the map using trait polymorphism
    trait AnyTypeCanBe{}
    impl<T> AnyTypeCanBe for T{} // impl AnyTypeCanBe for every T, reduces the time of implementing trait
    let any_map1: std::collections::HashMap<String, Box<dyn AnyTypeCanBe + Send + Sync + 'static>>;
    let mut any_map1 = std::collections::HashMap::new();
    any_map1.insert(String::from("wildonion"), Box::new(0));
    // or 
    // any_map1.insert(String::from("wildonion"), Box::new(String::from("")));

    // to have any types we can dynamically dispatch the Any trait which is an object safe trait
    type AnyType = Box<dyn std::any::Any + Send + Sync + 'static>;
    let any_map: std::collections::HashMap<String, AnyType>; // the value can be any type impls the Any trait
    let boxed_trait_object: Box<dyn AnyTypeCanBe>; // Boxed trait object
    let arced_trait_object: std::sync::Arc<dyn AnyTypeCanBe>; // thread safe trait object

    fn getTrait(t: &(dyn AnyTypeCanBe + Send)){ // dynamic dispatch

    }
    fn getTrait1(t: impl AnyTypeCanBe + Send){ // static dispatch
        
    }


}