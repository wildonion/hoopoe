


use crate::lockers::llm::Product;


/* -----------------
    https://boats.gitlab.io/blog/post/async-methods-i/
    GATs can now have generic params which allows us to have async 
    method in traits cause the future returned by an async function 
    captures all lifetimes inputed into the function, in order 
    to express that lifetime relationship, the Future type needs 
    to be generic over a lifetime, so that it can capture the 
    lifetime from the self argument on the method.
    generic and lifetime wasn't supported in GAT till Rust 1.79
    by the result we can have async methods in traits without 
    using third party crates.
    the fact that async method wasn't supported in traits was
    due to the unspported feature of generic and lifetime in GAT
    which wouldn't allow to return a future object from the trait
    method cause future obejcts capture lifetimes forces us to pass
    the GAT with lifetime as the return type of async trait method
    hence using the GAT as the return type of async trait method 
    wasn't supported therefore having future objects in trait method 
    return type was invalid.
*/
pub trait ProductExt{
    type Product;
    async fn atomic_purchase_status(&mut self) -> (bool, tokio::sync::mpsc::Receiver<Self::Product>);
    async fn mint(&mut self) -> (bool, Product); // the actual logic to purchase a product and send it to the mint service
}