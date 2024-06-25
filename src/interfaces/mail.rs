




use crate::*;


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
*/
pub trait Mailer{
    type This;
    async fn build(&mut self);
    async fn send(&self);
}