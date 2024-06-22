




use crate::*;


/* -----------------
    https://boats.gitlab.io/blog/post/async-methods-i/
    GATs can now generic params which allows us to have async 
    method in traits cause the future returned by an async function 
    captures all lifetimes inputed into the function, in order 
    to express that lifetime relationship, the Future type needs 
    to be generic over a lifetime, so that it can capture the 
    lifetime from the self argument on the method.
*/
pub trait TransactionPoolActorTxExt{
    type Tx;
    async fn execute(&self) -> Self;
    async fn get_status(&self) -> Self;
    async fn started(&self);
    async fn aborted(&self);
}