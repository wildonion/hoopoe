
use futures::lock;
use ractor::Message;
use rand_chacha::ChaCha12Core;
use wallexerr::misc::Wallet;
use crate::interfaces::tx::TransactionExt;
use crate::*;

pub static WALLET: Lazy<std::sync::Arc<tokio::sync::Mutex<wallexerr::misc::Wallet>>> = Lazy::new(||{
    let wallet = Wallet::new_ed25519();
    std::sync::Arc::new(
        tokio::sync::Mutex::new( // since most of the wallet methods are mutable we need to define this to be mutex 
            wallet
        )
    ) // a safe and shareable wallet object between threads
});

/* --------------------- a fintech object to do any financial process
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
 */
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Transaction{
    amount: f64, 
    from: String, 
    to: String, 
    tax: f64,
    data: Vec<u8>, // every tx object can store data
    tx_type: TxType,
    treasury_type: TreasuryType,
    status: TxStatus,
    hash: Option<String>, // sha256 ash of the transaction
    tx_sig: Option<String>, // the signature result of signing the tx hash with private key, this will use to verify the tx along with the pubkey of the signer
    signer: String, // the one who has signed the tx
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum TxType{
    Withdraw,
    #[default]
    Deposit,
    Buy,
    Airdrop,
    Claim,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum TreasuryType{
    #[default]
    Debit, 
    Credit
}

impl Transaction{
    pub async fn new(last_tx: Self, amount: f64, from: &str, to: &str, tax: f64, data: &[u8]) -> Self{ // a tx object might have some data inside of itself

        // create a new tx object
        let mut tx_data = Self{
            amount,
            from: from.to_string(),
            to: to.to_string(),
            tax,
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

    pub fn on_error<E>(e: E) where E: FnMut() -> () + Send + Sync + 'static{

    }

    pub fn on_success<S>(s: S) where S: FnMut() -> () + Send + Sync + 'static{
        
    }

    pub fn on_reject<R>(r: R) where R: FnMut() -> () + Send + Sync + 'static{
        
    }
    
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum TxStatus{
    #[default]
    Started,
    Committed,
    Dropped,
    Rejected(FailedTx),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
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

        while let Some((tx_signature, tx_hash)) = rx.recv().await{
            tx_data.tx_sig = tx_signature;
            tx_data.hash = tx_hash;
        }

        // update from and to balances and calculate user and sys treasury
        // ...

        tx_data


    }

    async fn get_status(&self) -> Self {
        todo!()
    }

    async fn started(&self){}

    async fn aborted(&self){}

}