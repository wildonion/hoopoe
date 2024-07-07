

use salvo::websocket::Message;
use tokio::sync::mpsc::UnboundedSender;

use crate::*;
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Server<T: Clone + Debug + Default>{
    pub status: ServerStatus,
    pub data: T
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum ServerStatus{
    #[default]
    Alive,
    Dead, 
    Halted,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Response<'m, T: Clone + Debug + Default>{
    pub data: T,
    pub message: &'m str,
    pub is_err: bool,
    pub status: u16,
    pub meta: Option<serde_json::Value>
}