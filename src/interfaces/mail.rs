




use crate::*;


pub trait Mailer{
    type This;
    async fn build(&mut self);
    async fn send(&self);
}