


use crate::*;


// make the trait Sendable cause it has async methods
// macros extend code at compile time so we access to 
// NotifExtSend already in other crates at runtime
#[trait_variant::make(NotifExtSend: Send)] 
pub trait NotifExt<T>{ // polymorphism

    type This; // dynamic typing
    async fn get_notifs(&self) -> Vec<T>;
    fn set_notifs(&mut self) -> Vec<T>;
    fn extend_notifs(&mut self) -> Vec<T>;
}