

use crate::*;



#[trait_variant::make(AtomicPurchaseSend: Send)] 
pub trait AtomicPurchase{
    type Product;
    async fn atomic_purchase_status(&self) -> (bool, tokio::sync::mpsc::Receiver<Self::Product>);
}