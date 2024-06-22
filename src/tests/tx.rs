
use wallexerr::misc::Wallet;
use crate::interfaces::tx::TransactionPoolActorTxExt;
use crate::*;

pub static WALLET: Lazy<std::sync::Arc<wallexerr::misc::Wallet>> = Lazy::new(||{
    let wallet = Wallet::new_ed25519();
    std::sync::Arc::new(wallet) // a shareable wallet object
});

/* --------------------- a fintech object to do any financial process
    atomic tx syncing execution to avoid deadlocks, race conditions and double spendings using tokio select spawn arc mutex channels
    produce tx actor objects to rmq then consume them in txpool service and update the balance in there
    use notif prodcons actor worker to produce and consume tx objects
    send each tx object into the exchange 
    tx pool service concumes every incoming tx and execute them in the background
    safe tx execution without double spending issue using tokio spawn select mutex and channels
    finally tx pool service publishes the result of executed each tx into the exchange 

    1 => create ed25591 wallet keypairs with high entropy seed for its rng
    2 => then build tx object and use sha256 to make a hash of the stringified of tx object
    3 => sign the hash of the tx object with prvkey to create the tx signature
    4 => use pubkey and hash of tx object to verify the signature
    use hex::encode() to generate hex string from utf8 bytes
    use hex::decode() to generate utf8 bytes from hex string
    use base58 and base64 to generate base58 or base64 from utf8 bytes  
 */
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct TransactionPoolActor{
    amount: f64, 
    from: String, 
    to: String, 
    tax: f64,
    data: Vec<u8>, // every tx object can store data
    tx_type: TxType,
    treasury_type: TreasuryType,
    status: TxStatus,
    hash: String, // sha256 ash of the transaction
    tx_sig: String, // the signature result of signing the tx hash with private key, this will use to verify the tx along with the pubkey of the signer
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

impl TransactionPoolActor{
    pub fn new(amount: f64, from: &str, to: &str, tax: f64, data: &[u8]) -> Self{

        let wallet = WALLET.clone();
        let mut tx = Self{
            amount,
            from: from.to_string(),
            to: to.to_string(),
            tax,
            data: data.to_vec(), 
            status: TxStatus::Started,
            tx_type: TxType::Deposit,
            treasury_type: TreasuryType::Credit,
            hash: String::from(""), // stringify the whole tx object then hash it
            tx_sig: String::from(""), // sign the the stringified_tx_object with prvkey
            signer: String::from("") // the one who has signed with the prv key usually the server
        };


        // tx.on_error(||{});
        // tx.on_success(||{});
        // tx.on_reject(||{});

        tx
        
    }
    
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum TxStatus{
    #[default]
    Started,
    Executed,
    Rejected(FailedTx),
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct FailedTx{
    time: i64,
    cause: String
}

impl TransactionPoolActorTxExt for TransactionPoolActor{
    type Tx = Self;

    async fn execute(&self) -> Self {
        
        // update user balance and treasury logics
        // ...

        todo!()
    }

    async fn get_status(&self) -> Self {
        todo!()
    }

    async fn started(&self){}

    async fn aborted(&self){}
}