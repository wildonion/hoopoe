

use crate::*;



#[trait_variant::make(ProductExtSend: Send)] 
pub trait ProductExt{
    type Product;
    async fn atomic_purchase_status(&self) -> (bool, tokio::sync::mpsc::Receiver<Self::Product>);
    async fn mint(&mut self) -> (bool, Self::Product);
}