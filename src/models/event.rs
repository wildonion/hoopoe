


use serde::{Serialize, Deserialize};
use crate::*;


#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ProduceNotifInfo{
    pub info: ProduceNotif,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ConsumeNotifInfo{
    pub info: ConsumeNotif,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct RegisterNotif{
    pub producer_info: Option<ProduceNotifInfo>,
    pub consumer_info: Option<ConsumeNotifInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HoopEvent{
    pub etype: String, 
    pub manager: i32,
    pub entrance_fee: i64,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ReceiverInfo{
    pub id: i32, // a unique identity
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ActionerInfo{
    pub id: i32, // a unique identity
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub enum ActionType{ // all the action type that causes the notif to get fired
    #[default]
    ProductPurchased, // or minted
    Zerlog,
    EventCreated,
    EventExpired,
    EventLocked,
    // probably other system notifs
    // ...
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct NotifData{
    pub id: String,
    pub receiver_info: Option<serde_json::Value>,
    pub action_data: Option<serde_json::Value>,
    pub actioner_info: Option<serde_json::Value>,
    pub action_type: ActionType,
    pub fired_at: Option<i64>, 
    pub is_seen: bool,
}