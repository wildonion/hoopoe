

use actix::Addr;
use crate::*;
use self::actors::producers::notif::NotifProducerActor;



#[trait_variant::make(ProductExtSend: Send)] 
pub trait ProductExt{
    type Product;
    async fn atomic_purchase_status(&self, notif_producer_actor: Addr<NotifProducerActor>) -> (bool, tokio::sync::mpsc::Receiver<Self::Product>);
    async fn mint(&mut self, notif_producer_actor: Addr<NotifProducerActor>) -> (bool, Self::Product);
}