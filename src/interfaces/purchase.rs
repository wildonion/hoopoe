

use actix::Addr;
use crate::*;
use self::workers::producers::notif::NotifProducerActor;



#[trait_variant::make(ProductExtSend: Send)] 
pub trait ProductExt{
    type Product;
    /* 
        following returns a boolean and an mpsc receiver to check the 
        locking status of the product and receive the result of minting 
        process of the product.
    */
    async fn atomic_purchase_status(&self, notif_producer_actor: Addr<NotifProducerActor>) -> (bool, tokio::sync::mpsc::Receiver<Self::Product>);
    /* 
        following returns a boolean and the product instance itself to check 
        the minting status of the product, if there was any error.
    */
    async fn mint(&mut self, notif_producer_actor: Addr<NotifProducerActor>) -> (bool, Self::Product);
}