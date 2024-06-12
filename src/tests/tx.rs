

use actix::Context;

use crate::*;

/* --------------------- a fintech object
    produce tx actor objects to rmq then consume them in txpool service and update the balance in there
    use notif prodcons actor worker to produce and consume tx objects
    send each tx object into the exchange 
    tx pool service concumes every incoming tx and execute them in the background
    safe tx execution without double spending issue using tokio spawn select mutex and channels
    finally tx pool service publishes the result of executed each tx into the exchange 
    tokio spawn, select, mutex, channels, redis, rmq, actor worker <---> ws, http, tcp, quic, grpc, p2p, wrtc
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
    status: TxStatus
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

        trait ArrExt{
            fn getCode(&self) -> &Self;
        }
        type Arr = [String];
        impl ArrExt for Arr{
            // returning the borrowed form of [String] with the lifetime of &self 
            // since [String] has no fixed size at compile time
            fn getCode(&self) -> &Self{
                todo!()
            }
        }

        let hex: u8 = 0xff; // hex
        let oct = 0o777; // octal
        let bin = 00001110; // bin

        let arr_str: &Arr = &[String::from("")]; // slices need to behind the & due to their unknown size at compile time
        _ = arr_str.getCode();

        let tx = Self{
            amount,
            from: from.to_string(),
            to: to.to_string(),
            tax,
            data: data.to_vec(), 
            status: TxStatus::Started,
            tx_type: TxType::Deposit,
            treasury_type: TreasuryType::Credit,
        };
        // tx.on_error(||{});
        // tx.on_success(||{});
        // tx.on_reject(||{});

        tx
        
    }
    
}

impl Actor for TransactionPoolActor{

    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {

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

trait TransactionPoolActorTx{
    type Tx;
    fn execute(&self) -> Self;
    fn get_status(&self) -> Self;
    fn started(&self);
    fn aborted(&self);
}

impl TransactionPoolActorTx for TransactionPoolActor{
    type Tx = Self;

    fn execute(&self) -> Self {
        
        // update user balance and treasury logics
        // ...

        todo!()
    }

    fn get_status(&self) -> Self {
        todo!()
    }

    fn started(&self){}

    fn aborted(&self){}
}